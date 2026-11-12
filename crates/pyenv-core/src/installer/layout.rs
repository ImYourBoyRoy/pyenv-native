// ./crates/pyenv-core/src/installer/layout.rs
//! Portable install-root layout shared by install, update, uninstall, and the GUI.

use std::path::{Path, PathBuf};

/// Base names of the executables shipped in a release bundle.
pub const PYENV_BINARY: &str = "pyenv";
pub const MCP_BINARY: &str = "pyenv-mcp";
pub const GUI_BINARY: &str = "pyenv-gui";

/// Platform-specific file name for an executable base name (`pyenv` -> `pyenv.exe` on Windows).
pub fn executable_file_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

/// Directory layout of a portable pyenv-native install rooted at `PYENV_ROOT`.
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

    pub fn pyenv_exe(&self) -> PathBuf {
        self.bin_dir().join(executable_file_name(PYENV_BINARY))
    }

    pub fn mcp_exe(&self) -> PathBuf {
        self.bin_dir().join(executable_file_name(MCP_BINARY))
    }

    pub fn gui_exe(&self) -> PathBuf {
        self.bin_dir().join(executable_file_name(GUI_BINARY))
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

    /// Whether the root contains pyenv-native's own directories. Used as a safety check before
    /// destructive operations so a mistyped root can never be mistaken for an install.
    pub fn looks_like_install(&self) -> bool {
        self.pyenv_exe().is_file()
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
        assert!(layout.pyenv_exe().starts_with("/opt/pyenv/bin"));
        let name = layout.pyenv_exe();
        let name = name.file_name().unwrap().to_string_lossy().to_string();
        assert_eq!(name, executable_file_name("pyenv"));
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
