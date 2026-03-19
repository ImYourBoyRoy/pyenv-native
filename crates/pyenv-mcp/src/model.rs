// ./crates/pyenv-mcp/src/model.rs
//! Shared MCP request and response models for pyenv-native's agent-facing server.

use std::collections::BTreeMap;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolkitGuide {
    pub server_name: String,
    pub purpose: String,
    pub recommended_sequence: Vec<GuideStep>,
    pub install: InstallInstructions,
    pub tool_summaries: Vec<ToolSummary>,
    pub common_workflows: Vec<WorkflowRecipe>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuideStep {
    pub step: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolSummary {
    pub tool_name: String,
    pub use_when: String,
    pub returns: String,
    pub side_effects: String,
    pub arguments: Vec<ToolArgument>,
    pub example_input: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolArgument {
    pub name: String,
    pub required: bool,
    pub data_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowRecipe {
    pub name: String,
    pub goal: String,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowStep {
    pub tool_name: Option<String>,
    pub description: String,
    pub example_input: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InstallInstructions {
    pub github_repo: String,
    pub default_install_roots: PlatformInstallCommands,
    pub latest_release: PlatformInstallCommands,
    pub pinned_release_example: PlatformInstallCommands,
    pub pip_bootstrap: BootstrapInstallCommands,
    pub uninstall: PlatformInstallCommands,
    pub mcp_client_config: McpClientConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlatformInstallCommands {
    pub windows_powershell: String,
    pub linux_or_macos: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BootstrapInstallCommands {
    pub pipx: String,
    pub pip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpClientConfig {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: BTreeMap<String, McpServerEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpServerEntry {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct ProjectPathParams {
    pub project_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct InstallInstructionParams {
    pub github_repo: Option<String>,
    pub install_root: Option<PathBuf>,
    pub server_name: Option<String>,
    pub mcp_command: Option<String>,
    pub pyenv_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct AvailableVersionsParams {
    pub family: Option<String>,
    pub pattern: Option<String>,
    pub known: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EnsureRuntimeParams {
    pub version: String,
    pub force: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SetLocalVersionParams {
    pub versions: Vec<String>,
    pub project_dir: Option<PathBuf>,
    pub force: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SetGlobalVersionParams {
    pub versions: Vec<String>,
    pub unset: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EnsureProjectVenvParams {
    pub project_dir: Option<PathBuf>,
    pub version: Option<String>,
    pub venv_path: Option<PathBuf>,
    pub install_if_missing: Option<bool>,
    pub set_local_version: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VersionCatalogResponse {
    pub provider_backed: bool,
    pub family_filter: Option<String>,
    pub pattern_filter: Option<String>,
    pub groups: Vec<VersionCatalogGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VersionCatalogGroup {
    pub family: String,
    pub family_slug: String,
    pub source: Option<String>,
    pub provider: Option<String>,
    pub architecture: Option<String>,
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RuntimeInventory {
    pub root: PathBuf,
    pub installed_versions: Vec<String>,
    pub selected_versions: Vec<String>,
    pub missing_versions: Vec<String>,
    pub version_origin: String,
    pub version_file_path: PathBuf,
    pub primary_version: Option<String>,
    pub primary_interpreter: Option<PathBuf>,
    pub shims_dir: PathBuf,
    pub versions_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EnsureRuntimeResponse {
    pub requested_version: String,
    pub resolved_version: String,
    pub already_installed: bool,
    pub provider: String,
    pub family: String,
    pub architecture: String,
    pub install_dir: PathBuf,
    pub python_executable: PathBuf,
    pub receipt_path: Option<PathBuf>,
    pub pip_bootstrapped: bool,
    pub base_venv_created: bool,
    pub progress_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VersionSelectionResponse {
    pub scope: String,
    pub version_file_path: PathBuf,
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectVenvResponse {
    pub project_dir: PathBuf,
    pub requested_version: Option<String>,
    pub resolved_version: String,
    pub runtime_installed: bool,
    pub local_version_written: bool,
    pub venv_path: PathBuf,
    pub python_path: PathBuf,
    pub pip_path: PathBuf,
    pub created: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JsonForwardResponse {
    pub payload: Value,
}
