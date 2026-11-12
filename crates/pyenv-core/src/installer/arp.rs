// ./crates/pyenv-core/src/installer/arp.rs
//! Windows "Apps & features" (Add/Remove Programs) registration.
//!
//! Every install path (the NSIS installer, `install.ps1`, `pyenv self-update`, the pipx
//! bootstrap) ends in `pyenv self-install`, so registering here keeps the entry correct for all of
//! them, including installs created by earlier releases that gain the entry on their next update.
//! The entry lives under HKCU, so no elevation is required, and it is removed by
//! `pyenv self-uninstall`. Elsewhere these functions do nothing.

use std::path::Path;

use super::layout::InstallLayout;

pub(crate) const DISPLAY_NAME: &str = "Pyenv Native";
pub(crate) const PUBLISHER: &str = "pyenv-native";
pub(crate) const HELP_LINK: &str = "https://github.com/imyourboyroy/pyenv-native";

/// The values written for the Apps & features entry. Pure data so it is testable on any host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UninstallEntry {
    pub display_name: String,
    pub display_version: String,
    pub publisher: String,
    pub install_location: String,
    pub display_icon: String,
    pub uninstall_string: String,
    pub quiet_uninstall_string: String,
    pub help_link: String,
    pub estimated_size_kb: u32,
}

fn quoted(path: &Path) -> String {
    format!("\"{}\"", path.display())
}

pub(crate) fn uninstall_entry(layout: &InstallLayout, version: &str) -> UninstallEntry {
    let pyenv = layout.pyenv_exe();
    let icon = if layout.gui_exe().is_file() {
        layout.gui_exe()
    } else {
        pyenv.clone()
    };
    UninstallEntry {
        display_name: DISPLAY_NAME.to_string(),
        display_version: version.to_string(),
        publisher: PUBLISHER.to_string(),
        install_location: layout.root().display().to_string(),
        display_icon: quoted(&icon),
        uninstall_string: format!("{} self-uninstall", quoted(&pyenv)),
        quiet_uninstall_string: format!("{} self-uninstall --yes", quoted(&pyenv)),
        help_link: HELP_LINK.to_string(),
        estimated_size_kb: program_size_kb(layout),
    }
}

/// Size of the program files (not the Python versions the user installed) in KiB.
fn program_size_kb(layout: &InstallLayout) -> u32 {
    let bytes: u64 = std::fs::read_dir(layout.bin_dir())
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|entry| entry.metadata().ok())
                .filter(|meta| meta.is_file())
                .map(|meta| meta.len())
                .sum()
        })
        .unwrap_or(0);
    u32::try_from(bytes / 1024).unwrap_or(u32::MAX)
}

/// Create or refresh the Apps & features entry. Returns whether the platform supports it.
pub fn register(layout: &InstallLayout, version: &str) -> std::io::Result<bool> {
    platform::register(&uninstall_entry(layout, version))
}

/// Remove the entry if present. Returns whether one was removed.
pub fn unregister() -> std::io::Result<bool> {
    platform::unregister()
}

#[cfg(windows)]
mod platform {
    use std::io;

    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

    use super::UninstallEntry;

    /// Subkey name under `UNINSTALL_ROOT`; the NSIS setup reads the same key (`ARP_KEY`).
    const UNINSTALL_KEY_NAME: &str = "pyenv-native";
    const UNINSTALL_ROOT: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

    pub(super) fn register(entry: &UninstallEntry) -> io::Result<bool> {
        let root = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = root.create_subkey(format!(r"{UNINSTALL_ROOT}\{UNINSTALL_KEY_NAME}"))?;
        key.set_value("DisplayName", &entry.display_name)?;
        key.set_value("DisplayVersion", &entry.display_version)?;
        key.set_value("Publisher", &entry.publisher)?;
        key.set_value("InstallLocation", &entry.install_location)?;
        key.set_value("DisplayIcon", &entry.display_icon)?;
        key.set_value("UninstallString", &entry.uninstall_string)?;
        key.set_value("QuietUninstallString", &entry.quiet_uninstall_string)?;
        key.set_value("HelpLink", &entry.help_link)?;
        key.set_value("URLInfoAbout", &entry.help_link)?;
        key.set_value("EstimatedSize", &entry.estimated_size_kb)?;
        key.set_value("NoModify", &1u32)?;
        key.set_value("NoRepair", &1u32)?;
        Ok(true)
    }

    pub(super) fn unregister() -> io::Result<bool> {
        let parent = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(UNINSTALL_ROOT, KEY_READ | KEY_WRITE)?;
        match parent.delete_subkey_all(UNINSTALL_KEY_NAME) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use std::io;

    use super::UninstallEntry;

    pub(super) fn register(_entry: &UninstallEntry) -> io::Result<bool> {
        Ok(false)
    }

    pub(super) fn unregister() -> io::Result<bool> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_points_the_uninstaller_at_the_installed_binary() {
        let layout = InstallLayout::new("/home/u/.pyenv");
        let entry = uninstall_entry(&layout, "1.2.3");
        assert_eq!(entry.display_version, "1.2.3");
        assert_eq!(entry.install_location, "/home/u/.pyenv");
        assert!(entry.uninstall_string.ends_with(" self-uninstall"));
        assert!(entry.uninstall_string.starts_with('"'));
        assert!(
            entry
                .quiet_uninstall_string
                .ends_with(" self-uninstall --yes")
        );
    }

    #[test]
    fn registration_is_a_no_op_off_windows() {
        if cfg!(windows) {
            return;
        }
        let layout = InstallLayout::new("/tmp/none");
        assert!(!register(&layout, "1.0.0").unwrap());
        assert!(!unregister().unwrap());
    }
}
