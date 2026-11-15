// ./crates/enchi-mcp/src/ops/manage.rs
//! Inventory and cleanup tools: list managed venvs, resolve an executable, and the plan-first
//! removal of runtimes and venvs.
//!
//! The destructive tools only change anything when the caller passes `apply: true`; without it they
//! return the same plan `--dry-run` prints on the command line. None of them can prompt, because
//! the stdio server's stdin/stdout are the JSON-RPC channel.

use anyhow::{Result, anyhow};

use enchi_core::{
    AppContext, CommandReport, cmd_uninstall, cmd_venv_delete, cmd_venv_rename, cmd_which,
    list_managed_venvs, plan_uninstall, plan_venv_delete, plan_venv_rename,
};

use crate::model::JsonForwardResponse;

pub fn list_venvs_response(ctx: &AppContext) -> Result<JsonForwardResponse> {
    let venvs = list_managed_venvs(ctx).map_err(|error| anyhow!(error.to_string()))?;
    Ok(JsonForwardResponse {
        payload: serde_json::json!({ "count": venvs.len(), "venvs": venvs }),
    })
}

pub fn which_response(
    ctx: &AppContext,
    command: &str,
    no_system: bool,
) -> Result<JsonForwardResponse> {
    let report = cmd_which(ctx, command, no_system, true);
    let report = ensure_ok(report)?;
    Ok(JsonForwardResponse {
        payload: serde_json::json!({
            "command": command,
            "path": report.stdout.first().cloned(),
        }),
    })
}

pub fn uninstall_runtime_response(
    ctx: &AppContext,
    versions: Vec<String>,
    apply: bool,
    remove_dependent_venvs: bool,
) -> Result<JsonForwardResponse> {
    let plan = ensure_ok(plan_uninstall(ctx, &versions))?;
    if !apply {
        return Ok(plan_response(plan));
    }

    if !remove_dependent_venvs {
        let dependents: Vec<String> = list_managed_venvs(ctx)
            .map_err(|error| anyhow!(error.to_string()))?
            .into_iter()
            .filter(|venv| versions.contains(&venv.base_version))
            .map(|venv| venv.spec)
            .collect();
        if !dependents.is_empty() {
            return Err(anyhow!(
                "managed venvs depend on this runtime: {}. Move them with venv_upgrade or pass remove_dependent_venvs=true to delete them too.",
                dependents.join(", ")
            ));
        }
    }

    // `force` here only suppresses the stdin confirmation; the dependents check above has
    // already decided whether venvs may go.
    applied_response(cmd_uninstall(ctx, &versions, true))
}

pub fn venv_delete_response(
    ctx: &AppContext,
    spec: &str,
    apply: bool,
) -> Result<JsonForwardResponse> {
    let plan = ensure_ok(plan_venv_delete(ctx, spec))?;
    if !apply {
        return Ok(plan_response(plan));
    }
    applied_response(cmd_venv_delete(ctx, spec, true))
}

pub fn venv_rename_response(
    ctx: &AppContext,
    spec: &str,
    new_name: &str,
    apply: bool,
) -> Result<JsonForwardResponse> {
    let plan = ensure_ok(plan_venv_rename(ctx, spec, new_name))?;
    if !apply {
        return Ok(plan_response(plan));
    }
    applied_response(cmd_venv_rename(ctx, spec, new_name))
}

fn ensure_ok(report: CommandReport) -> Result<CommandReport> {
    if report.exit_code == 0 {
        Ok(report)
    } else {
        let mut lines = report.stderr;
        lines.extend(report.stdout);
        Err(anyhow!(lines.join("\n")))
    }
}

fn plan_response(plan: CommandReport) -> JsonForwardResponse {
    JsonForwardResponse {
        payload: serde_json::json!({
            "applied": false,
            "plan": plan.stdout,
            "hint": "Call again with apply=true to make this change.",
        }),
    }
}

fn applied_response(report: CommandReport) -> Result<JsonForwardResponse> {
    let report = ensure_ok(report)?;
    Ok(JsonForwardResponse {
        payload: serde_json::json!({
            "applied": true,
            "stdout": report.stdout,
            "stderr": report.stderr,
        }),
    })
}
