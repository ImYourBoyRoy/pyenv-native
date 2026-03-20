// ./crates/pyenv-core/src/plugin/types.rs
//! Shared models for plugin hook execution and parsed hook actions.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookResult {
    pub path: PathBuf,
    pub stdout: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct HookActions {
    pub command_path: Option<PathBuf>,
    pub prepend_paths: Vec<PathBuf>,
    pub env_pairs: Vec<(String, String)>,
    pub passthrough_lines: Vec<String>,
}
