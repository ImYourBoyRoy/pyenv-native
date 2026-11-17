// ./crates/enchi-mcp/src/ops/maintenance.rs
//! Runtime patch upgrades and fleet-wide package updates for MCP tools.
//!
//! Both are plan-first: they only change anything when the caller passes `apply: true`, and both
//! return the same typed report the CLI prints with `--json`, plus the step log.

use anyhow::{Context, Result};

use enchi_core::{
    AppContext, BulkScope, BulkUpdateOptions, UpgradeOptions, run_upgrades, update_all_packages,
};

use crate::model::JsonForwardResponse;

/// Collects progress lines and mirrors them to `logs/operation-progress.jsonl` so a second
/// process (or the user) can watch a long operation while the tool call is still running.
struct ProgressLog {
    steps: Vec<String>,
    path: std::path::PathBuf,
}

impl ProgressLog {
    fn start(ctx: &AppContext, operation: &str) -> Self {
        let dir = ctx.root.join("logs");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("operation-progress.jsonl");
        let _ = std::fs::write(&path, "");
        let mut log = Self {
            steps: Vec::new(),
            path,
        };
        log.push(&format!("{operation}: started"));
        log
    }

    fn push(&mut self, step: &str) {
        self.steps.push(step.to_string());
        let payload = serde_json::json!({
            "event": "progress",
            "step": step,
            "ts_unix": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_secs())
                .unwrap_or(0),
        });
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            use std::io::Write as _;
            let _ = writeln!(file, "{payload}");
        }
    }
}

pub fn upgrade_runtimes_response(
    ctx: &AppContext,
    targets: Vec<String>,
    apply: bool,
    migrate_venvs: bool,
    repin: bool,
    remove_old: bool,
) -> Result<JsonForwardResponse> {
    let mut log = ProgressLog::start(ctx, "upgrade_runtimes");
    let report = run_upgrades(
        ctx,
        &UpgradeOptions {
            targets,
            dry_run: !apply,
            skip_venvs: !migrate_venvs,
            skip_repin: !repin,
            remove_old,
        },
        &mut |step| log.push(step),
    )
    .map_err(|error| anyhow::anyhow!(error.to_string()))?;

    let mut payload =
        serde_json::to_value(&report).context("failed to serialize upgrade report")?;
    payload["applied"] = serde_json::Value::Bool(apply);
    payload["has_failures"] = serde_json::Value::Bool(report.has_failures());
    payload["progress_steps"] = serde_json::json!(log.steps);
    Ok(JsonForwardResponse { payload })
}

pub fn update_all_packages_response(
    ctx: &AppContext,
    scope: Option<&str>,
    only: Vec<String>,
    apply: bool,
    allow_new_conflicts: bool,
) -> Result<JsonForwardResponse> {
    let scope = match scope.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("all") => BulkScope::All,
        Some("runtimes") => BulkScope::Runtimes,
        Some("venvs") => BulkScope::Venvs,
        Some(other) => anyhow::bail!("scope must be one of all, runtimes, venvs (got `{other}`)"),
    };
    let mut log = ProgressLog::start(ctx, "update_all_packages");
    let report = update_all_packages(
        ctx,
        &BulkUpdateOptions {
            scope,
            only,
            dry_run: !apply,
            allow_new_conflicts,
            // Local builds like torch+cu126 stay put; an agent that wants them updated says so with
            // `enchi pip update <target> <package>`.
            include_held: false,
        },
        &mut |step| log.push(step),
    )
    .map_err(|error| anyhow::anyhow!(error.to_string()))?;

    let mut payload = serde_json::to_value(&report).context("failed to serialize bulk report")?;
    payload["applied"] = serde_json::Value::Bool(apply);
    payload["has_failures"] = serde_json::Value::Bool(report.has_failures());
    payload["progress_steps"] = serde_json::json!(log.steps);
    Ok(JsonForwardResponse { payload })
}
