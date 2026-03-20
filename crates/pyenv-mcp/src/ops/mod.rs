// ./crates/pyenv-mcp/src/ops/mod.rs
//! Structured helper operations for the pyenv-native MCP server and companion CLI.

mod context;
mod docs;
mod project;
mod runtime;
mod tests;

pub(crate) use context::{
    DEFAULT_GITHUB_REPO, DEFAULT_SERVER_NAME, build_client_config, build_context,
};
pub(crate) use docs::{build_install_instructions, build_toolkit_guide};
pub(crate) use project::{
    ensure_project_venv_response, set_global_versions_response, set_local_versions_response,
};
pub(crate) use runtime::{
    doctor_response, ensure_runtime_response, list_available_versions_response,
    resolve_runtime_inventory,
};
