// ./crates/pyenv-mcp/src/ops/docs/mod.rs
//! Toolkit-guide and install-instruction builders for MCP clients and AI onboarding flows.

mod guide;
mod install;
mod summaries;
mod workflows;

pub(crate) use guide::build_toolkit_guide;
pub(crate) use install::build_install_instructions;
