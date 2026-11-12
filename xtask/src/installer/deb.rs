// ./xtask/src/installer/deb.rs
//! Debian archives (`.deb`) written without `dpkg-deb`, so any CI runner can build them.
//!
//! A `.deb` is an `ar` archive holding `debian-binary`, `control.tar.gz`, and `data.tar.gz`, in
//! that order. Headers are normalised (root ownership, `SOURCE_DATE_EPOCH` timestamps) so the same
//! commit rebuilds to the same bytes.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use flate2::Compression;
use flate2::write::GzEncoder;
use md5::{Digest, Md5};

use super::payload::Payload;
use super::{Manifest, package_version, source_date_epoch};

const MAINTAINER: &str = "pyenv-native maintainers <noreply@users.noreply.github.com>";
const HOMEPAGE: &str = "https://github.com/imyourboyroy/pyenv-native";
const DESCRIPTION: &str = "Native, cross-platform Python version manager\n pyenv-native installs and switches between Python runtimes, manages virtual\n environments, and ships an MCP server for coding agents. It is a single Rust\n binary that keeps compatibility with the classic pyenv workflow.\n .\n Python versions and shims live in ~/.pyenv; this package updates through the\n system package manager.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flavor {
    Debian,
    Termux,
}

fn architecture(flavor: Flavor, arch: &str) -> Result<&'static str> {
    Ok(match (flavor, arch) {
        (Flavor::Debian, "x64") => "amd64",
        (Flavor::Debian, "arm64") => "arm64",
        (Flavor::Debian, "x86") => "i386",
        (Flavor::Termux, "x64") => "x86_64",
        (Flavor::Termux, "arm64") => "aarch64",
        (Flavor::Termux, "x86") => "i686",
        (_, other) => bail!("no Debian architecture for `{other}`"),
    })
}

pub(crate) struct Control<'a> {
    pub version: &'a str,
    pub architecture: &'a str,
    pub installed_size_kib: u64,
    pub depends: Vec<String>,
    pub recommends: Vec<String>,
}

impl Control<'_> {
    pub(crate) fn render(&self) -> String {
        let mut text = format!(
            "Package: pyenv-native\nVersion: {}\nArchitecture: {}\nMaintainer: {MAINTAINER}\nInstalled-Size: {}\n",
            self.version, self.architecture, self.installed_size_kib
        );
        if !self.depends.is_empty() {
            text.push_str(&format!("Depends: {}\n", self.depends.join(", ")));
        }
        if !self.recommends.is_empty() {
            text.push_str(&format!("Recommends: {}\n", self.recommends.join(", ")));
        }
        text.push_str(&format!(
            "Conflicts: pyenv\nSection: devel\nPriority: optional\nHomepage: {HOMEPAGE}\nDescription: {DESCRIPTION}\n"
        ));
        text
    }
}

fn tar_header(kind: tar::EntryType, mode: u32, size: usize, mtime: u64) -> tar::Header {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(kind);
    header.set_mode(mode);
    header.set_size(size as u64);
    header.set_mtime(mtime);
    header.set_uid(0);
    header.set_gid(0);
    let _ = header.set_username("root");
    let _ = header.set_groupname("root");
    header
}

/// Append an entry named exactly `name` (the `tar` crate would strip the leading `./` that
/// Debian tooling expects). Names must fit the 100-byte ustar field.
fn append_named(
    builder: &mut tar::Builder<GzEncoder<Vec<u8>>>,
    mut header: tar::Header,
    name: &str,
    data: &[u8],
) -> Result<()> {
    let bytes = name.as_bytes();
    if bytes.len() > 99 {
        bail!("`{name}` is too long for a ustar header");
    }
    header.as_old_mut().name[..bytes.len()].copy_from_slice(bytes);
    header.set_cksum();
    builder.append(&header, data)?;
    Ok(())
}

fn gzip_tar(
    build: impl FnOnce(&mut tar::Builder<GzEncoder<Vec<u8>>>) -> Result<()>,
) -> Result<Vec<u8>> {
    let mut builder = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::best()));
    build(&mut builder)?;
    Ok(builder.into_inner()?.finish()?)
}

