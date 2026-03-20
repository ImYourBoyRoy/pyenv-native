// ./crates/pyenv-mcp/src/ops/docs/guide.rs
//! High-level toolkit-guide composition for agent onboarding and recommended workflows.

use std::path::Path;

use crate::model::{GuideStep, ToolkitGuide};

use super::install::build_install_instructions;
use super::summaries::build_tool_summaries;
use super::workflows::build_common_workflows;

pub(crate) fn build_toolkit_guide(
    github_repo: &str,
    install_root: Option<&Path>,
    server_name: &str,
    mcp_command: &Path,
    pyenv_root: &Path,
) -> ToolkitGuide {
    ToolkitGuide {
        server_name: server_name.to_string(),
        purpose: "Manage pyenv-native, discover project Python requirements, install runtimes, and create project-local virtual environments with structured JSON responses.".to_string(),
        recommended_sequence: vec![
            GuideStep { step: "Call get_toolkit_guide first when the model does not already understand pyenv-native".to_string(), reason: "It returns install commands, MCP client config, tool summaries, and recommended workflows in one JSON blob.".to_string() },
            GuideStep { step: "Call resolve_project_environment before making changes".to_string(), reason: "It tells the agent what Python version is active, where it came from, and whether it is missing.".to_string() },
            GuideStep { step: "Call ensure_runtime when the project version is missing or when you need a specific runtime".to_string(), reason: "This is the idempotent runtime installer for managed Python versions.".to_string() },
            GuideStep { step: "Call ensure_project_venv for project work".to_string(), reason: "It creates or reuses a predictable project-local .venv and returns concrete python/pip paths.".to_string() },
            GuideStep { step: "Use doctor when anything looks odd".to_string(), reason: "It returns machine-readable diagnostics about path issues, roots, shims, and host readiness.".to_string() },
        ],
        install: build_install_instructions(github_repo, install_root, server_name, mcp_command, pyenv_root),
        tool_summaries: build_tool_summaries(),
        common_workflows: build_common_workflows(),
        notes: vec![
            "The MCP server is agent-first: prefer structured tools over shelling out to pyenv manually.".to_string(),
            "Project virtual environments default to <project>/.venv so IDEs and agents can find them easily.".to_string(),
            "The guide includes direct install commands for pyenv-native itself, but the MCP server assumes pyenv-native is already installed when the server is running.".to_string(),
        ],
    }
}
