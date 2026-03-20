// ./crates/pyenv-mcp/src/ops/docs/summaries.rs
//! Tool-summary builders used in the MCP toolkit guide.

use serde_json::json;

use crate::model::{ToolArgument, ToolSummary};

pub(super) fn build_tool_summaries() -> Vec<ToolSummary> {
    vec![
        ToolSummary {
            tool_name: "get_toolkit_guide".to_string(),
            use_when: "The model needs a single structured orientation blob before doing anything else.".to_string(),
            returns: "Install instructions, MCP client config, common workflows, and tool summaries.".to_string(),
            side_effects: "Read-only.".to_string(),
            arguments: vec![
                ToolArgument { name: "github_repo".to_string(), required: false, data_type: "string".to_string(), description: "Override the GitHub owner/repo used in the generated install commands.".to_string() },
                ToolArgument { name: "install_root".to_string(), required: false, data_type: "path".to_string(), description: "Override the install root that should appear in example commands.".to_string() },
                ToolArgument { name: "server_name".to_string(), required: false, data_type: "string".to_string(), description: "Override the MCP server name used in the returned client config snippet.".to_string() },
            ],
            example_input: Some(json!({})),
        },
        ToolSummary {
            tool_name: "get_install_instructions".to_string(),
            use_when: "You need only install and uninstall commands plus the MCP config snippet without the rest of the guide.".to_string(),
            returns: "Platform-specific install commands for pyenv-native itself, uninstall commands, and an MCP client config block.".to_string(),
            side_effects: "Read-only.".to_string(),
            arguments: vec![
                ToolArgument { name: "github_repo".to_string(), required: false, data_type: "string".to_string(), description: "Override the GitHub owner/repo used in the generated install commands.".to_string() },
                ToolArgument { name: "install_root".to_string(), required: false, data_type: "path".to_string(), description: "Override the install root that should appear in example commands.".to_string() },
            ],
            example_input: Some(json!({ "install_root": "~/.pyenv" })),
        },
        ToolSummary {
            tool_name: "resolve_project_environment".to_string(),
            use_when: "You need to know what Python version a folder should use before taking action.".to_string(),
            returns: "Selected versions, missing versions, version-file origin, installed versions, and the best-effort interpreter path.".to_string(),
            side_effects: "Read-only.".to_string(),
            arguments: vec![ToolArgument { name: "project_dir".to_string(), required: false, data_type: "path".to_string(), description: "Project directory to inspect. If omitted, the current working directory is used.".to_string() }],
            example_input: Some(json!({ "project_dir": "/workspace/app" })),
        },
        ToolSummary {
            tool_name: "list_available_versions".to_string(),
            use_when: "You need installable runtime choices or the broader known catalog before choosing a version.".to_string(),
            returns: "Grouped runtime families with optional provider, architecture, and source metadata.".to_string(),
            side_effects: "Read-only.".to_string(),
            arguments: vec![
                ToolArgument { name: "family".to_string(), required: false, data_type: "string".to_string(), description: "Optional family filter such as cpython or pypy.".to_string() },
                ToolArgument { name: "pattern".to_string(), required: false, data_type: "string".to_string(), description: "Optional prefix or pattern filter such as 3.13 or pypy3.11.".to_string() },
                ToolArgument { name: "known".to_string(), required: false, data_type: "boolean".to_string(), description: "When true, return the broader known catalog instead of only provider-backed installable versions.".to_string() },
            ],
            example_input: Some(json!({ "family": "cpython", "pattern": "3.13" })),
        },
        ToolSummary {
            tool_name: "ensure_runtime".to_string(),
            use_when: "A managed Python runtime must exist before project work can continue.".to_string(),
            returns: "Resolved version, provider, install directory, interpreter path, whether the runtime already existed, and structured progress steps describing what happened.".to_string(),
            side_effects: "Downloads and installs a runtime if it is not already present or if force=true is used.".to_string(),
            arguments: vec![
                ToolArgument { name: "version".to_string(), required: true, data_type: "string".to_string(), description: "Requested runtime version or prefix, such as 3.12, 3.13.12, or pypy3.11.".to_string() },
                ToolArgument { name: "force".to_string(), required: false, data_type: "boolean".to_string(), description: "Reinstall or replace an already-installed runtime at the same path.".to_string() },
            ],
            example_input: Some(json!({ "version": "3.12" })),
        },
        ToolSummary {
            tool_name: "set_local_version".to_string(),
            use_when: "You want a project to resolve to one or more specific managed runtimes.".to_string(),
            returns: "The written .python-version path and the versions that were stored there.".to_string(),
            side_effects: "Writes or overwrites a local .python-version file.".to_string(),
            arguments: vec![
                ToolArgument { name: "versions".to_string(), required: true, data_type: "array<string>".to_string(), description: "One or more runtime identifiers to store in the project's .python-version file.".to_string() },
                ToolArgument { name: "project_dir".to_string(), required: false, data_type: "path".to_string(), description: "Project directory where the .python-version file should be written.".to_string() },
                ToolArgument { name: "force".to_string(), required: false, data_type: "boolean".to_string(), description: "Overwrite a conflicting local version file when necessary.".to_string() },
            ],
            example_input: Some(json!({ "project_dir": "/workspace/app", "versions": ["3.12.10"] })),
        },
        ToolSummary {
            tool_name: "set_global_version".to_string(),
            use_when: "You want to change the default managed runtime for new shells or projects without local overrides.".to_string(),
            returns: "The global version file path and the versions now stored there.".to_string(),
            side_effects: "Writes or clears the global version file under PYENV_ROOT.".to_string(),
            arguments: vec![
                ToolArgument { name: "versions".to_string(), required: true, data_type: "array<string>".to_string(), description: "One or more runtime identifiers to store globally.".to_string() },
                ToolArgument { name: "unset".to_string(), required: false, data_type: "boolean".to_string(), description: "When true, clear the global version file instead of writing versions.".to_string() },
            ],
            example_input: Some(json!({ "versions": ["3.13.12"] })),
        },
        ToolSummary {
            tool_name: "ensure_project_venv".to_string(),
            use_when: "A project-local virtual environment should be created or reused in a predictable location.".to_string(),
            returns: "The concrete venv path plus python and pip paths that can be used immediately.".to_string(),
            side_effects: "May install a missing runtime, create a venv, and optionally write a local .python-version file.".to_string(),
            arguments: vec![
                ToolArgument { name: "project_dir".to_string(), required: false, data_type: "path".to_string(), description: "Project directory where the venv should live. Defaults to the current directory.".to_string() },
                ToolArgument { name: "version".to_string(), required: false, data_type: "string".to_string(), description: "Explicit runtime to use. If omitted, resolve from the project selection rules.".to_string() },
                ToolArgument { name: "venv_path".to_string(), required: false, data_type: "path".to_string(), description: "Explicit venv path. Defaults to <project>/.venv.".to_string() },
                ToolArgument { name: "install_if_missing".to_string(), required: false, data_type: "boolean".to_string(), description: "Install the selected runtime first when it is missing.".to_string() },
                ToolArgument { name: "set_local_version".to_string(), required: false, data_type: "boolean".to_string(), description: "Also write the chosen runtime into the project's .python-version file.".to_string() },
            ],
            example_input: Some(json!({ "project_dir": "/workspace/app", "version": "3.12", "install_if_missing": true, "set_local_version": true })),
        },
        ToolSummary {
            tool_name: "doctor".to_string(),
            use_when: "Something about the install, shell, shims, or host toolchain looks wrong.".to_string(),
            returns: "The same structured doctor payload available from pyenv doctor --json.".to_string(),
            side_effects: "Read-only.".to_string(),
            arguments: vec![ToolArgument { name: "project_dir".to_string(), required: false, data_type: "path".to_string(), description: "Optional project directory for context-sensitive diagnostics.".to_string() }],
            example_input: Some(json!({})),
        },
    ]
}
