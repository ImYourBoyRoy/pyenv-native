// ./crates/pyenv-core/src/manage/types.rs
//! Shared option and entry models for manage command handlers.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VersionsCommandOptions {
    pub bare: bool,
    pub skip_aliases: bool,
    pub skip_envs: bool,
    pub executables: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VersionEntry {
    pub name: String,
    pub link_target: Option<PathBuf>,
}
