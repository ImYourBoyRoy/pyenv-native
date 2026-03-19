// ./crates/pyenv-mcp/src/ops.rs
//! Structured helper operations for the pyenv-native MCP server and companion CLI.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, anyhow, bail};
use serde_json::{Value, json};

use pyenv_core::{
    AppContext, BASE_VENV_DIR_NAME, CommandReport, InstallCommandOptions, InstallOutcome,
    InstallPlan, cmd_doctor, cmd_global, cmd_install, cmd_local, install_runtime_plan,
    installed_version_dir, installed_version_names, resolve_dir, resolve_install_plan,
    resolve_selected_versions, version_file_path,
};

use crate::model::{
    BootstrapInstallCommands, EnsureRuntimeResponse, InstallInstructions, JsonForwardResponse,
    McpClientConfig, McpServerEntry, PlatformInstallCommands, ProjectVenvResponse,
    RuntimeInventory, ToolArgument, ToolSummary, ToolkitGuide, VersionCatalogGroup,
    VersionCatalogResponse, VersionSelectionResponse, WorkflowRecipe, WorkflowStep,
};

pub const DEFAULT_GITHUB_REPO: &str = "imyourboyroy/pyenv-native";
pub const DEFAULT_SERVER_NAME: &str = "pyenv-native";

pub fn build_context(project_dir: Option<PathBuf>) -> anyhow::Result<AppContext> {
    let mut ctx = AppContext::from_system().map_err(|error| anyhow!(error.to_string()))?;
    if let Some(dir) = project_dir {
        let resolved =
            resolve_dir(Some(OsString::from(dir))).map_err(|error| anyhow!(error.to_string()))?;
        ctx.dir = resolved;
    }
    Ok(ctx)
}

pub fn build_client_config(
    command_path: &Path,
    pyenv_root: &Path,
    server_name: &str,
) -> McpClientConfig {
    let mut env = BTreeMap::new();
    env.insert("PYENV_ROOT".to_string(), pyenv_root.display().to_string());

    let mut servers = BTreeMap::new();
    servers.insert(
        server_name.to_string(),
        McpServerEntry {
            command: command_path.display().to_string(),
            args: Vec::new(),
            env,
        },
    );

    McpClientConfig {
        mcp_servers: servers,
    }
}

fn render_windows_install_root(install_root: Option<&Path>) -> String {
    install_root
        .map(|path| path.display().to_string().replace('/', "\\"))
        .unwrap_or_else(|| "$HOME\\.pyenv".to_string())
}

fn render_posix_install_root(install_root: Option<&Path>) -> String {
    install_root
        .map(|path| path.display().to_string().replace('\\', "/"))
        .unwrap_or_else(|| "~/.pyenv".to_string())
}

