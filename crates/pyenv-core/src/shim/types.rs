// ./crates/pyenv-core/src/shim/types.rs
//! Shared shim models and guards for exec/rehash workflows.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub(super) const SHIM_MANIFEST_FILE: &str = ".pyenv-shims.json";
pub(super) const SHIM_LOCK_FILE: &str = ".pyenv-shims.lock";
pub(super) const SHIM_LOCK_STALE_SECS: u64 = 60 * 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(super) struct ShimManifest {
    pub generated_at_epoch_seconds: u64,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExecTarget {
    pub executable: PathBuf,
    pub prefix_dirs: Vec<PathBuf>,
    pub version_name: Option<String>,
}

#[derive(Debug)]
pub(super) struct RehashLockGuard {
    pub path: PathBuf,
}

impl Drop for RehashLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
