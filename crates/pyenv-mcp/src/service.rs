// ./crates/pyenv-mcp/src/service.rs
//! MCP server implementation for pyenv-native with structured, agent-friendly tools.

use anyhow::Result;
use rmcp::{
    Json, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::model::{
    AvailableVersionsParams, EnsureProjectVenvParams, EnsureRuntimeParams,
    InstallInstructionParams, JsonForwardResponse, ProjectPathParams, ProjectVenvResponse,
    RuntimeInventory, SetGlobalVersionParams, SetLocalVersionParams, ToolkitGuide,
    VersionCatalogResponse, VersionSelectionResponse,
};
use crate::ops::{
    DEFAULT_GITHUB_REPO, DEFAULT_SERVER_NAME, build_context, build_install_instructions,
    build_toolkit_guide, doctor_response, ensure_project_venv_response, ensure_runtime_response,
    list_available_versions_response, resolve_runtime_inventory, set_global_versions_response,
    set_local_versions_response,
};

#[derive(Debug, Clone)]
pub struct PyenvNativeMcpServer {
    tool_router: ToolRouter<Self>,
}

impl Default for PyenvNativeMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl PyenvNativeMcpServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for PyenvNativeMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Use resolve_project_environment before making changes. Prefer ensure_runtime and ensure_project_venv over shelling out. When unsure, call get_toolkit_guide first. Prefer project-local .venv environments for agent work.",
        )
    }
}

#[tool_router(router = tool_router)]
impl PyenvNativeMcpServer {
    #[tool(
        name = "get_toolkit_guide",
        description = "Return a structured JSON guide explaining how an agent should use pyenv-native, including install commands for pyenv-native itself, MCP client config, and recommended tool order."
    )]
    pub async fn get_toolkit_guide(
        &self,
        Parameters(params): Parameters<InstallInstructionParams>,
    ) -> Result<Json<ToolkitGuide>, String> {
        let ctx = build_context(None).map_err(|error| error.to_string())?;
        let github_repo = params
            .github_repo
            .unwrap_or_else(|| DEFAULT_GITHUB_REPO.to_string());
        let server_name = params
            .server_name
            .unwrap_or_else(|| DEFAULT_SERVER_NAME.to_string());
        let mcp_command = params
            .mcp_command
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("pyenv-mcp"))
            });
        let pyenv_root = params.pyenv_root.unwrap_or_else(|| ctx.root.clone());
        Ok(Json(build_toolkit_guide(
            &github_repo,
            params.install_root.as_deref(),
            &server_name,
            &mcp_command,
            &pyenv_root,
        )))
    }

    #[tool(
        name = "get_install_instructions",
        description = "Return structured install and uninstall commands for pyenv-native itself plus an MCP client JSON config snippet. Useful when the agent needs to tell a user or another system how to install the toolkit."
    )]
    pub async fn get_install_instructions(
        &self,
        Parameters(params): Parameters<InstallInstructionParams>,
    ) -> Result<Json<crate::model::InstallInstructions>, String> {
        let ctx = build_context(None).map_err(|error| error.to_string())?;
        let github_repo = params
            .github_repo
            .unwrap_or_else(|| DEFAULT_GITHUB_REPO.to_string());
        let server_name = params
            .server_name
            .unwrap_or_else(|| DEFAULT_SERVER_NAME.to_string());
        let mcp_command = params
            .mcp_command
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("pyenv-mcp"))
            });
        let pyenv_root = params.pyenv_root.unwrap_or_else(|| ctx.root.clone());
        Ok(Json(build_install_instructions(
            &github_repo,
            params.install_root.as_deref(),
            &server_name,
            &mcp_command,
            &pyenv_root,
        )))
    }

    #[tool(
        name = "doctor",
        description = "Return the machine-readable pyenv-native doctor report for the current machine or an optional project directory."
    )]
    pub async fn doctor(
        &self,
        Parameters(params): Parameters<ProjectPathParams>,
    ) -> Result<Json<JsonForwardResponse>, String> {
        let ctx = build_context(params.project_dir).map_err(|error| error.to_string())?;
        doctor_response(&ctx)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        name = "resolve_project_environment",
        description = "Resolve the effective Python environment for a project directory: installed versions, selected versions, version origin, version file path, and the best available interpreter path."
    )]
    pub async fn resolve_project_environment(
        &self,
        Parameters(params): Parameters<ProjectPathParams>,
    ) -> Result<Json<RuntimeInventory>, String> {
        let ctx = build_context(params.project_dir).map_err(|error| error.to_string())?;
        Ok(Json(resolve_runtime_inventory(&ctx)))
    }

    #[tool(
        name = "list_available_versions",
        description = "List installable Python runtimes grouped by family. By default this is provider-backed. Set known=true to ask for the broader known catalog instead."
    )]
    pub async fn list_available_versions(
        &self,
        Parameters(params): Parameters<AvailableVersionsParams>,
    ) -> Result<Json<VersionCatalogResponse>, String> {
        let ctx = build_context(None).map_err(|error| error.to_string())?;
        list_available_versions_response(
            &ctx,
            params.family,
            params.pattern,
            params.known.unwrap_or(false),
        )
        .map(Json)
        .map_err(|error| error.to_string())
    }

    #[tool(
        name = "ensure_runtime",
        description = "Ensure that a managed Python runtime exists. This is the idempotent runtime installer for pyenv-native and returns structured install metadata."
    )]
    pub async fn ensure_runtime(
        &self,
        Parameters(params): Parameters<EnsureRuntimeParams>,
    ) -> Result<Json<crate::model::EnsureRuntimeResponse>, String> {
        let ctx = build_context(None).map_err(|error| error.to_string())?;
        ensure_runtime_response(&ctx, &params.version, params.force.unwrap_or(false))
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        name = "set_local_version",
        description = "Write a local .python-version file for a project directory using one or more versions. Prefer this over editing files manually."
    )]
    pub async fn set_local_version(
        &self,
        Parameters(params): Parameters<SetLocalVersionParams>,
    ) -> Result<Json<VersionSelectionResponse>, String> {
        let ctx = build_context(params.project_dir).map_err(|error| error.to_string())?;
        set_local_versions_response(&ctx, &params.versions, params.force.unwrap_or(false))
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        name = "set_global_version",
        description = "Write the global pyenv-native version file using one or more versions, or unset it."
    )]
    pub async fn set_global_version(
        &self,
        Parameters(params): Parameters<SetGlobalVersionParams>,
    ) -> Result<Json<VersionSelectionResponse>, String> {
        let ctx = build_context(None).map_err(|error| error.to_string())?;
        set_global_versions_response(&ctx, &params.versions, params.unset.unwrap_or(false))
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        name = "ensure_project_venv",
        description = "Create or reuse a project-local virtual environment, defaulting to <project>/.venv. Optionally installs the requested runtime first and can write the project's local version file."
    )]
    pub async fn ensure_project_venv(
        &self,
        Parameters(params): Parameters<EnsureProjectVenvParams>,
    ) -> Result<Json<ProjectVenvResponse>, String> {
        let ctx = build_context(params.project_dir.clone()).map_err(|error| error.to_string())?;
        ensure_project_venv_response(
            &ctx,
            params.version,
            params.venv_path,
            params.install_if_missing.unwrap_or(true),
            params.set_local_version.unwrap_or(false),
        )
        .map(Json)
        .map_err(|error| error.to_string())
    }
}
