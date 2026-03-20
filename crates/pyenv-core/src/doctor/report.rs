// ./crates/pyenv-core/src/doctor/report.rs
//! Doctor command rendering for human-readable and JSON diagnostics output.

use std::env;

use crate::catalog::installed_version_names;
use crate::command::CommandReport;
use crate::context::AppContext;

use super::checks::collect_checks;
use super::types::DoctorReport;

pub fn cmd_doctor(ctx: &AppContext, json: bool) -> CommandReport {
    let checks = collect_checks(ctx);
    let installed_versions = installed_version_names(ctx)
        .map(|items| items.len())
        .unwrap_or(0);
    let report = DoctorReport {
        root: ctx.root.display().to_string(),
        platform: env::consts::OS.to_string(),
        installed_versions,
        checks,
    };

    if json {
        return match serde_json::to_string_pretty(&report) {
            Ok(payload) => CommandReport::success(payload.lines().map(ToOwned::to_owned).collect()),
            Err(error) => CommandReport::failure(
                vec![format!("pyenv: failed to serialize doctor output: {error}")],
                1,
            ),
        };
    }

    let mut stdout = vec![
        format!("pyenv root: {}", report.root),
        format!("platform: {}", report.platform),
        format!("installed versions: {}", report.installed_versions),
        String::new(),
    ];
    stdout.extend(report.checks.into_iter().map(|check| {
        format!(
            "[{}] {}: {}",
            check.status.label(),
            check.name,
            check.detail
        )
    }));
    CommandReport::success(stdout)
}
