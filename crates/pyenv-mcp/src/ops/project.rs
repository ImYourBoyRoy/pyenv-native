// ./crates/pyenv-mcp/src/ops/project.rs
//! Project version-file and local-venv helpers used by MCP workflows.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};

use pyenv_core::{AppContext, CommandReport, cmd_global, cmd_local, resolve_install_plan};

use crate::model::{ProjectVenvResponse, VersionSelectionResponse};

use super::runtime::{
    ensure_runtime_response, resolve_interpreter_path, resolve_runtime_inventory,
};

pub fn set_local_versions_response(
    ctx: &AppContext,
    versions: &[String],
    force: bool,
) -> Result<VersionSelectionResponse> {
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
) -> Result<VersionSelectionResponse> {
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
) -> Result<ProjectVenvResponse> {
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
            ensure_runtime_response(ctx, &version, false)?
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

pub(super) fn venv_python_path(venv_path: &Path) -> PathBuf {
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

fn create_venv(interpreter_path: &Path, venv_path: &Path) -> Result<()> {
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

pub(super) fn ensure_success(report: CommandReport) -> Result<()> {
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

pub(super) fn parse_json_report<T: serde::de::DeserializeOwned>(
    report: &CommandReport,
) -> Result<T> {
    ensure_success(CommandReport {
        stdout: report.stdout.clone(),
        stderr: report.stderr.clone(),
        exit_code: report.exit_code,
    })?;

    let joined = report.stdout.join("\n");
    serde_json::from_str(&joined).with_context(|| format!("invalid JSON payload: {joined}"))
}
