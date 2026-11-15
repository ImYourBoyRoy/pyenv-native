// ./crates/enchi-mcp/src/service.rs
//! MCP server implementation for Enchi with structured, agent-friendly tools.

use anyhow::Result;
use rmcp::{
    Json, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router,
};

use crate::model::{
    AvailableVersionsParams, ConfigGetParams, ConfigSetParams, DoctorFixParams,
    EnsureProjectVenvParams, EnsureRuntimeParams, InstallInstructionParams, JsonForwardResponse,
    PipAnalyzeParams, PipInstallParams, PipListParams, PipPrecheckParams, PipUpdateParams,
    ProjectPathParams, ProjectVenvResponse, RuntimeInventory, SelfUpdateParams,
    SetGlobalVersionParams, SetLocalVersionParams, ToolkitGuide, UpdateAllPackagesParams,
    UpgradeRuntimesParams, VenvUpgradeParams, VersionCatalogResponse, VersionSelectionResponse,
};
use crate::ops::{
    DEFAULT_GITHUB_REPO, DEFAULT_SERVER_NAME, build_context, build_install_instructions,
    build_toolkit_guide, doctor_fix_response, doctor_response, ensure_project_venv_response,
    ensure_runtime_response, get_config_response, inspect_environment_response,
    list_available_versions_response, pip_analyze_imports_response, pip_check_response,
    pip_install_response, pip_list_response, pip_outdated_response, pip_precheck_response,
    pip_update_response, preflight_response, resolve_runtime_inventory, self_update_response,
    set_config_response, set_global_versions_response, set_local_versions_response,
    update_all_packages_response, upgrade_runtimes_response, venv_upgrade_response,
};

#[derive(Debug, Clone)]
pub struct EnchiMcpServer {
    tool_router: ToolRouter<Self>,
}

impl Default for EnchiMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl EnchiMcpServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for EnchiMcpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                DEFAULT_SERVER_NAME,
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
            "Use preflight before ensure_runtime on macOS/Android/Linux source builds. Use resolve_project_environment before making changes. Prefer ensure_runtime and ensure_project_venv over shelling out. When unsure, call get_toolkit_guide first. Prefer project-local .venv environments for agent work. Install progress is streamed to <root>/logs/install-progress.jsonl and returned in progress_steps.",
        )
    }
}

