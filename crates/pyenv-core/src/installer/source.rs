// ./crates/pyenv-core/src/installer/source.rs
//! Detect installs owned by a system package manager.
//!
//! Native packages (`.deb`, `.rpm`, the Termux `.deb`) place `pyenv` in the system prefix and ship
//! `<prefix>/share/pyenv-native/install-source` containing the package format. Those installs must
//! be updated by the package manager, so `pyenv self-update` declines and says how instead.
//! Portable installs under `PYENV_ROOT` have no marker and keep updating themselves exactly as
//! earlier releases did.

use std::fs;
use std::path::Path;

/// Marker location relative to the prefix that holds `bin/pyenv`.
pub const INSTALL_SOURCE_MARKER: &str = "share/pyenv-native/install-source";

/// The package format that owns the executable at `exe`, when there is one.
pub fn package_source_for(exe: &Path) -> Option<String> {
    let resolved = fs::canonicalize(exe).unwrap_or_else(|_| exe.to_path_buf());
    let prefix = resolved.parent()?.parent()?;
    let text = fs::read_to_string(prefix.join(INSTALL_SOURCE_MARKER)).ok()?;
    let token = text.lines().next()?.trim().to_ascii_lowercase();
    let valid = !token.is_empty()
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'));
    valid.then_some(token)
}

/// The command a user runs to upgrade a package-managed install.
pub fn upgrade_command(source: &str) -> &'static str {
    match source {
        "deb" => "sudo apt update && sudo apt install --only-upgrade pyenv-native",
        "rpm" => "sudo dnf upgrade pyenv-native",
        "termux" => "pkg upgrade pyenv-native",
        _ => "your package manager's upgrade command",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn prefix_with_marker(contents: Option<&str>) -> (TempDir, std::path::PathBuf) {
        let temp = TempDir::new().unwrap();
        let bin = temp.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        let exe = bin.join("pyenv");
        fs::write(&exe, "x").unwrap();
        if let Some(contents) = contents {
            let marker = temp.path().join(INSTALL_SOURCE_MARKER);
            fs::create_dir_all(marker.parent().unwrap()).unwrap();
            fs::write(marker, contents).unwrap();
        }
        (temp, exe)
    }

    #[test]
    fn portable_installs_have_no_source() {
        let (_temp, exe) = prefix_with_marker(None);
        assert_eq!(package_source_for(&exe), None);
    }

    #[test]
    fn marker_names_the_package_format() {
        let (_temp, exe) = prefix_with_marker(Some("DEB\n"));
        assert_eq!(package_source_for(&exe).as_deref(), Some("deb"));
    }

    #[test]
    fn garbage_markers_are_ignored() {
        let (_temp, exe) = prefix_with_marker(Some("rm -rf /\n"));
        assert_eq!(package_source_for(&exe), None);
        let (_temp, exe) = prefix_with_marker(Some(""));
        assert_eq!(package_source_for(&exe), None);
    }

    #[test]
    fn every_known_format_has_an_upgrade_command() {
        for source in ["deb", "rpm", "termux"] {
            assert!(!upgrade_command(source).starts_with("your "));
        }
    }
}
