// ./crates/pyenv-mcp/src/ops/pip.rs
//! MCP helpers for Pip package explorer, auditing, prechecking, installing, and updates.

use anyhow::{Context, Result};
use serde_json::Value;

use pyenv_core::{
    AppContext, cmd_pip_analyze_imports, cmd_pip_check, cmd_pip_install, cmd_pip_list,
    cmd_pip_outdated, cmd_pip_precheck_requirements, cmd_pip_update,
};

use super::project::parse_json_report;
use crate::model::JsonForwardResponse;

pub fn pip_list_response(ctx: &AppContext, target: &str) -> Result<JsonForwardResponse> {
    let report = cmd_pip_list(ctx, target, true);
    let payload = parse_json_report::<Value>(&report).context("failed to parse pip list JSON")?;
    Ok(JsonForwardResponse { payload })
}

pub fn pip_outdated_response(ctx: &AppContext, target: &str) -> Result<JsonForwardResponse> {
    let report = cmd_pip_outdated(ctx, target, true);
    let payload =
        parse_json_report::<Value>(&report).context("failed to parse pip outdated JSON")?;
    Ok(JsonForwardResponse { payload })
}

pub fn pip_check_response(ctx: &AppContext, target: &str) -> Result<JsonForwardResponse> {
    let report = cmd_pip_check(ctx, target, true);
    let payload = parse_json_report::<Value>(&report).context("failed to parse pip check JSON")?;
    Ok(JsonForwardResponse { payload })
}

pub fn pip_precheck_response(
    ctx: &AppContext,
    target: &str,
    path_or_url: &str,
) -> Result<JsonForwardResponse> {
    let report = cmd_pip_precheck_requirements(ctx, target, path_or_url);
    let payload =
        parse_json_report::<Value>(&report).context("failed to parse pip precheck JSON")?;
    Ok(JsonForwardResponse { payload })
}

pub fn pip_analyze_imports_response(
    ctx: &AppContext,
    target: &str,
    dir_path: &str,
) -> Result<JsonForwardResponse> {
    let report = cmd_pip_analyze_imports(ctx, target, dir_path);
    let payload =
        parse_json_report::<Value>(&report).context("failed to parse codebase scanner JSON")?;
    Ok(JsonForwardResponse { payload })
}

pub fn pip_install_response(
    ctx: &AppContext,
    target: &str,
    path_or_url: &str,
) -> Result<JsonForwardResponse> {
    let report = cmd_pip_install(ctx, target, path_or_url);
    let payload = serde_json::json!({
        "exit_code": report.exit_code,
        "stdout": report.stdout,
        "stderr": report.stderr,
    });
    Ok(JsonForwardResponse { payload })
}

pub fn pip_update_response(
    ctx: &AppContext,
    target: &str,
    packages: &[String],
    all: bool,
) -> Result<JsonForwardResponse> {
    let report = cmd_pip_update(ctx, target, packages, all);
    let payload = serde_json::json!({
        "exit_code": report.exit_code,
        "stdout": report.stdout,
        "stderr": report.stderr,
    });
    Ok(JsonForwardResponse { payload })
}