pub fn build_install_instructions(
    github_repo: &str,
    install_root: Option<&Path>,
    server_name: &str,
    mcp_command: &Path,
    pyenv_root: &Path,
) -> InstallInstructions {
    let install_root_windows = render_windows_install_root(install_root);
    let install_root_posix = render_posix_install_root(install_root);

    InstallInstructions {
        github_repo: github_repo.to_string(),
        default_install_roots: PlatformInstallCommands {
            windows_powershell: "$HOME\\.pyenv".to_string(),
            linux_or_macos: "~/.pyenv".to_string(),
        },
        latest_release: PlatformInstallCommands {
            windows_powershell: format!(
                "$installer = Join-Path $env:TEMP 'pyenv-native-install.ps1'; Invoke-WebRequest https://raw.githubusercontent.com/{github_repo}/main/install.ps1 -OutFile $installer; & $installer -InstallRoot \"{install_root_windows}\""
            ),
            linux_or_macos: format!(
                "curl -fsSL https://raw.githubusercontent.com/{github_repo}/main/install.sh | sh -s -- --install-root {install_root_posix}"
            ),
        },
        pinned_release_example: PlatformInstallCommands {
            windows_powershell: format!(
                "$tag = 'vX.Y.Z'; $installer = Join-Path $env:TEMP 'pyenv-native-install.ps1'; Invoke-WebRequest \"https://raw.githubusercontent.com/{github_repo}/$tag/install.ps1\" -OutFile $installer; & $installer -Tag $tag -InstallRoot \"{install_root_windows}\" -Force"
            ),
            linux_or_macos: format!(
                "tag='vX.Y.Z'; curl -fsSL \"https://raw.githubusercontent.com/{github_repo}/${{tag}}/install.sh\" | sh -s -- --tag \"$tag\" --install-root {install_root_posix}"
            ),
        },
        pip_bootstrap: BootstrapInstallCommands {
            pipx: format!(
                "pipx install pyenv-native && pyenv-native install --github-repo {github_repo} --install-root {install_root_posix}"
            ),
            pip: format!(
                "python -m pip install pyenv-native && pyenv-native install --github-repo {github_repo} --install-root {install_root_posix}"
            ),
        },
        uninstall: PlatformInstallCommands {
            windows_powershell: format!(
                "$uninstaller = Join-Path $env:TEMP 'pyenv-native-uninstall.ps1'; Invoke-WebRequest https://raw.githubusercontent.com/{github_repo}/main/uninstall.ps1 -OutFile $uninstaller; & $uninstaller -InstallRoot \"{install_root_windows}\" -RemoveRoot"
            ),
            linux_or_macos: format!(
                "curl -fsSL https://raw.githubusercontent.com/{github_repo}/main/uninstall.sh | sh -s -- --install-root {install_root_posix} --remove-root"
            ),
        },
        mcp_client_config: build_client_config(mcp_command, pyenv_root, server_name),
    }
}