fn control_tar(control: &str, md5sums: &str, conffiles: &[String], mtime: u64) -> Result<Vec<u8>> {
    gzip_tar(|builder| {
        let mut entries: Vec<(&str, String)> = vec![
            ("./control", control.to_string()),
            ("./md5sums", md5sums.to_string()),
        ];
        if !conffiles.is_empty() {
            entries.push(("./conffiles", format!("{}\n", conffiles.join("\n"))));
        }
        for (name, body) in entries {
            let header = tar_header(tar::EntryType::Regular, 0o644, body.len(), mtime);
            append_named(builder, header, name, body.as_bytes())?;
        }
        Ok(())
    })
}

fn data_tar(payload: &Payload, mtime: u64) -> Result<Vec<u8>> {
    gzip_tar(|builder| {
        let root = tar_header(tar::EntryType::Directory, 0o755, 0, mtime);
        append_named(builder, root, "./", &[])?;
        for dir in payload.directories() {
            let header = tar_header(tar::EntryType::Directory, 0o755, 0, mtime);
            append_named(builder, header, &format!(".{dir}/"), &[])?;
        }
        for file in &payload.files {
            let header = tar_header(tar::EntryType::Regular, file.mode, file.data.len(), mtime);
            append_named(builder, header, &format!(".{}", file.path), &file.data)?;
        }
        Ok(())
    })
}

fn md5sums(payload: &Payload) -> String {
    payload
        .files
        .iter()
        .map(|file| {
            let digest = Md5::digest(&file.data);
            format!(
                "{}  {}\n",
                hex::encode(digest),
                file.path.trim_start_matches('/')
            )
        })
        .collect()
}

/// One `ar` member: 60-byte header, data, padded to an even length.
fn ar_member(out: &mut Vec<u8>, name: &str, data: &[u8], mtime: u64) {
    let header = format!(
        "{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n",
        name,
        mtime,
        0,
        0,
        "100644",
        data.len()
    );
    debug_assert_eq!(header.len(), 60);
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(data);
    if data.len() % 2 == 1 {
        out.push(b'\n');
    }
}

pub(crate) fn build(
    payload: &Payload,
    manifest: &Manifest,
    flavor: Flavor,
    mtime: u64,
) -> Result<(String, Vec<u8>)> {
    let arch = architecture(flavor, &manifest.architecture)?;
    let version = package_version(&manifest.version);

    let mut depends = Vec::new();
    if let (Flavor::Debian, Some(minor)) = (flavor, payload.glibc_floor()) {
        depends.push(format!("libc6 (>= 2.{minor})"));
        if payload.needs_libgcc_s() {
            depends.push("libgcc-s1".to_string());
        }
    }
    let has_gui = payload
        .files
        .iter()
        .any(|file| file.path.ends_with("/bin/pyenv-gui"));
    let recommends = if has_gui {
        vec![
            "libgtk-3-0 | libgtk-3-0t64".to_string(),
            "libwebkit2gtk-4.1-0".to_string(),
        ]
    } else {
        Vec::new()
    };
    let control = Control {
        version: &version,
        architecture: arch,
        installed_size_kib: payload.installed_size_kib(),
        depends,
        recommends,
    }
    .render();
    let conffiles: Vec<String> = payload
        .files
        .iter()
        .filter(|file| file.config)
        .map(|file| file.path.clone())
        .collect();

    let mut deb = b"!<arch>\n".to_vec();
    ar_member(&mut deb, "debian-binary", b"2.0\n", mtime);
    ar_member(
        &mut deb,
        "control.tar.gz",
        &control_tar(&control, &md5sums(payload), &conffiles, mtime)?,
        mtime,
    );
    ar_member(&mut deb, "data.tar.gz", &data_tar(payload, mtime)?, mtime);
    Ok((format!("pyenv-native_{version}_{arch}.deb"), deb))
}

