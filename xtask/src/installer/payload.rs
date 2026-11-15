// ./xtask/src/installer/payload.rs
//! The file list shared by every system package format.

use std::collections::BTreeSet;

use anyhow::{Context, Result};

use super::Bundle;
use crate::brand::{self, APP_ID};

const ICON_SIZES: [&str; 4] = ["32x32", "128x128", "256x256", "512x512"];

/// Where a package format puts things.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Layout {
    /// Install prefix (`/usr`, or the Termux prefix).
    pub prefix: String,
    /// Directory for `profile.d` (`/etc/profile.d`, or `<prefix>/etc/profile.d` on Termux); none on
    /// macOS, where login shells have no such mechanism.
    pub profile_d: Option<String>,
    /// Written to `share/enchi/install-source`; read by `enchi self-update`.
    pub source: &'static str,
    /// Where the license text goes (`/usr/share/doc/...` for Debian, `/usr/share/licenses/...` for RPM).
    pub license_path: String,
    /// Whether the GUI launcher and icons make sense (Termux has no desktop).
    pub desktop_integration: bool,
}

impl Layout {
    pub(crate) fn debian() -> Self {
        Self {
            prefix: "/usr".into(),
            profile_d: Some("/etc/profile.d".into()),
            source: "deb",
            license_path: "/usr/share/doc/enchi/copyright".into(),
            desktop_integration: true,
        }
    }

    pub(crate) fn rpm() -> Self {
        Self {
            prefix: "/usr".into(),
            profile_d: Some("/etc/profile.d".into()),
            source: "rpm",
            license_path: "/usr/share/licenses/enchi/LICENSE".into(),
            desktop_integration: true,
        }
    }

    /// The macOS `.pkg`: `/usr/local` (already on every user's `PATH`) plus an app in `/Applications`.
    pub(crate) fn macos() -> Self {
        Self {
            prefix: "/usr/local".into(),
            profile_d: None,
            source: "pkg",
            license_path: "/usr/local/share/doc/enchi/LICENSE".into(),
            desktop_integration: false,
        }
    }