pub fn build_toolkit_guide(
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
            crate::model::GuideStep {
                step: "Call get_toolkit_guide first when the model does not already understand pyenv-native".to_string(),
                reason: "It returns install commands, MCP client config, tool summaries, and recommended workflows in one JSON blob.".to_string(),
            },
            crate::model::GuideStep {
                step: "Call resolve_project_environment before making changes".to_string(),
                reason: "It tells the agent what Python version is active, where it came from, and whether it is missing.".to_string(),
            },
            crate::model::GuideStep {
                step: "Call ensure_runtime when the project version is missing or when you need a specific runtime".to_string(),
                reason: "This is the idempotent runtime installer for managed Python versions.".to_string(),
            },
            crate::model::GuideStep {
                step: "Call ensure_project_venv for project work".to_string(),
                reason: "It creates or reuses a predictable project-local .venv and returns concrete python/pip paths.".to_string(),
            },
            crate::model::GuideStep {
                step: "Use doctor when anything looks odd".to_string(),
                reason: "It returns machine-readable diagnostics about path issues, roots, shims, and host readiness.".to_string(),
            },
        ],
        install: build_install_instructions(github_repo, install_root, server_name, mcp_command, pyenv_root),
        tool_summaries: vec![
            ToolSummary {
                tool_name: "get_toolkit_guide".to_string(),
                use_when: "The model needs a single structured orientation blob before doing anything else.".to_string(),
                returns: "Install instructions, MCP client config, common workflows, and tool summaries.".to_string(),
                side_effects: "Read-only.".to_string(),
                arguments: vec![
                    ToolArgument {
                        name: "github_repo".to_string(),
                        required: false,
                        data_type: "string".to_string(),
                        description: "Override the GitHub owner/repo used in the generated install commands.".to_string(),
                    },
                    ToolArgument {
                        name: "install_root".to_string(),
                        required: false,
                        data_type: "path".to_string(),
                        description: "Override the install root that should appear in example commands.".to_string(),
                    },
                    ToolArgument {
                        name: "server_name".to_string(),
                        required: false,
                        data_type: "string".to_string(),
                        description: "Override the MCP server name used in the returned client config snippet.".to_string(),
                    },
                ],
                example_input: Some(json!({})),
            },
            ToolSummary {
                tool_name: "get_install_instructions".to_string(),
                use_when: "You need only install and uninstall commands plus the MCP config snippet without the rest of the guide.".to_string(),
                returns: "Platform-specific install commands for pyenv-native itself, uninstall commands, and an MCP client config block.".to_string(),
                side_effects: "Read-only.".to_string(),
                arguments: vec![
                    ToolArgument {
                        name: "github_repo".to_string(),
                        required: false,
                        data_type: "string".to_string(),
                        description: "Override the GitHub owner/repo used in the generated install commands.".to_string(),
                    },
                    ToolArgument {
                        name: "install_root".to_string(),
                        required: false,
                        data_type: "path".to_string(),
                        description: "Override the install root that should appear in example commands.".to_string(),
                    },
                ],
                example_input: Some(json!({ "install_root": "~/.pyenv" })),
            },
            ToolSummary {
                tool_name: "resolve_project_environment".to_string(),
                use_when: "You need to know what Python version a folder should use before taking action.".to_string(),
                returns: "Selected versions, missing versions, version-file origin, installed versions, and the best-effort interpreter path.".to_string(),
                side_effects: "Read-only.".to_string(),
                arguments: vec![
                    ToolArgument {
                        name: "project_dir".to_string(),
                        required: false,
                        data_type: "path".to_string(),
                        description: "Project directory to inspect. If omitted, the current working directory is used.".to_string(),
                    },
                ],
                example_input: Some(json!({ "project_dir": "/workspace/app" })),
            },
            ToolSummary {
                tool_name: "list_available_versions".to_string(),
                use_when: "You need installable runtime choices or the broader known catalog before choosing a version.".to_string(),
                returns: "Grouped runtime families with optional provider, architecture, and source metadata.".to_string(),
                side_effects: "Read-only.".to_string(),
                arguments: vec![
                    ToolArgument {
                        name: "family".to_string(),
                        required: false,
                        data_type: "string".to_string(),
                        description: "Optional family filter such as cpython or pypy.".to_string(),
                    },
                    ToolArgument {
                        name: "pattern".to_string(),
                        required: false,
                        data_type: "string".to_string(),
                        description: "Optional prefix or pattern filter such as 3.13 or pypy3.11.".to_string(),
                    },
                    ToolArgument {
                        name: "known".to_string(),
                        required: false,
                        data_type: "boolean".to_string(),
                        description: "When true, return the broader known catalog instead of only provider-backed installable versions.".to_string(),
                    },
                ],
                example_input: Some(json!({ "family": "cpython", "pattern": "3.13" })),
            },
            ToolSummary {
                tool_name: "ensure_runtime".to_string(),
                use_when: "A managed Python runtime must exist before project work can continue.".to_string(),
                returns: "Resolved version, provider, install directory, interpreter path, whether the runtime already existed, and structured progress steps describing what happened.".to_string(),
                side_effects: "Downloads and installs a runtime if it is not already present or if force=true is used.".to_string(),
                arguments: vec![
                    ToolArgument {
                        name: "version".to_string(),
                        required: true,
                        data_type: "string".to_string(),
                        description: "Requested runtime version or prefix, such as 3.12, 3.13.12, or pypy3.11.".to_string(),
                    },
                    ToolArgument {
                        name: "force".to_string(),
                        required: false,
                        data_type: "boolean".to_string(),
                        description: "Reinstall or replace an already-installed runtime at the same path.".to_string(),
                    },
                ],
                example_input: Some(json!({ "version": "3.12" })),
            },
            ToolSummary {
                tool_name: "set_local_version".to_string(),
                use_when: "You want a project to resolve to one or more specific managed runtimes.".to_string(),
                returns: "The written .python-version path and the versions that were stored there.".to_string(),
                side_effects: "Writes or overwrites a local .python-version file.".to_string(),
                arguments: vec![
                    ToolArgument {
                        name: "versions".to_string(),
                        required: true,
                        data_type: "array<string>".to_string(),
                        description: "One or more runtime identifiers to store in the project's .python-version file.".to_string(),
                    },
                    ToolArgument {
                        name: "project_dir".to_string(),
                        required: false,
                        data_type: "path".to_string(),
                        description: "Project directory where the .python-version file should be written.".to_string(),
                    },
                    ToolArgument {
                        name: "force".to_string(),
                        required: false,
                        data_type: "boolean".to_string(),
                        description: "Overwrite a conflicting local version file when necessary.".to_string(),
                    },
                ],
                example_input: Some(json!({ "project_dir": "/workspace/app", "versions": ["3.12.10"] })),
            },
            ToolSummary {
                tool_name: "set_global_version".to_string(),
                use_when: "You want to change the default managed runtime for new shells or projects without local overrides.".to_string(),
                returns: "The global version file path and the versions now stored there.".to_string(),
                side_effects: "Writes or clears the global version file under PYENV_ROOT.".to_string(),
                arguments: vec![
                    ToolArgument {
                        name: "versions".to_string(),
                        required: true,
                        data_type: "array<string>".to_string(),
                        description: "One or more runtime identifiers to store globally.".to_string(),
                    },
                    ToolArgument {
                        name: "unset".to_string(),
                        required: false,
                        data_type: "boolean".to_string(),
                        description: "When true, clear the global version file instead of writing versions.".to_string(),
                    },
                ],
                example_input: Some(json!({ "versions": ["3.13.12"] })),
            },
            ToolSummary {
                tool_name: "ensure_project_venv".to_string(),
                use_when: "A project-local virtual environment should be created or reused in a predictable location.".to_string(),
                returns: "The concrete venv path plus python and pip paths that can be used immediately.".to_string(),
                side_effects: "May install a missing runtime, create a venv, and optionally write a local .python-version file.".to_string(),
                arguments: vec![
                    ToolArgument {
                        name: "project_dir".to_string(),
                        required: false,
                        data_type: "path".to_string(),
                        description: "Project directory where the venv should live. Defaults to the current directory.".to_string(),
                    },
                    ToolArgument {
                        name: "version".to_string(),
                        required: false,
                        data_type: "string".to_string(),
                        description: "Explicit runtime to use. If omitted, resolve from the project selection rules.".to_string(),
                    },
                    ToolArgument {
                        name: "venv_path".to_string(),
                        required: false,
                        data_type: "path".to_string(),
                        description: "Explicit venv path. Defaults to <project>/.venv.".to_string(),
                    },
                    ToolArgument {
                        name: "install_if_missing".to_string(),
                        required: false,
                        data_type: "boolean".to_string(),
                        description: "Install the selected runtime first when it is missing.".to_string(),
                    },
                    ToolArgument {
                        name: "set_local_version".to_string(),
                        required: false,
                        data_type: "boolean".to_string(),
                        description: "Also write the chosen runtime into the project's .python-version file.".to_string(),
                    },
                ],
                example_input: Some(json!({
                    "project_dir": "/workspace/app",
                    "version": "3.12",
                    "install_if_missing": true,
                    "set_local_version": true
                })),
            },
            ToolSummary {
                tool_name: "doctor".to_string(),
                use_when: "Something about the install, shell, shims, or host toolchain looks wrong.".to_string(),
                returns: "The same structured doctor payload available from pyenv doctor --json.".to_string(),
                side_effects: "Read-only.".to_string(),
                arguments: vec![
                    ToolArgument {
                        name: "project_dir".to_string(),
                        required: false,
                        data_type: "path".to_string(),
                        description: "Optional project directory for context-sensitive diagnostics.".to_string(),
                    },
                ],
                example_input: Some(json!({})),
            },
        ],
        common_workflows: vec![
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
                        example_input: Some(json!({
                            "project_dir": "/workspace/app",
                            "install_if_missing": true,
                            "set_local_version": true
                        })),
                    },
                ],
            },
        ],
        notes: vec![
            "The MCP server is agent-first: prefer structured tools over shelling out to pyenv manually.".to_string(),
            "Project virtual environments default to <project>/.venv so IDEs and agents can find them easily.".to_string(),
            "The guide includes direct install commands for pyenv-native itself, but the MCP server assumes pyenv-native is already installed when the server is running.".to_string(),
        ],
    }
}

