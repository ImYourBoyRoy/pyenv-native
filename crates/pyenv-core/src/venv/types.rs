// ./crates/pyenv-core/src/venv/types.rs
//! Shared managed-venv types used by CLI, MCP, and future GUI-facing workflows.

use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VenvUseScope {
    Local,
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagedVenvInfo {
    pub name: String,
    pub base_version: String,
    pub spec: String,
    pub path: PathBuf,
    pub python_path: Option<PathBuf>,
    pub pip_path: Option<PathBuf>,
}
