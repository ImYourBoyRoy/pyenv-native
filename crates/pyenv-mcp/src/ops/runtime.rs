// ./crates/pyenv-mcp/src/ops/runtime.rs
//! Runtime inventory, install, doctor, and interpreter-resolution helpers for MCP tools.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;

use pyenv_core::{
    AppContext, BASE_VENV_DIR_NAME, InstallCommandOptions, InstallOutcome, InstallPlan, cmd_doctor,
    cmd_install, install_runtime_plan, installed_version_dir, installed_version_names,
    resolve_install_plan, resolve_selected_versions, version_file_path,
};

use crate::model::{
    EnsureRuntimeResponse, JsonForwardResponse, RuntimeInventory, VersionCatalogGroup,
    VersionCatalogResponse,
};

use super::project::parse_json_report;

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
) -> Result<VersionCatalogResponse> {
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

pub fn doctor_response(ctx: &AppContext) -> Result<JsonForwardResponse> {
    let report = cmd_doctor(ctx, true);
    let payload = parse_json_report::<Value>(&report).context("failed to parse doctor JSON")?;
    Ok(JsonForwardResponse { payload })
}

pub fn ensure_runtime_response(
    ctx: &AppContext,
    requested_version: &str,
    force: bool,
) -> Result<EnsureRuntimeResponse> {
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

pub fn resolve_interpreter_path(ctx: &AppContext, version: &str) -> Result<PathBuf> {
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