pub fn resolve_runtime_inventory(ctx: &AppContext) -> RuntimeInventory {
    let selected = resolve_selected_versions(ctx, false);
    let primary_version = selected.versions.first().cloned();
    let primary_interpreter = primary_version
        .as_deref()
        .and_then(|version| resolve_interpreter_path(ctx, version).ok());

    RuntimeInventory {
        root: ctx.root.clone(),
        installed_versions: installed_version_names(ctx).unwrap_or_default(),
        selected_versions: selected.versions,
        missing_versions: selected.missing,
        version_origin: selected.origin.to_string(),
        version_file_path: version_file_path(ctx, None),
        primary_version,
        primary_interpreter,
        shims_dir: ctx.shims_dir(),
        versions_dir: ctx.versions_dir(),
    }
}

pub fn list_available_versions_response(
    ctx: &AppContext,
    family: Option<String>,
    pattern: Option<String>,
    known: bool,
) -> anyhow::Result<VersionCatalogResponse> {
    let report = cmd_install(
        ctx,
        &InstallCommandOptions {
            list: true,
            force: false,
            dry_run: false,
            json: true,
            known,
            family: family.clone(),
            versions: pattern.clone().into_iter().collect(),
        },
    );
    let groups: Vec<VersionCatalogGroup> =
        parse_json_report(&report).context("failed to parse install list JSON")?;
    Ok(VersionCatalogResponse {
        provider_backed: !known,
        family_filter: family,
        pattern_filter: pattern,
        groups,
    })
}