#[tool_router(router = tool_router)]
impl EnchiMcpServer {
    #[tool(
        name = "get_toolkit_guide",
        annotations(read_only_hint = true, open_world_hint = false),
        description = "Return a structured JSON guide explaining how an agent should use Enchi, including install commands for Enchi itself, MCP client config, and recommended tool order."
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
                std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("enchi-mcp"))
            });
        let enchi_root = params.enchi_root.unwrap_or_else(|| ctx.root.clone());
        Ok(Json(build_toolkit_guide(
            &github_repo,
            params.install_root.as_deref(),
            &server_name,
            &mcp_command,
            &enchi_root,
        )))
    }

    #[tool(
        name = "get_install_instructions",
        annotations(read_only_hint = true, open_world_hint = false),
        description = "Return structured install and uninstall commands for Enchi itself plus an MCP client JSON config snippet. Useful when the agent needs to tell a user or another system how to install the toolkit."
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
                std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("enchi-mcp"))
            });
        let enchi_root = params.enchi_root.unwrap_or_else(|| ctx.root.clone());
        Ok(Json(build_install_instructions(
            &github_repo,
            params.install_root.as_deref(),
            &server_name,
            &mcp_command,
            &enchi_root,
        )))
    }

    #[tool(
        name = "doctor",
        annotations(read_only_hint = true, open_world_hint = false),
        description = "Return the machine-readable Enchi doctor report for the current machine or an optional project directory."
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
        name = "doctor_fix",
        annotations(read_only_hint = false, destructive_hint = false),
        description = "Plan safe automated doctor repairs (shims/layout, Termux packages, best-effort macOS CLT/OpenSSL). Returns the plan only; pass apply=true to run it. Review the plan first: some fixes install system packages."
    )]
    pub async fn doctor_fix(
        &self,
        Parameters(params): Parameters<DoctorFixParams>,
    ) -> Result<Json<JsonForwardResponse>, String> {
        let ctx = build_context(params.project_dir).map_err(|error| error.to_string())?;
        doctor_fix_response(
            &ctx,
            params.apply == Some(true) || params.force == Some(true),
        )
        .map(Json)
        .map_err(|error| error.to_string())
    }

    #[tool(
        name = "preflight",
        annotations(read_only_hint = true, open_world_hint = false),
        description = "Return platform intelligence and install-readiness: OS/arch, install strategy, Xcode/CLT/OpenSSL on macOS, Termux readiness on Android, blocking issues, and recommended actions. Call this before ensure_runtime on source-build platforms."
    )]
    pub async fn preflight(
        &self,
        Parameters(params): Parameters<ProjectPathParams>,
    ) -> Result<Json<JsonForwardResponse>, String> {
        let ctx = build_context(params.project_dir).map_err(|error| error.to_string())?;
        preflight_response(&ctx)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        name = "resolve_project_environment",
        annotations(read_only_hint = true, open_world_hint = false),
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
        name = "inspect_environment",
        annotations(read_only_hint = true, open_world_hint = false),
        description = "Get a high-level overview of the active environment, including selected versions, their origins, and details about the active managed virtual environment if one is present."
    )]
    pub async fn inspect_environment(
        &self,
        Parameters(params): Parameters<ProjectPathParams>,
    ) -> Result<Json<crate::model::EnvironmentStatusProxy>, String> {
        let ctx = build_context(params.project_dir).map_err(|error| error.to_string())?;
        inspect_environment_response(&ctx)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        name = "list_available_versions",
        annotations(read_only_hint = true, open_world_hint = false),
        description = "List installable Python runtimes grouped by family. CPython versions are refreshed from the publisher index (python.org, or NuGet on Windows) and cached, so new patch releases appear without a toolkit update. Set known=true for the broader family catalog."
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
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true
        ),
        description = "Ensure that a managed Python runtime exists. This is the idempotent runtime installer for Enchi and returns structured install metadata."
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
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true
        ),
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
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true
        ),
        description = "Write the global Enchi version file using one or more versions, or unset it."
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
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true
        ),
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
            params.managed_venv_name,
            params.install_if_missing.unwrap_or(true),
            params.set_local_version.unwrap_or(false),
        )
        .map(Json)
        .map_err(|error| error.to_string())
    }

    #[tool(
        name = "pip_list",
        annotations(read_only_hint = true, open_world_hint = false),
        description = "List all installed pip packages in a target environment (runtimes like '3.13.2' or managed venv names like 'venv:testing')."
    )]
    pub async fn pip_list(
        &self,
        Parameters(params): Parameters<PipListParams>,
    ) -> Result<Json<JsonForwardResponse>, String> {
        let ctx = build_context(params.project_dir).map_err(|error| error.to_string())?;
        pip_list_response(&ctx, &params.target)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        name = "pip_outdated",
        annotations(read_only_hint = true, open_world_hint = false),
        description = "Audit outdated pip packages in a target environment against PyPI. Returns {name, version, latest_version}. Copy names or pins into pip_update; this JSON is not a pip_update argument."
    )]
    pub async fn pip_outdated(
        &self,
        Parameters(params): Parameters<PipListParams>,
    ) -> Result<Json<JsonForwardResponse>, String> {
        let ctx = build_context(params.project_dir).map_err(|error| error.to_string())?;
        pip_outdated_response(&ctx, &params.target)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        name = "pip_check",
        annotations(read_only_hint = true, open_world_hint = false),
        description = "Audit and diagnose dependency conflicts or broken requirement constraints (pip check) inside a target environment."
    )]
    pub async fn pip_check(
        &self,
        Parameters(params): Parameters<PipListParams>,
    ) -> Result<Json<JsonForwardResponse>, String> {
        let ctx = build_context(params.project_dir).map_err(|error| error.to_string())?;
        pip_check_response(&ctx, &params.target)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        name = "pip_precheck",
        annotations(read_only_hint = true, open_world_hint = false),
        description = "Perform a static version dependency analysis before installation, comparing an incoming requirements.txt file or remote HTTPS URL against currently installed packages to identify potential version conflicts. Auto-resolves GitHub URLs to raw content. http:// is rejected."
    )]
    pub async fn pip_precheck(
        &self,
        Parameters(params): Parameters<PipPrecheckParams>,
    ) -> Result<Json<JsonForwardResponse>, String> {
        let ctx = build_context(params.project_dir).map_err(|error| error.to_string())?;
        pip_precheck_response(&ctx, &params.target, &params.path_or_url)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        name = "pip_install",
        annotations(read_only_hint = false, destructive_hint = false),
        description = "Install pip requirements packages in a target environment from a local requirements.txt path or remote HTTPS URL. Always run pip_precheck before triggering this. http:// is rejected."
    )]
    pub async fn pip_install(
        &self,
        Parameters(params): Parameters<PipInstallParams>,
    ) -> Result<Json<JsonForwardResponse>, String> {
        let ctx = build_context(params.project_dir).map_err(|error| error.to_string())?;
        pip_install_response(&ctx, &params.target, &params.path_or_url)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        name = "pip_update",
        annotations(read_only_hint = false, destructive_hint = true),
        description = "Upgrade named packages or name==version pins, or set all=true to upgrade every outdated package. Upgrades pip first when pip is outdated. all=true fails if the outdated scan cannot run."
    )]
    pub async fn pip_update(
        &self,
        Parameters(params): Parameters<PipUpdateParams>,
    ) -> Result<Json<JsonForwardResponse>, String> {
        let ctx = build_context(params.project_dir).map_err(|error| error.to_string())?;
        let pkgs = params.packages.unwrap_or_default();
        let all = params.all.unwrap_or(false);
        pip_update_response(&ctx, &params.target, &pkgs, all)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        name = "pip_analyze_imports",
        annotations(read_only_hint = true, open_world_hint = false),
        description = "Scan Python files statically within a workspace directory using AST to identify all imported libraries, detect missing third-party dependencies, and categorize currently installed ones."
    )]
    pub async fn pip_analyze_imports(
        &self,
        Parameters(params): Parameters<PipAnalyzeParams>,
    ) -> Result<Json<JsonForwardResponse>, String> {
        let ctx = build_context(params.project_dir).map_err(|error| error.to_string())?;
        pip_analyze_imports_response(&ctx, &params.target, &params.dir_path)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        name = "get_config",
        annotations(read_only_hint = true, open_world_hint = false),
        description = "Read Enchi configuration. Omit key to return the full TOML. Use a dotted key such as windows.registry_mode to read one value."
    )]
    pub async fn get_config(
        &self,
        Parameters(params): Parameters<ConfigGetParams>,
    ) -> Result<Json<JsonForwardResponse>, String> {
        let ctx = build_context(None).map_err(|error| error.to_string())?;
        get_config_response(&ctx, params.key.as_deref())
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        name = "set_config",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true
        ),
        description = "Persist a Enchi config key. Example keys: install.bootstrap_pip, windows.registry_mode, plugins.search_path."
    )]
    pub async fn set_config(
        &self,
        Parameters(params): Parameters<ConfigSetParams>,
    ) -> Result<Json<JsonForwardResponse>, String> {
        let mut ctx = build_context(None).map_err(|error| error.to_string())?;
        set_config_response(&mut ctx, &params.key, &params.value)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        name = "self_update",
        annotations(read_only_hint = false, destructive_hint = true),
        description = "Check for or apply a Enchi self-update from GitHub Releases. yes defaults to false so agents cannot upgrade without an explicit confirmation flag. Use check=true for a read-only version comparison."
    )]
    pub async fn self_update(
        &self,
        Parameters(params): Parameters<SelfUpdateParams>,
    ) -> Result<Json<JsonForwardResponse>, String> {
        let ctx = build_context(None).map_err(|error| error.to_string())?;
        self_update_response(
            &ctx,
            params.check.unwrap_or(false),
            params.yes.unwrap_or(false),
            params.force.unwrap_or(false),
            params.github_repo,
            params.tag,
        )
        .map(Json)
        .map_err(|error| error.to_string())
    }

    #[tool(
        name = "venv_upgrade",
        annotations(read_only_hint = false, destructive_hint = true),
        description = "Migrate a managed venv onto another installed Python runtime. A target prefix resolves to the newest installed matching runtime. Snapshots pip freeze requirements, tries exact restoration first, then retries ordinary version pins as lower bounds when the target Python rejects them while preserving direct/VCS/editable lines. Requires pip check before activation; inventory, restore, compatibility, verification, and activation failures leave the source env in place. Never prompts; force=true permits replacing an existing target through a rollback path (it is restored if anything fails)."
    )]
    pub async fn venv_upgrade(
        &self,
        Parameters(params): Parameters<VenvUpgradeParams>,
    ) -> Result<Json<JsonForwardResponse>, String> {
        let ctx = build_context(params.project_dir).map_err(|error| error.to_string())?;
        venv_upgrade_response(
            &ctx,
            &params.spec,
            &params.new_runtime,
            params.force.unwrap_or(false),
            params.set_local.unwrap_or(false),
        )
        .map(Json)
        .map_err(|error| error.to_string())
    }

    #[tool(
        name = "upgrade_runtimes",
        annotations(read_only_hint = false, destructive_hint = true),
        description = "Move installed CPython runtimes to the newest patch of the same major.minor line (3.14.1 -> 3.14.10), migrate their managed venvs onto it, and re-point global/local pins. Plan-only unless apply=true; call it without apply first and show the user the plan. Never crosses a minor line (use venv_upgrade for 3.14 -> 3.15) and keeps the old runtime unless remove_old=true. Returns per-runtime status, per-venv results and progress_steps; progress is also written to <root>/logs/operation-progress.jsonl."
    )]
    pub async fn upgrade_runtimes(
        &self,
        Parameters(params): Parameters<UpgradeRuntimesParams>,
    ) -> Result<Json<JsonForwardResponse>, String> {
        let ctx = build_context(None).map_err(|error| error.to_string())?;
        tokio::task::spawn_blocking(move || {
            upgrade_runtimes_response(
                &ctx,
                params.targets.unwrap_or_default(),
                params.apply.unwrap_or(false),
                params.migrate_venvs.unwrap_or(true),
                params.repin.unwrap_or(true),
                params.remove_old.unwrap_or(false),
            )
        })
        .await
        .map_err(|error| error.to_string())?
        .map(Json)
        .map_err(|error| error.to_string())
    }

    #[tool(
        name = "update_all_packages",
        annotations(read_only_hint = false, destructive_hint = true),
        description = "Update outdated pip packages across every installed runtime and managed venv in one operation (or a filtered subset). Plan-only unless apply=true. Each environment is checked with `pip check` afterwards; if an update makes it worse the touched packages are pinned back and that environment is reported as rolled_back, without stopping the others. Returns per-environment status, the exact packages and versions, a summary and progress_steps."
    )]
    pub async fn update_all_packages(
        &self,
        Parameters(params): Parameters<UpdateAllPackagesParams>,
    ) -> Result<Json<JsonForwardResponse>, String> {
        let ctx = build_context(None).map_err(|error| error.to_string())?;
        tokio::task::spawn_blocking(move || {
            update_all_packages_response(
                &ctx,
                params.scope.as_deref(),
                params.only.unwrap_or_default(),
                params.apply.unwrap_or(false),
                params.allow_new_conflicts.unwrap_or(false),
            )
        })
        .await
        .map_err(|error| error.to_string())?
        .map(Json)
        .map_err(|error| error.to_string())
    }
}
