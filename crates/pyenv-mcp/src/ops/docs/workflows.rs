// ./crates/pyenv-mcp/src/ops/docs/workflows.rs
//! Common MCP workflow recipes for toolkit onboarding and project preparation.

use serde_json::json;

use crate::model::{WorkflowRecipe, WorkflowStep};

pub(super) fn build_common_workflows() -> Vec<WorkflowRecipe> {
    vec![
        WorkflowRecipe {
            name: "install_pyenv_native".to_string(),
            goal: "Teach a user or another system how to install pyenv-native itself and register the MCP server.".to_string(),
            steps: vec![
                WorkflowStep {
                    tool_name: Some("get_install_instructions".to_string()),
                    description: "Fetch the platform-specific web install command plus the MCP client config snippet.".to_string(),
                    example_input: Some(json!({})),
                },
                WorkflowStep {
                    tool_name: None,
                    description: "Tell the user to run the returned latest_release command for their platform, then add the returned mcp_client_config block to their MCP client.".to_string(),
                    example_input: None,
                },
            ],
        },
        WorkflowRecipe {
            name: "install_cpython_runtime".to_string(),
            goal: "Install a CPython runtime like 3.12 or 3.13 using provider-backed resolution.".to_string(),
            steps: vec![
                WorkflowStep {
                    tool_name: Some("list_available_versions".to_string()),
                    description: "Optional: inspect installable CPython choices before selecting one.".to_string(),
                    example_input: Some(json!({ "family": "cpython", "pattern": "3.13" })),
                },
                WorkflowStep {
                    tool_name: Some("ensure_runtime".to_string()),
                    description: "Install or reuse the selected CPython version.".to_string(),
                    example_input: Some(json!({ "version": "3.13" })),
                },
            ],
        },
        WorkflowRecipe {
            name: "install_pypy_runtime".to_string(),
            goal: "Install a PyPy runtime using provider-backed resolution.".to_string(),
            steps: vec![
                WorkflowStep {
                    tool_name: Some("list_available_versions".to_string()),
                    description: "Optional: inspect installable PyPy choices before selecting one.".to_string(),
                    example_input: Some(json!({ "family": "pypy", "pattern": "pypy3.11" })),
                },
                WorkflowStep {
                    tool_name: Some("ensure_runtime".to_string()),
                    description: "Install or reuse the selected PyPy version.".to_string(),
                    example_input: Some(json!({ "version": "pypy3.11" })),
                },
            ],
        },
        WorkflowRecipe {
            name: "prepare_project_environment".to_string(),
            goal: "Resolve the right Python for a project, ensure it exists, and create a project-local .venv.".to_string(),
            steps: vec![
                WorkflowStep {
                    tool_name: Some("resolve_project_environment".to_string()),
                    description: "Inspect the project to see what version it wants and whether anything is missing.".to_string(),
                    example_input: Some(json!({ "project_dir": "/workspace/app" })),
                },
                WorkflowStep {
                    tool_name: Some("ensure_project_venv".to_string()),
                    description: "Create or reuse <project>/.venv and optionally write the local version file.".to_string(),
                    example_input: Some(json!({ "project_dir": "/workspace/app", "install_if_missing": true, "set_local_version": true })),
                },
            ],
        },
    ]
}