pub fn doctor_response(ctx: &AppContext) -> anyhow::Result<JsonForwardResponse> {
    let report = cmd_doctor(ctx, true);
    let payload = parse_json_report::<Value>(&report).context("failed to parse doctor JSON")?;
    Ok(JsonForwardResponse { payload })
}

pub fn ensure_runtime_response(
    ctx: &AppContext,
    requested_version: &str,
    force: bool,
) -> anyhow::Result<EnsureRuntimeResponse> {
    let plan =
        resolve_install_plan(ctx, requested_version).map_err(|error| anyhow!(error.to_string()))?;
    let already_installed = plan.python_executable.is_file();
    if already_installed && !force {
        return Ok(build_ensure_runtime_response(
            requested_version,
            &plan,
            None,
            true,
        ));
    }

    let outcome =
        install_runtime_plan(ctx, &plan, force).map_err(|error| anyhow!(error.to_string()))?;
    Ok(build_ensure_runtime_response(
        requested_version,
        &outcome.plan,
        Some(&outcome),
        false,
    ))
}

fn build_ensure_runtime_response(
    requested_version: &str,
    plan: &InstallPlan,
    outcome: Option<&InstallOutcome>,
    already_installed: bool,
) -> EnsureRuntimeResponse {
    let receipt_path = outcome.map(|value| value.receipt_path.clone()).or_else(|| {
        let candidate = plan.install_dir.join(".pyenv-install.json");
        candidate.is_file().then_some(candidate)
    });

    EnsureRuntimeResponse {
        requested_version: requested_version.to_string(),
        resolved_version: plan.resolved_version.clone(),
        already_installed,
        provider: plan.provider.clone(),
        family: plan.family.clone(),
        architecture: plan.architecture.clone(),
        install_dir: plan.install_dir.clone(),
        python_executable: plan.python_executable.clone(),
        receipt_path,
        pip_bootstrapped: outcome
            .map(|value| value.pip_bootstrapped)
            .unwrap_or(plan.bootstrap_pip),
        base_venv_created: outcome
            .map(|value| value.base_venv_created)
            .unwrap_or_else(|| {
                plan.base_venv_path
                    .as_ref()
                    .is_some_and(|path| path.exists())
            }),
        progress_steps: outcome
            .map(|value| value.progress_steps.clone())
            .unwrap_or_else(|| {
                vec![format!(
                    "Runtime {} is already installed at {}",
                    plan.resolved_version,
                    plan.install_dir.display()
                )]
            }),
    }
}

pub fn set_local_versions_response(
    ctx: &AppContext,
    versions: &[String],
    force: bool,
) -> anyhow::Result<VersionSelectionResponse> {
    let report = cmd_local(ctx, versions, false, force);
    ensure_success(report)?;
    Ok(VersionSelectionResponse {
        scope: "local".to_string(),
        version_file_path: ctx.dir.join(".python-version"),
        versions: versions.to_vec(),
    })
}