    pub(crate) fn termux() -> Self {
        let prefix = "/data/data/com.termux/files/usr";
        Self {
            prefix: prefix.into(),
            profile_d: Some(format!("{prefix}/etc/profile.d")),
            source: "termux",
            license_path: format!("{prefix}/share/doc/enchi/copyright"),
            desktop_integration: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PayloadFile {
    /// Absolute destination path inside the package.
    pub path: String,
    pub mode: u32,
    pub data: Vec<u8>,
    /// Administrator-editable configuration that package upgrades must not overwrite.
    pub config: bool,
}

/// A symbolic link the package installs (`path` -> `target`, relative to the link's directory).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PayloadLink {
    /// Absolute path of the link inside the package.
    pub path: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Payload {
    pub files: Vec<PayloadFile>,
    pub links: Vec<PayloadLink>,
}

impl Payload {
    /// Whether an executable links the dynamic `libgcc_s.so.1` (Rust unwinding on glibc).
    pub(crate) fn needs_libgcc_s(&self) -> bool {
        self.files
            .iter()
            .filter(|file| file.path.contains("/bin/"))
            .any(|file| {
                file.data
                    .windows(b"libgcc_s.so.1".len())
                    .any(|window| window == b"libgcc_s.so.1")
            })
    }

    /// Oldest glibc minor version (`2.N`) the packaged executables can run on, taken from the
    /// highest versioned symbol any of them references. `None` for static (musl) builds.
    pub(crate) fn glibc_floor(&self) -> Option<u32> {
        self.files
            .iter()
            .filter(|file| file.path.contains("/bin/"))
            .filter_map(|file| max_glibc_minor(&file.data))
            .max()
    }

    /// Every directory that must exist for the files, shallowest first (deduplicated).
    pub(crate) fn directories(&self) -> Vec<String> {
        let mut dirs = BTreeSet::new();
        let paths = self
            .files
            .iter()
            .map(|file| &file.path)
            .chain(self.links.iter().map(|link| &link.path));
        for path in paths {
            let mut current = String::new();
            let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
            for part in &parts[..parts.len().saturating_sub(1)] {
                current.push('/');
                current.push_str(part);
                dirs.insert(current.clone());
            }
        }
        let mut ordered: Vec<String> = dirs.into_iter().collect();
        ordered.sort_by_key(|dir| dir.matches('/').count());
        ordered
    }

    pub(crate) fn installed_size_kib(&self) -> u64 {
        self.files
            .iter()
            .map(|file| file.data.len() as u64)
            .sum::<u64>()
            .div_ceil(1024)
    }
}

/// Highest `GLIBC_2.N` symbol version referenced by an ELF binary, if it links glibc at all.
pub(crate) fn max_glibc_minor(binary: &[u8]) -> Option<u32> {
    const NEEDLE: &[u8] = b"GLIBC_2.";
    let mut best = None;
    let mut index = 0;
    while let Some(found) = binary[index..]
        .windows(NEEDLE.len())
        .position(|window| window == NEEDLE)
    {
        let start = index + found + NEEDLE.len();
        let digits: Vec<u8> = binary[start..]
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .copied()
            .collect();
        if let Some(minor) = std::str::from_utf8(&digits)
            .ok()
            .and_then(|text| text.parse::<u32>().ok())
        {
            best = best.max(Some(minor));
        }
        index = start;
    }
    best
}

/// The install root for the login-shell PATH entry: an explicit `ENCHI_ROOT`, else the classic
/// `PYENV_ROOT`, else whichever of `~/.enchi` (fresh installs) and `~/.pyenv` (pyenv-native installs
/// that were never moved) exists, else `~/.enchi`.
const PROFILE_D: &str = "\
# Enchi: put the shims for Python versions managed in ~/.enchi (or ~/.pyenv) on PATH.
# Installed by the enchi package; delete this file to opt out.
_enchi_root=\"${ENCHI_ROOT:-${PYENV_ROOT:-}}\"
if [ -z \"$_enchi_root\" ]; then
  if [ -d \"$HOME/.enchi\" ] || [ ! -d \"$HOME/.pyenv\" ]; then
    _enchi_root=\"$HOME/.enchi\"
  else
    _enchi_root=\"$HOME/.pyenv\"
  fi
fi
case \":$PATH:\" in
  *\":$_enchi_root/shims:\"*) ;;
  *) PATH=\"$_enchi_root/shims:$PATH\"; export PATH ;;
esac
unset _enchi_root
";

fn desktop_entry() -> String {
    format!(
        "[Desktop Entry]\nType=Application\nVersion=1.0\nName={name}\nGenericName=Python Environment Manager\nComment=Manage Python versions and virtual environments\nExec={gui}\nIcon={APP_ID}\nStartupWMClass={gui}\nCategories=Development;Utility;\nTerminal=false\n",
        name = brand::NAME,
        gui = brand::GUI,
    )
}

/// Debian-style machine-readable copyright wrapping the project license.
fn debian_copyright(license: &str) -> String {
    let indented: String = license
        .lines()
        .map(|line| {
            if line.trim().is_empty() {
                " .\n".to_string()
            } else {
                format!(" {line}\n")
            }
        })
        .collect();
    format!(
        "Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/\nUpstream-Name: {name}\nSource: {url}\n\nFiles: *\nCopyright: {name} contributors\nLicense: MIT\n{indented}",
        name = brand::NAME,
        url = brand::repo_url(),
    )
}

pub(crate) fn system_payload(bundle: &Bundle, layout: &Layout) -> Result<Payload> {
    let mut files = Vec::new();
    let mut push = |path: String, mode: u32, data: Vec<u8>, config: bool| {
        files.push(PayloadFile {
            path,
            mode,
            data,
            config,
        });
    };

    let prefix = &layout.prefix;
    for name in [brand::CLI, brand::MCP] {
        let data = bundle
            .read(name)
            .with_context(|| format!("the bundle has no `{name}` executable"))?;
        push(format!("{prefix}/bin/{name}"), 0o755, data, false);
    }
    let gui = bundle.manifest.gui && layout.desktop_integration;
    if gui {
        let data = bundle.read(brand::GUI).with_context(|| {
            format!("the manifest lists {} but the bundle lacks it", brand::GUI)
        })?;
        push(format!("{prefix}/bin/{}", brand::GUI), 0o755, data, false);
        push(
            format!("{prefix}/share/applications/{APP_ID}.desktop"),
            0o644,
            desktop_entry().into_bytes(),
            false,
        );
        for size in ICON_SIZES {
            if let Some(icon) =
                bundle.read(&format!("share/icons/hicolor/{size}/apps/{APP_ID}.png"))
            {
                push(
                    format!("{prefix}/share/icons/hicolor/{size}/apps/{APP_ID}.png"),
                    0o644,
                    icon,
                    false,
                );
            }
        }
    }

    push(
        format!("{prefix}/share/{}/install-source", brand::PACKAGE),
        0o644,
        format!("{}\n", layout.source).into_bytes(),
        false,
    );
    if let Some(profile_d) = &layout.profile_d {
        push(
            format!("{profile_d}/{}.sh", brand::PACKAGE),
            0o644,
            PROFILE_D.as_bytes().to_vec(),
            true,
        );
    }

    for (source, name) in [
        ("README.md", "README.md"),
        ("INSTRUCTIONS.md", "INSTRUCTIONS.md"),
        ("MCP.md", "MCP.md"),
    ] {
        if let Some(data) = bundle.read(source) {
            push(
                format!("{prefix}/share/doc/{}/{name}", brand::PACKAGE),
                0o644,
                data,
                false,
            );
        }
    }
    if let Some(license) = bundle.read("LICENSE") {
        let text = String::from_utf8_lossy(&license).into_owned();
        let data = if matches!(layout.source, "deb" | "termux") {
            debian_copyright(&text).into_bytes()
        } else {
            license
        };
        push(layout.license_path.clone(), 0o644, data, false);
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    // Compatibility links so scripts and MCP configs written for pyenv-native keep resolving; the
    // real executables are the `enchi*` ones above.
    let links = [
        (brand::LEGACY_CLI, brand::CLI),
        (brand::LEGACY_MCP, brand::MCP),
    ]
    .into_iter()
    .map(|(link, target)| PayloadLink {
        path: format!("{prefix}/bin/{link}"),
        target: target.to_string(),
    })
    .collect();
    Ok(Payload { files, links })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str) -> PayloadFile {
        PayloadFile {
            path: path.into(),
            mode: 0o644,
            data: vec![0; 2048],
            config: false,
        }
    }

    #[test]
    fn glibc_requirement_is_the_highest_symbol_version() {
        assert_eq!(
            max_glibc_minor(b"GLIBC_2.17 GLIBC_2.34 GLIBC_2.9"),
            Some(34)
        );
        assert_eq!(max_glibc_minor(b"statically linked musl"), None);
    }

    #[test]
    fn payload_floor_spans_every_executable_in_bin() {
        let exe = |path: &str, text: &[u8]| PayloadFile {
            path: path.into(),
            mode: 0o755,
            data: text.to_vec(),
            config: false,
        };
        let payload = Payload {
            files: vec![
                exe("/usr/bin/enchi", b"GLIBC_2.31"),
                exe("/usr/bin/enchi-gui", b"GLIBC_2.35"),
                exe("/usr/share/doc/x", b"GLIBC_2.99"),
            ],
            links: vec![],
        };
        assert_eq!(payload.glibc_floor(), Some(35));
        assert_eq!(Payload::default().glibc_floor(), None);
    }

    #[test]
    fn directories_are_listed_parents_first_without_duplicates() {
        let payload = Payload {
            files: vec![file("/usr/bin/enchi"), file("/usr/share/doc/x/README")],
            links: vec![PayloadLink {
                path: "/usr/sbin/pyenv".into(),
                target: "enchi".into(),
            }],
        };
        assert_eq!(
            payload.directories(),
            [
                "/usr",
                "/usr/bin",
                "/usr/sbin",
                "/usr/share",
                "/usr/share/doc",
                "/usr/share/doc/x"
            ]
        );
    }

    #[test]
    fn installed_size_rounds_up_to_kib() {
        let payload = Payload {
            files: vec![file("/a"), file("/b")],
            links: vec![],
        };
        assert_eq!(payload.installed_size_kib(), 4);
    }

    #[test]
    fn layouts_keep_each_format_in_its_own_prefix() {
        assert_eq!(Layout::debian().prefix, "/usr");
        assert_eq!(Layout::rpm().source, "rpm");
        let termux = Layout::termux();
        assert!(termux.prefix.starts_with("/data/data/com.termux/"));
        assert!(
            termux
                .profile_d
                .as_deref()
                .is_some_and(|dir| dir.starts_with(&termux.prefix))
        );
        assert!(!termux.desktop_integration);
    }

    #[test]
    fn profile_snippet_is_posix_and_guards_against_duplicates() {
        assert!(PROFILE_D.contains("${ENCHI_ROOT:-${PYENV_ROOT:-}}"));
        assert!(PROFILE_D.contains("$HOME/.enchi"));
        assert!(PROFILE_D.contains("$HOME/.pyenv"));
        assert!(PROFILE_D.contains("case \":$PATH:\" in"));
        assert!(!PROFILE_D.contains("[["));
    }

    fn fake_bundle() -> Bundle {
        let dir = tempfile::tempdir().unwrap();
        for name in ["enchi", "enchi-mcp", "README.md", "LICENSE"] {
            std::fs::write(dir.path().join(name), format!("{name} contents")).unwrap();
        }
        Bundle {
            dir,
            manifest: super::super::Manifest {
                version: "1.0.0".into(),
                platform: "linux".into(),
                architecture: "x64".into(),
                gui: false,
            },
        }
    }

    #[test]
    fn system_payload_installs_enchi_and_legacy_compat_links() {
        let payload = system_payload(&fake_bundle(), &Layout::debian()).unwrap();
        let paths: Vec<&str> = payload.files.iter().map(|f| f.path.as_str()).collect();
        for expected in [
            "/usr/bin/enchi",
            "/usr/bin/enchi-mcp",
            "/usr/share/enchi/install-source",
            "/etc/profile.d/enchi.sh",
            "/usr/share/doc/enchi/README.md",
            "/usr/share/doc/enchi/copyright",
        ] {
            assert!(
                paths.contains(&expected),
                "{expected} missing from {paths:?}"
            );
        }
        assert!(
            paths.iter().all(|path| !path.contains("pyenv")),
            "no regular file keeps the old name: {paths:?}"
        );
        let links: Vec<(&str, &str)> = payload
            .links
            .iter()
            .map(|l| (l.path.as_str(), l.target.as_str()))
            .collect();
        assert_eq!(
            links,
            [
                ("/usr/bin/pyenv", "enchi"),
                ("/usr/bin/pyenv-mcp", "enchi-mcp")
            ]
        );
        let marker = payload
            .files
            .iter()
            .find(|f| f.path == "/usr/share/enchi/install-source")
            .unwrap();
        assert_eq!(marker.data, b"deb\n");
    }

    #[test]
    fn desktop_entry_uses_the_enchi_app_id_and_gui() {
        let entry = desktop_entry();
        assert!(entry.contains("Name=Enchi\n"));
        assert!(entry.contains("Exec=enchi-gui\n"));
        assert!(entry.contains("Icon=io.github.imyourboyroy.enchi\n"));
        assert!(entry.contains("StartupWMClass=enchi-gui\n"));
    }

    #[test]
    fn copyright_wraps_blank_lines_as_dot_lines() {
        let text = debian_copyright("MIT License\n\nPermission is hereby granted");
        assert!(text.contains("License: MIT\n MIT License\n .\n Permission"));
        assert!(text.contains("Upstream-Name: Enchi\n"));
    }
}
