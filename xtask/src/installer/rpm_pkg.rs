// ./xtask/src/installer/rpm_pkg.rs
//! RPM packages through the pure-Rust `rpm` crate (no `rpmbuild`, works on any CI runner).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rpm::{BuildConfig, CompressionType, Dependency, FileOptions, PackageBuilder};

use super::payload::Payload;
use super::{Manifest, package_version, source_date_epoch};

const URL: &str = "https://github.com/imyourboyroy/pyenv-native";

fn architecture(arch: &str) -> Result<&'static str> {
    Ok(match arch {
        "x64" => "x86_64",
        "arm64" => "aarch64",
        "x86" => "i686",
        other => bail!("no RPM architecture for `{other}`"),
    })
}

pub(crate) fn write(payload: &Payload, manifest: &Manifest, output_root: &Path) -> Result<PathBuf> {
    let arch = architecture(&manifest.architecture)?;
    let version = package_version(&manifest.version);

    let config = BuildConfig::default()
        .compression(CompressionType::Gzip)
        .source_date(source_date_epoch() as u32);
    let mut builder = PackageBuilder::new(
        "pyenv-native",
        &version,
        "MIT",
        arch,
        "Native, cross-platform Python version manager",
    );
    builder
        .using_config(config)
        .release("1")
        .url(URL)
        .vendor("pyenv-native")
        .description(
            "pyenv-native installs and switches between Python runtimes, manages virtual \
             environments, and ships an MCP server for coding agents. Python versions and \
             shims live in ~/.pyenv; this package updates through the system package manager.",
        )
        .conflicts(Dependency::any("pyenv"));
    // rpm's own dependency generator would add this from the ELF; without it an old distribution
    // would install the package and then fail at run time.
    if let Some(minor) = payload.glibc_floor() {
        let suffix = if arch == "x86_64" || arch == "aarch64" {
            "(64bit)"
        } else {
            ""
        };
        builder.requires(Dependency::any(format!(
            "libc.so.6(GLIBC_2.{minor}){suffix}"
        )));
        if payload.needs_libgcc_s() {
            builder.requires(Dependency::any(format!("libgcc_s.so.1(){suffix}")));
        }
    }

    for file in &payload.files {
        let mut options = FileOptions::new(&file.path).permissions(file.mode as u16);
        if file.config {
            options = options.config();
        }
        builder
            .with_file_contents(file.data.clone(), options)
            .with_context(|| format!("cannot add {}", file.path))?;
    }
    let package = builder.build().context("rpm build failed")?;

    fs::create_dir_all(output_root)?;
    let path = output_root.join(format!("pyenv-native-{version}-1.{arch}.rpm"));
    package
        .write_file(&path)
        .with_context(|| format!("cannot write {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::installer::payload::PayloadFile;

    #[test]
    fn architectures_map() {
        assert_eq!(architecture("x64").unwrap(), "x86_64");
        assert_eq!(architecture("arm64").unwrap(), "aarch64");
        assert!(architecture("sparc").is_err());
    }

    #[test]
    fn written_package_round_trips_through_the_reader() {
        let temp = tempfile::tempdir().unwrap();
        let payload = Payload {
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
                    data: b"bin".to_vec(),
                    config: false,
                },
                PayloadFile {
                    path: "/usr/share/pyenv-native/install-source".into(),
                    mode: 0o644,
                    data: b"rpm\n".to_vec(),
                    config: false,
                },
            ],
        };
        let manifest = Manifest {
            version: "1.2.3".into(),
            platform: "linux".into(),
            architecture: "x64".into(),
            gui: false,
        };
        let path = write(&payload, &manifest, temp.path()).unwrap();
        assert_eq!(path.file_name().unwrap(), "pyenv-native-1.2.3-1.x86_64.rpm");

        let package = rpm::Package::open(&path).unwrap();
        assert_eq!(package.metadata.get_name().unwrap(), "pyenv-native");
        assert_eq!(package.metadata.get_version().unwrap(), "1.2.3");
        assert_eq!(package.metadata.get_arch().unwrap(), "x86_64");
        let paths: Vec<String> = package
            .metadata
            .get_file_paths()
            .unwrap()
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        assert!(paths.contains(&"/usr/bin/pyenv".to_string()), "{paths:?}");
        assert!(paths.contains(&"/etc/profile.d/pyenv-native.sh".to_string()));
    }
}