pub fn set_global_versions_response(
    ctx: &AppContext,
    versions: &[String],
    unset: bool,
) -> anyhow::Result<VersionSelectionResponse> {
    let report = cmd_global(ctx, versions, unset);
    ensure_success(report)?;
    Ok(VersionSelectionResponse {
        scope: "global".to_string(),
        version_file_path: ctx.root.join("version"),
        versions: if unset { Vec::new() } else { versions.to_vec() },
    })
}

pub fn ensure_project_venv_response(
    ctx: &AppContext,
    requested_version: Option<String>,
    explicit_venv_path: Option<PathBuf>,
    install_if_missing: bool,
    set_local_version: bool,
) -> anyhow::Result<ProjectVenvResponse> {
    let project_dir = ctx.dir.clone();
    let mut runtime_installed = true;
    let resolved_version = if let Some(version) = requested_version.clone() {
        let ensured = if install_if_missing {
            ensure_runtime_response(ctx, &version, false)?
        } else {
            let plan =
                resolve_install_plan(ctx, &version).map_err(|error| anyhow!(error.to_string()))?;
            if !plan.python_executable.is_file() {
                bail!(
                    "requested runtime '{}' is not installed",
                    plan.resolved_version
                );
            }
            build_ensure_runtime_response(&version, &plan, None, true)
        };
        runtime_installed = ensured.already_installed || ensured.receipt_path.is_some();
        ensured.resolved_version
    } else {
        let inventory = resolve_runtime_inventory(ctx);
        if let Some(version) = inventory.primary_version {
            if inventory.missing_versions.is_empty() {
                version
            } else if install_if_missing {
                let missing = inventory
                    .missing_versions
                    .first()
                    .cloned()
                    .unwrap_or(version.clone());
                let ensured = ensure_runtime_response(ctx, &missing, false)?;
                runtime_installed = ensured.already_installed || ensured.receipt_path.is_some();
                ensured.resolved_version
            } else {
                bail!(
                    "project runtime is missing; call ensure_runtime first or pass install_if_missing=true"
                )
            }
        } else if install_if_missing && !inventory.missing_versions.is_empty() {
            let missing = inventory.missing_versions[0].clone();
            let ensured = ensure_runtime_response(ctx, &missing, false)?;
            runtime_installed = ensured.already_installed || ensured.receipt_path.is_some();
            ensured.resolved_version
        } else {
            bail!("project does not currently resolve to a managed runtime")
        }
    };

    let interpreter_path = resolve_interpreter_path(ctx, &resolved_version).with_context(|| {
        format!("failed to locate interpreter for runtime '{resolved_version}'")
    })?;

    let venv_path = explicit_venv_path.unwrap_or_else(|| project_dir.join(".venv"));
    let venv_python = venv_python_path(&venv_path);
    let created = if venv_python.is_file() {
        false
    } else {
        create_venv(&interpreter_path, &venv_path)?;
        true
    };

    let local_version_written = if set_local_version {
        let versions = vec![resolved_version.clone()];
        let report = cmd_local(ctx, &versions, false, true);
        ensure_success(report)?;
        true
    } else {
        false
    };

    let pip_path = venv_pip_path(&venv_path)
        .ok_or_else(|| anyhow!("failed to locate pip inside {}", venv_path.display()))?;

    Ok(ProjectVenvResponse {
        project_dir,
        requested_version,
        resolved_version,
        runtime_installed,
        local_version_written,
        venv_path,
        python_path: venv_python,
        pip_path,
        created,
    })
}