pub(crate) fn write(
    payload: &Payload,
    manifest: &Manifest,
    flavor: Flavor,
    output_root: &Path,
) -> Result<PathBuf> {
    let (name, bytes) = build(payload, manifest, flavor, source_date_epoch())?;
    fs::create_dir_all(output_root)?;
    let path = output_root.join(name);
    let mut file =
        fs::File::create(&path).with_context(|| format!("cannot write {}", path.display()))?;
    file.write_all(&bytes)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::installer::payload::PayloadFile;
    use std::process::Command;

    fn manifest() -> Manifest {
        Manifest {
            version: "1.2.3-rc.1".into(),
            platform: "linux".into(),
            architecture: "x64".into(),
            gui: false,
        }
    }

    fn payload() -> Payload {
        let mut binary = b"\x7fELF....GLIBC_2.17\0..GLIBC_2.34\0..GLIBC_2.9\0".to_vec();
        binary.resize(4096, 0);
        Payload {
            files: vec![
                PayloadFile {
                    path: "/etc/profile.d/pyenv-native.sh".into(),
                    mode: 0o644,
                    data: b"# x\n".to_vec(),
                    config: true,
                },
                PayloadFile {
                    path: "/usr/bin/pyenv".into(),
                    mode: 0o755,
                    data: binary,
                    config: false,
                },
                PayloadFile {
                    path: "/usr/share/pyenv-native/install-source".into(),
                    mode: 0o644,
                    data: b"deb\n".to_vec(),
                    config: false,
                },
            ],
        }
    }

    #[test]
    fn architectures_map_per_flavor() {
        assert_eq!(architecture(Flavor::Debian, "x64").unwrap(), "amd64");
        assert_eq!(architecture(Flavor::Termux, "arm64").unwrap(), "aarch64");
        assert!(architecture(Flavor::Debian, "riscv").is_err());
    }

    #[test]
    fn archive_layout_is_ar_with_the_three_members_in_order() {
        let (name, bytes) = build(&payload(), &manifest(), Flavor::Debian, 1_700_000_000).unwrap();
        assert_eq!(name, "pyenv-native_1.2.3~rc.1_amd64.deb");
        assert!(bytes.starts_with(b"!<arch>\ndebian-binary   "));
        let text = String::from_utf8_lossy(&bytes);
        let positions: Vec<usize> = ["debian-binary", "control.tar.gz", "data.tar.gz"]
            .iter()
            .map(|member| text.find(member).unwrap())
            .collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn building_twice_yields_identical_bytes() {
        let first = build(&payload(), &manifest(), Flavor::Debian, 1_700_000_000).unwrap();
        let second = build(&payload(), &manifest(), Flavor::Debian, 1_700_000_000).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn control_declares_conflicts_and_the_glibc_floor() {
        let control = Control {
            version: "1.0.0",
            architecture: "amd64",
            installed_size_kib: 10,
            depends: vec!["libc6 (>= 2.34)".into()],
            recommends: vec![],
        }
        .render();
        assert!(control.contains("Depends: libc6 (>= 2.34)\n"));
        assert!(control.contains("Conflicts: pyenv\n"));
        assert!(
            control.contains("\nDescription: Native, cross-platform Python version manager\n ")
        );
    }

    #[test]
    fn dpkg_accepts_the_package_when_available() {
        if Command::new("dpkg-deb").arg("--version").output().is_err() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let (name, bytes) = build(&payload(), &manifest(), Flavor::Debian, 1_700_000_000).unwrap();
        let path = temp.path().join(name);
        fs::write(&path, bytes).unwrap();

        let info = Command::new("dpkg-deb")
            .arg("--info")
            .arg(&path)
            .output()
            .unwrap();
        let info_text = String::from_utf8_lossy(&info.stdout);
        assert!(
            info.status.success(),
            "{}",
            String::from_utf8_lossy(&info.stderr)
        );
        assert!(info_text.contains("Package: pyenv-native"), "{info_text}");
        assert!(
            info_text.contains("Depends: libc6 (>= 2.34)"),
            "{info_text}"
        );

        let contents = Command::new("dpkg-deb")
            .arg("--contents")
            .arg(&path)
            .output()
            .unwrap();
        let listing = String::from_utf8_lossy(&contents.stdout);
        assert!(listing.contains("./usr/bin/pyenv"), "{listing}");
        assert!(
            listing.contains("./etc/profile.d/pyenv-native.sh"),
            "{listing}"
        );

        let extract = temp.path().join("root");
        let status = Command::new("dpkg-deb")
            .arg("-x")
            .arg(&path)
            .arg(&extract)
            .status()
            .unwrap();
        assert!(status.success());
        assert_eq!(
            fs::read_to_string(extract.join("usr/share/pyenv-native/install-source")).unwrap(),
            "deb\n"
        );

        let control = temp.path().join("control");
        assert!(
            Command::new("dpkg-deb")
                .arg("-e")
                .arg(&path)
                .arg(&control)
                .status()
                .unwrap()
                .success()
        );
        assert!(
            fs::read_to_string(control.join("conffiles"))
                .unwrap()
                .contains("/etc/profile.d/pyenv-native.sh")
        );
        assert!(
            fs::read_to_string(control.join("md5sums"))
                .unwrap()
                .contains("usr/bin/pyenv")
        );
    }
}
