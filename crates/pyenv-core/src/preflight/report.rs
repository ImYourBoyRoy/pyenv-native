// ./crates/pyenv-core/src/preflight/report.rs
//! Human and JSON rendering for `pyenv preflight` / environment intelligence.

use crate::command::CommandReport;
use crate::context::AppContext;

use super::intel::build_platform_intelligence;
use super::types::PlatformIntelligence;

pub fn cmd_preflight(ctx: &AppContext, json: bool) -> CommandReport {
    let intel = build_platform_intelligence(ctx);
    render_intelligence(intel, json)
}

pub fn cmd_environment(ctx: &AppContext, json: bool) -> CommandReport {
    // Alias surface for agents/users looking for an "environment" dashboard.
    cmd_preflight(ctx, json)
}

fn render_intelligence(intel: PlatformIntelligence, json: bool) -> CommandReport {
    if json {
        return match serde_json::to_string_pretty(&intel) {
            Ok(payload) => CommandReport::success(payload.lines().map(ToOwned::to_owned).collect()),
            Err(error) => CommandReport::failure(
                vec![format!(
                    "pyenv: failed to serialize preflight output: {error}"
                )],
                1,
            ),
        };
    }

    let mut stdout = vec![
        format!("pyenv platform intelligence [{}]", intel.verdict.label()),
        intel.summary.clone(),
        String::new(),
        "Host facts:".to_string(),
    ];
    for fact in &intel.facts {
        stdout.push(format!("  {}: {}", fact.label, fact.value));
    }

    stdout.push(String::new());
    stdout.push("Preflight checks:".to_string());
    for check in &intel.checks {
        stdout.push(format!(
            "  [{}] {}: {}",
            check.status.label(),
            check.name,
            check.detail
        ));
    }

    if !intel.blocking_issues.is_empty() {
        stdout.push(String::new());
        stdout.push("Blocking issues:".to_string());
        for issue in &intel.blocking_issues {
            stdout.push(format!("  - {issue}"));
        }
    }

    if !intel.warnings.is_empty() {
        stdout.push(String::new());
        stdout.push("Warnings:".to_string());
        for warning in &intel.warnings {
            stdout.push(format!("  - {warning}"));
        }
    }

    if !intel.recommended_actions.is_empty() {
        stdout.push(String::new());
        stdout.push("Recommended actions:".to_string());
        for action in &intel.recommended_actions {
            let auto = if action.automated {
                "automated"
            } else {
                "manual"
            };
            stdout.push(format!("  - [{}] {}", auto, action.description));
            if let Some(hint) = &action.command_hint {
                stdout.push(format!("      hint: {hint}"));
            }
        }
        stdout.push(String::new());
        stdout.push("Tip: run `pyenv doctor --fix` to apply safe automated repairs.".to_string());
    }

    let exit_code = if intel.ready_to_install { 0 } else { 1 };
    if exit_code == 0 {
        CommandReport::success(stdout)
    } else {
        CommandReport {
            stdout,
            stderr: vec![
                "pyenv: preflight reports the host is not ready for a successful install"
                    .to_string(),
            ],
            exit_code,
        }
    }
}
