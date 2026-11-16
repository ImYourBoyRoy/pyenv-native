// ./crates/enchi-core/src/installer/source.rs
//! Detect installs owned by a system package manager.
//!
//! Native packages (`.deb`, `.rpm`, the Termux `.deb`, the macOS `.pkg`) place `enchi` in the system prefix and ship
//! `<prefix>/share/pyenv-native/install-source` containing the package format. Those installs must
//! be updated by the package manager, so `enchi self-update` declines and says how instead.
//! Portable installs under `ENCHI_ROOT` have no marker and keep updating themselves exactly as
//! earlier releases did.

use std::fs;
use std::path::Path;

/// Marker location relative to the prefix that holds `bin/enchi`.
pub const INSTALL_SOURCE_MARKER: &str = "share/enchi/install-source";

/// Where pyenv-native packages put it. Still read, so an install that predates the rename is
/// recognised as package-managed.
const LEGACY_INSTALL_SOURCE_MARKER: &str = "share/pyenv-native/install-source";

/// Marker inside a macOS `.app` (`Contents/Resources/install-source`), for the GUI executable,
/// whose prefix is `Contents` rather than the system prefix.
const APP_MARKER: &str = "Resources/install-source";

/// The package format that owns the executable at `exe`, when there is one.
pub fn package_source_for(exe: &Path) -> Option<String> {
    let resolved = fs::canonicalize(exe).unwrap_or_else(|_| exe.to_path_buf());
    let prefix = resolved.parent()?.parent()?;
    let text = fs::read_to_string(prefix.join(INSTALL_SOURCE_MARKER))
        .or_else(|_| fs::read_to_string(prefix.join(LEGACY_INSTALL_SOURCE_MARKER)))
        .or_else(|_| fs::read_to_string(prefix.join(APP_MARKER)))
        .ok()?;
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
        "deb" => "sudo apt update && sudo apt install --only-upgrade enchi",
        "rpm" => "sudo dnf upgrade enchi",
        "termux" => "pkg upgrade enchi",
        "pkg" => {
            "download the newer enchi-macos-<arch>.pkg from https://github.com/imyourboyroy/pyenv-native/releases/latest and open it (installing over the old version upgrades it)"
        }
        _ => "your package manager's upgrade command",
    }
}

/// The command that removes the program files of a package-managed install.
pub fn remove_command(source: &str) -> &'static str {
    match source {
        "deb" => "sudo apt remove enchi",
        "rpm" => "sudo dnf remove enchi",
        "termux" => "pkg uninstall enchi",
        "pkg" => "sudo enchi self-uninstall --system",
        _ => "your package manager's remove command",
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
    fn a_pyenv_native_marker_still_counts() {
        let temp = TempDir::new().unwrap();
        let bin = temp.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        let exe = bin.join("pyenv");
        fs::write(&exe, "x").unwrap();
        let marker = temp.path().join(LEGACY_INSTALL_SOURCE_MARKER);
        fs::create_dir_all(marker.parent().unwrap()).unwrap();
        fs::write(marker, "rpm\n").unwrap();
        assert_eq!(package_source_for(&exe).as_deref(), Some("rpm"));
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
        for source in ["deb", "rpm", "termux", "pkg"] {
            assert!(!upgrade_command(source).starts_with("your "));
            assert!(!remove_command(source).starts_with("your "));
        }
    }

    #[test]
    fn the_gui_inside_an_app_bundle_finds_its_marker() {
        let temp = TempDir::new().unwrap();
        let macos = temp.path().join("Pyenv Native.app/Contents/MacOS");
        let resources = temp.path().join("Pyenv Native.app/Contents/Resources");
        fs::create_dir_all(&macos).unwrap();
        fs::create_dir_all(&resources).unwrap();
        let exe = macos.join("enchi-gui");
        fs::write(&exe, "x").unwrap();
        fs::write(resources.join("install-source"), "pkg\n").unwrap();
        assert_eq!(package_source_for(&exe).as_deref(), Some("pkg"));
    }
}
