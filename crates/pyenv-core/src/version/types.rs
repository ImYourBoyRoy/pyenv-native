// ./crates/pyenv-core/src/version/types.rs
//! Shared version-selection data types used by version-file parsing, runtime selection, and
//! command rendering.

use std::path::PathBuf;

use crate::error::PyenvError;

pub(super) const LOCAL_VERSION_FILE: &str = ".python-version";
pub(super) const GLOBAL_VERSION_FILE: &str = "version";

#[derive(Debug)]
pub(super) struct ParsedVersionFile {
    pub(super) versions: Vec<String>,
    pub(super) warnings: Vec<PyenvError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionOrigin {
    Environment,
    File(PathBuf),
}

impl std::fmt::Display for VersionOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Environment => write!(f, "PYENV_VERSION environment variable"),
            Self::File(path) => write!(f, "{}", path.display()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedVersions {
    pub versions: Vec<String>,
    pub missing: Vec<String>,
    pub origin: VersionOrigin,
}
