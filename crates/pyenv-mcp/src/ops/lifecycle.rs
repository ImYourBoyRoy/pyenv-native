// ./crates/pyenv-mcp/src/ops/lifecycle.rs
//! Config, self-update, and managed-venv upgrade helpers for MCP tools.

use anyhow::{Result, anyhow};

use pyenv_core::{
    AppContext, SelfUpdateOptions, cmd_config_get, cmd_config_set, cmd_config_show,
    cmd_self_update, cmd_venv_upgrade,
};

use crate::model::JsonForwardResponse;

use super::project::ensure_success;

pub fn get_config_response(ctx: &AppContext, key: Option<&str>) -> Result<JsonForwardResponse> {
    let report = match key {
        Some(key) if !key.trim().is_empty() => cmd_config_get(ctx, key),
        _ => cmd_config_show(ctx),
    };
    command_report_response(report)
}

pub fn set_config_response(
    ctx: &mut AppContext,
    key: &str,
    value: &str,
) -> Result<JsonForwardResponse> {
    let report = cmd_config_set(ctx, key, value);
    command_report_response(report)
}

pub fn self_update_response(
    ctx: &AppContext,
    check: bool,
    yes: bool,
    force: bool,
    github_repo: Option<String>,
    tag: Option<String>,
) -> Result<JsonForwardResponse> {
    let report = cmd_self_update(
        ctx,
        &SelfUpdateOptions {
            check,
            yes,
            force,
            github_repo,
            tag,
            restart_gui: false,
        },
    );
    command_report_response(report)
}

pub fn venv_upgrade_response(
    ctx: &AppContext,
    spec: &str,
    new_runtime: &str,
    force: bool,
    set_local: bool,
) -> Result<JsonForwardResponse> {
    let report = cmd_venv_upgrade(ctx, spec, new_runtime, force, set_local);
    command_report_response(report)
}

fn command_report_response(report: pyenv_core::CommandReport) -> Result<JsonForwardResponse> {
    ensure_success(report.clone()).map_err(|error| anyhow!(error.to_string()))?;
    Ok(JsonForwardResponse {
        payload: serde_json::json!({
            "stdout": report.stdout,
            "stderr": report.stderr,
            "exit_code": report.exit_code,
        }),
    })
}