pub fn resolve_interpreter_path(ctx: &AppContext, version: &str) -> anyhow::Result<PathBuf> {
    if version == "system" {
        bail!(
            "system interpreter selection is not supported by the MCP helper; install or select a managed runtime instead"
        )
    }

    let version_dir = installed_version_dir(ctx, version);
    let mut candidates = Vec::new();

    if ctx.config.venv.auto_use_base_venv {
        let base_venv = version_dir.join(BASE_VENV_DIR_NAME);
        candidates.extend(python_candidates_for_prefix(&base_venv));
    }

    candidates.extend(python_candidates_for_prefix(&version_dir));

    for candidate in candidates {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    bail!("no interpreter was found under {}", version_dir.display())
}

fn python_candidates_for_prefix(prefix: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if cfg!(windows) {
        candidates.push(prefix.join("python.exe"));
        candidates.push(prefix.join("Scripts").join("python.exe"));
        candidates.push(prefix.join("Scripts").join("pypy3.exe"));
    } else {
        candidates.push(prefix.join("bin").join("python"));
        candidates.push(prefix.join("bin").join("python3"));
        candidates.push(prefix.join("bin").join("pypy3"));
        candidates.push(prefix.join("python"));
    }
    candidates
}

fn venv_python_path(venv_path: &Path) -> PathBuf {
    if cfg!(windows) {
        venv_path.join("Scripts").join("python.exe")
    } else {
        venv_path.join("bin").join("python")
    }
}

fn venv_pip_path(venv_path: &Path) -> Option<PathBuf> {
    let candidates = if cfg!(windows) {
        vec![
            venv_path.join("Scripts").join("pip.exe"),
            venv_path.join("Scripts").join("pip3.exe"),
        ]
    } else {
        vec![
            venv_path.join("bin").join("pip"),
            venv_path.join("bin").join("pip3"),
        ]
    };

    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn create_venv(interpreter_path: &Path, venv_path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = venv_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let status = Command::new(interpreter_path)
        .arg("-m")
        .arg("venv")
        .arg(venv_path)
        .status()
        .with_context(|| {
            format!(
                "failed to run '{}' -m venv {}",
                interpreter_path.display(),
                venv_path.display()
            )
        })?;

    if !status.success() {
        bail!(
            "'{} -m venv {}' failed with exit code {:?}",
            interpreter_path.display(),
            venv_path.display(),
            status.code()
        )
    }

    Ok(())
}

fn ensure_success(report: CommandReport) -> anyhow::Result<()> {
    if report.exit_code == 0 {
        return Ok(());
    }

    let mut messages = Vec::new();
    if !report.stderr.is_empty() {
        messages.push(report.stderr.join("\n"));
    }
    if !report.stdout.is_empty() {
        messages.push(report.stdout.join("\n"));
    }

    if messages.is_empty() {
        bail!("command failed without diagnostic output")
    }

    bail!(messages.join("\n"))
}

fn parse_json_report<T: serde::de::DeserializeOwned>(report: &CommandReport) -> anyhow::Result<T> {
    ensure_success(CommandReport {
        stdout: report.stdout.clone(),
        stderr: report.stderr.clone(),
        exit_code: report.exit_code,
    })?;

    let joined = report.stdout.join("\n");
    serde_json::from_str(&joined).with_context(|| format!("invalid JSON payload: {joined}"))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tempfile::TempDir;

    use super::{build_client_config, build_install_instructions, venv_python_path};

    #[test]
    fn client_config_contains_pyenv_root() {
        let config = build_client_config(
            Path::new("/tmp/pyenv-mcp"),
            Path::new("/tmp/.pyenv"),
            "pyenv-native",
        );
        let server = config
            .mcp_servers
            .get("pyenv-native")
            .expect("server entry");
        assert_eq!(server.command, "/tmp/pyenv-mcp");
        assert_eq!(
            server.env.get("PYENV_ROOT").expect("PYENV_ROOT"),
            "/tmp/.pyenv"
        );
    }

    #[test]
    fn install_instructions_reference_repo() {
        let instructions = build_install_instructions(
            "imyourboyroy/pyenv-native",
            Some(Path::new("/tmp/.pyenv")),
            "pyenv-native",
            Path::new("/tmp/pyenv-mcp"),
            Path::new("/tmp/.pyenv"),
        );
        assert!(
            instructions
                .latest_release
                .linux_or_macos
                .contains("raw.githubusercontent.com/imyourboyroy/pyenv-native")
        );
        assert!(
            instructions
                .pip_bootstrap
                .pipx
                .contains("pipx install pyenv-native")
        );
    }

    #[test]
    fn venv_python_path_uses_platform_layout() {
        let temp = TempDir::new().expect("tempdir");
        let path = venv_python_path(temp.path());
        if cfg!(windows) {
            assert!(path.ends_with("Scripts\\python.exe"));
        } else {
            assert!(path.ends_with("bin/python"));
        }
    }
}
