// ./crates/enchi-core/src/installer/layout.rs
//! Portable install-root layout shared by install, update, uninstall, and the GUI.

use std::path::{Path, PathBuf};

use crate::brand;

/// Base names of the executables shipped in a release bundle.
pub const CLI_BINARY: &str = brand::CLI;
pub const MCP_BINARY: &str = brand::MCP;
pub const GUI_BINARY: &str = brand::GUI;

/// Names earlier releases installed; kept as copies of the current binaries when upgrading.
pub const LEGACY_CLI_BINARY: &str = brand::LEGACY_CLI;
pub const LEGACY_MCP_BINARY: &str = brand::LEGACY_MCP;
pub const LEGACY_GUI_BINARY: &str = brand::LEGACY_GUI;

/// Platform-specific file name for an executable base name (`enchi` -> `enchi.exe` on Windows).
pub fn executable_file_name(base: &str) -> String {
    brand::exe_name(base)
}

/// Directory layout of a portable Enchi install rooted at `ENCHI_ROOT` (or the legacy `PYENV_ROOT`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallLayout {
    root: PathBuf,
}

impl InstallLayout {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }

    pub fn shims_dir(&self) -> PathBuf {
        self.root.join("shims")
    }

    pub fn versions_dir(&self) -> PathBuf {
        self.root.join("versions")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn share_dir(&self) -> PathBuf {
        self.root.join("share")
    }

    pub fn cli_exe(&self) -> PathBuf {
        self.bin_dir().join(executable_file_name(CLI_BINARY))
    }

    pub fn mcp_exe(&self) -> PathBuf {
        self.bin_dir().join(executable_file_name(MCP_BINARY))
    }

    pub fn gui_exe(&self) -> PathBuf {
        self.bin_dir().join(executable_file_name(GUI_BINARY))
    }

    pub fn legacy_cli_exe(&self) -> PathBuf {
        self.bin_dir().join(executable_file_name(LEGACY_CLI_BINARY))
    }

    pub fn legacy_mcp_exe(&self) -> PathBuf {
        self.bin_dir().join(executable_file_name(LEGACY_MCP_BINARY))
    }

    pub fn legacy_gui_exe(&self) -> PathBuf {
        self.bin_dir().join(executable_file_name(LEGACY_GUI_BINARY))
    }

    /// `(current, legacy)` executable pairs, for installing aliases and cleaning up.
    pub fn alias_pairs(&self) -> [(PathBuf, PathBuf); 3] {
        [
            (self.cli_exe(), self.legacy_cli_exe()),
            (self.mcp_exe(), self.legacy_mcp_exe()),
            (self.gui_exe(), self.legacy_gui_exe()),
        ]
    }

    /// An install made by Enchi: the old `enchi` binary is there and Enchi's is not yet.
    pub fn is_legacy_install(&self) -> bool {
        self.legacy_cli_exe().is_file() && !self.cli_exe().is_file()
    }

    /// Directories created for a fresh install.
    pub fn managed_dirs(&self) -> [PathBuf; 5] {
        [
            self.bin_dir(),
            self.shims_dir(),
            self.versions_dir(),
            self.cache_dir(),
            self.logs_dir(),
        ]
    }

    /// Whether the root contains Enchi's own directories. Used as a safety check before
    /// destructive operations so a mistyped root can never be mistaken for an install.
    pub fn looks_like_install(&self) -> bool {
        self.cli_exe().is_file()
            || self.legacy_cli_exe().is_file()
            || ["bin", "shims", "versions"]
                .iter()
                .any(|name| self.root.join(name).is_dir())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_paths_hang_off_the_root() {
        let layout = InstallLayout::new("/opt/pyenv");
        assert_eq!(layout.bin_dir(), Path::new("/opt/pyenv/bin"));
        assert_eq!(layout.shims_dir(), Path::new("/opt/pyenv/shims"));
        assert!(layout.cli_exe().starts_with("/opt/pyenv/bin"));
        let name = layout.cli_exe();
        let name = name.file_name().unwrap().to_string_lossy().to_string();
        assert_eq!(name, executable_file_name("enchi"));
        assert_eq!(
            layout
                .legacy_cli_exe()
                .file_name()
                .unwrap()
                .to_string_lossy(),
            executable_file_name("pyenv")
        );
    }

    #[test]
    fn a_pyenv_only_bin_dir_is_a_legacy_install() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let layout = InstallLayout::new(temp.path());
        std::fs::create_dir_all(layout.bin_dir()).unwrap();
        assert!(!layout.is_legacy_install());
        std::fs::write(layout.legacy_cli_exe(), "x").unwrap();
        assert!(layout.is_legacy_install());
        assert!(layout.looks_like_install());
        std::fs::write(layout.cli_exe(), "x").unwrap();
        assert!(!layout.is_legacy_install());
    }

    #[test]
    fn empty_directory_is_not_an_install() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let layout = InstallLayout::new(temp.path());
        assert!(!layout.looks_like_install());
        std::fs::create_dir_all(layout.versions_dir()).expect("versions");
        assert!(layout.looks_like_install());
    }
}
