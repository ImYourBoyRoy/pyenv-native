// ./crates/pyenv-core/src/pip/bulk.rs
//! Update the outdated packages of many environments in one pass: every installed runtime and
//! every managed venv, or a filtered subset.
//!
//! Each target is updated on its own and gated by `pip check`: if the update leaves the
//! environment with more broken requirements than it started with, the packages we touched are
//! pinned back and the target is reported as `rolled_back`. One bad environment never stops the
//! rest.

use std::path::Path;

use serde::Serialize;

use crate::catalog::installed_version_names;
use crate::command::CommandReport;
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::venv::list_managed_venvs;

use super::operations::{parse_pip_json_array, pip_subcommand, resolve_interpreter_path};
use super::types::OutdatedPackage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BulkScope {
    All,
    Runtimes,
    Venvs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkUpdateOptions {
    pub scope: BulkScope,
    /// Limit to these runtimes / venv specs or names. Empty means every target in scope.
    pub only: Vec<String>,
    pub dry_run: bool,
    /// Keep going when `pip check` gets worse instead of rolling the target back.
    pub allow_new_conflicts: bool,
}

impl Default for BulkUpdateOptions {
    fn default() -> Self {
        Self {
            scope: BulkScope::All,
            only: Vec::new(),
            dry_run: false,
            allow_new_conflicts: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Runtime,
    Venv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetStatus {
    UpToDate,
    /// Dry run: these packages would be updated.
    Planned,
    Updated,
    /// The update made `pip check` worse, so the touched packages were pinned back.
    RolledBack,
    Failed,
    /// No usable interpreter or pip (for example the `system` Python).
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TargetResult {
    pub spec: String,
    pub kind: TargetKind,
    pub status: TargetStatus,
    pub outdated: Vec<OutdatedPackage>,
    pub updated: Vec<String>,
    pub conflicts_before: usize,
    pub conflicts_after: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BulkSummary {
    pub targets: usize,
    pub up_to_date: usize,
    pub updated: usize,
    pub packages_updated: usize,
    pub rolled_back: usize,
    pub failed: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BulkUpdateReport {
    pub dry_run: bool,
    pub targets: Vec<TargetResult>,
    pub summary: BulkSummary,
}

impl BulkUpdateReport {
    pub fn has_failures(&self) -> bool {
        self.summary.failed > 0 || self.summary.rolled_back > 0
    }
}

struct Target {
    spec: String,
    kind: TargetKind,
}

fn list_targets(ctx: &AppContext, options: &BulkUpdateOptions) -> Result<Vec<Target>, PyenvError> {
    let mut targets = Vec::new();
    if options.scope != BulkScope::Venvs {
        for name in installed_version_names(ctx)? {
            if name != "system" && !name.contains('/') {
                targets.push(Target {
                    spec: name,
                    kind: TargetKind::Runtime,
                });
            }
        }
    }
    if options.scope != BulkScope::Runtimes {
        for venv in list_managed_venvs(ctx)? {
            targets.push(Target {
                spec: venv.spec,
                kind: TargetKind::Venv,
            });
        }
    }
    if !options.only.is_empty() {
        targets.retain(|target| {
            options.only.iter().any(|wanted| {
                let wanted = wanted.trim();
                target.spec == wanted
                    || target.spec.rsplit('/').next() == Some(wanted)
                    || target.spec.starts_with(&format!("{wanted}/"))
            })
        });
    }
    Ok(targets)
}

fn outdated(py: &Path) -> Result<Vec<OutdatedPackage>, String> {
    let output = pip_subcommand(py)
        .args([
            "-m",
            "pip",
            "--disable-pip-version-check",
            "list",
            "--outdated",
            "--format=json",
        ])
        .output()
        .map_err(|error| format!("could not run pip: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(last_line(&stderr)
            .unwrap_or("pip list --outdated failed")
            .to_string());
    }
    parse_pip_json_array(&output.stdout).map_err(|error| format!("unreadable pip output: {error}"))
}

/// Number of broken requirements `pip check` reports (0 when clean).
fn conflicts(py: &Path) -> usize {
    match pip_subcommand(py).args(["-m", "pip", "check"]).output() {
        Ok(output) if output.status.success() => 0,
        Ok(output) => String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count()
            .max(1),
        Err(_) => 0,
    }
}

fn pip_install(py: &Path, args: &[String]) -> Result<(), String> {
    let output = pip_subcommand(py)
        .args(["-m", "pip", "install"])
        .args(args)
        .output()
        .map_err(|error| format!("could not run pip: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(last_line(&stderr)
            .unwrap_or("pip install failed")
            .to_string())
    }
}

fn last_line(text: &str) -> Option<&str> {
    text.lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
}

fn update_target(
    ctx: &AppContext,
    target: &Target,
    options: &BulkUpdateOptions,
    on_progress: &mut dyn FnMut(&str),
) -> TargetResult {
    let mut result = TargetResult {
        spec: target.spec.clone(),
        kind: target.kind,
        status: TargetStatus::Skipped,
        outdated: Vec::new(),
        updated: Vec::new(),
        conflicts_before: 0,
        conflicts_after: 0,
        message: String::new(),
    };

    let py = match resolve_interpreter_path(ctx, &target.spec) {
        Ok(py) => py,
        Err(error) => {
            result.message = error.trim_start_matches("pyenv: ").to_string();
            return result;
        }
    };

    on_progress(&format!("{}: checking for updates", target.spec));
    result.outdated = match outdated(&py) {
        Ok(list) => list,
        Err(error) => {
            // A runtime without pip is not a failure worth alarming about.
            let no_pip = error.contains("No module named pip");
            result.status = if no_pip {
                TargetStatus::Skipped
            } else {
                TargetStatus::Failed
            };
            result.message = if no_pip {
                "pip is not installed".to_string()
            } else {
                error
            };
            return result;
        }
    };

    if result.outdated.is_empty() {
        result.status = TargetStatus::UpToDate;
        result.message = "all packages are current".to_string();
        return result;
    }
    if options.dry_run {
        result.status = TargetStatus::Planned;
        result.message = format!("{} package(s) would be updated", result.outdated.len());
        return result;
    }

    result.conflicts_before = conflicts(&py);

    // pip first, so the resolver doing the rest is the newest one.
    let is_pip = |pkg: &&OutdatedPackage| pkg.name.eq_ignore_ascii_case("pip");
    if result.outdated.iter().any(|pkg| is_pip(&pkg)) {
        on_progress(&format!("{}: updating pip", target.spec));
        match pip_install(&py, &["-U".into(), "pip".into()]) {
            Ok(()) => result.updated.push("pip".to_string()),
            Err(error) => {
                result.status = TargetStatus::Failed;
                result.message = format!("pip self-update failed: {error}");
                return result;
            }
        }
    }

    let rest: Vec<&OutdatedPackage> = result.outdated.iter().filter(|pkg| !is_pip(pkg)).collect();
    if !rest.is_empty() {
        on_progress(&format!(
            "{}: updating {} package(s)",
            target.spec,
            rest.len()
        ));
        let mut args = vec!["-U".to_string()];
        args.extend(rest.iter().map(|pkg| pkg.name.clone()));
        if let Err(error) = pip_install(&py, &args) {
            result.status = TargetStatus::Failed;
            result.message = format!("update failed: {error}");
            return result;
        }
        result
            .updated
            .extend(rest.iter().map(|pkg| pkg.name.clone()));
    }

    result.conflicts_after = conflicts(&py);
    if result.conflicts_after > result.conflicts_before && !options.allow_new_conflicts {
        on_progress(&format!(
            "{}: pip check got worse, rolling back",
            target.spec
        ));
        let pins: Vec<String> = result
            .outdated
            .iter()
            .map(|pkg| format!("{}=={}", pkg.name, pkg.version))
            .collect();
        let rolled = pip_install(&py, &pins);
        result.conflicts_after = conflicts(&py);
        result.status = TargetStatus::RolledBack;
        result.updated.clear();
        result.message = match rolled {
            Ok(()) => {
                "the update introduced dependency conflicts; previous versions restored".into()
            }
            Err(error) => format!(
                "the update introduced dependency conflicts and the rollback failed: {error}"
            ),
        };
        return result;
    }

    result.status = TargetStatus::Updated;
    result.message = format!("{} package(s) updated", result.updated.len());
    result
}

/// Update every selected environment. `on_progress` gets one line per step.
pub fn update_all_packages(
    ctx: &AppContext,
    options: &BulkUpdateOptions,
    on_progress: &mut dyn FnMut(&str),
) -> Result<BulkUpdateReport, PyenvError> {
    let targets = list_targets(ctx, options)?;
    let mut results = Vec::with_capacity(targets.len());
    for (index, target) in targets.iter().enumerate() {
        on_progress(&format!(
            "[{}/{}] {}",
            index + 1,
            targets.len(),
            target.spec
        ));
        results.push(update_target(ctx, target, options, on_progress));
    }

    let count = |status: TargetStatus| results.iter().filter(|r| r.status == status).count();
    let summary = BulkSummary {
        targets: results.len(),
        up_to_date: count(TargetStatus::UpToDate),
        updated: count(TargetStatus::Updated),
        packages_updated: results.iter().map(|r| r.updated.len()).sum(),
        rolled_back: count(TargetStatus::RolledBack),
        failed: count(TargetStatus::Failed),
        skipped: count(TargetStatus::Skipped),
    };
    Ok(BulkUpdateReport {
        dry_run: options.dry_run,
        targets: results,
        summary,
    })
}

/// `pyenv pip update-all` for the CLI/GUI/MCP. Never reads stdin.
pub fn cmd_pip_update_all(
    ctx: &AppContext,
    options: &BulkUpdateOptions,
    json: bool,
    on_progress: &mut dyn FnMut(&str),
) -> CommandReport {
    let report = match update_all_packages(ctx, options, on_progress) {
        Ok(report) => report,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };
    let exit_code = i32::from(report.has_failures());
    if json {
        return match serde_json::to_string_pretty(&report) {
            Ok(payload) => CommandReport {
                stdout: payload.lines().map(ToOwned::to_owned).collect(),
                stderr: Vec::new(),
                exit_code,
            },
            Err(error) => CommandReport::failure(vec![format!("pyenv: {error}")], 1),
        };
    }
    CommandReport {
        stdout: render_text(&report),
        stderr: Vec::new(),
        exit_code,
    }
}

pub(super) fn render_text(report: &BulkUpdateReport) -> Vec<String> {
    if report.targets.is_empty() {
        return vec!["No runtimes or managed venvs matched.".to_string()];
    }
    let width = report
        .targets
        .iter()
        .map(|t| t.spec.len())
        .max()
        .unwrap_or(0);
    let mut out = Vec::new();
    for target in &report.targets {
        let label = match target.status {
            TargetStatus::UpToDate => "up to date",
            TargetStatus::Planned => "outdated",
            TargetStatus::Updated => "updated",
            TargetStatus::RolledBack => "rolled back",
            TargetStatus::Failed => "failed",
            TargetStatus::Skipped => "skipped",
        };
        let detail = if target.message.is_empty() {
            String::new()
        } else {
            format!(" - {}", target.message)
        };
        out.push(format!("  {:<width$}  {label}{detail}", target.spec));
        if matches!(
            target.status,
            TargetStatus::Planned | TargetStatus::Updated | TargetStatus::RolledBack
        ) {
            for pkg in &target.outdated {
                out.push(format!(
                    "      {} {} -> {}",
                    pkg.name, pkg.version, pkg.latest_version
                ));
            }
        }
    }
    let s = &report.summary;
    out.push(String::new());
    out.push(if report.dry_run {
        format!(
            "{} of {} environment(s) have updates. Run without --dry-run to apply.",
            s.targets - s.up_to_date - s.skipped - s.failed,
            s.targets
        )
    } else {
        format!(
            "{} package(s) updated across {} environment(s); {} up to date, {} rolled back, {} failed, {} skipped.",
            s.packages_updated, s.updated, s.up_to_date, s.rolled_back, s.failed, s.skipped
        )
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(spec: &str, status: TargetStatus) -> TargetResult {
        TargetResult {
            spec: spec.into(),
            kind: TargetKind::Venv,
            status,
            outdated: vec![OutdatedPackage {
                name: "requests".into(),
                version: "2.31.0".into(),
                latest_version: "2.32.3".into(),
            }],
            updated: vec![],
            conflicts_before: 0,
            conflicts_after: 0,
            message: String::new(),
        }
    }

    fn report(dry_run: bool, targets: Vec<TargetResult>) -> BulkUpdateReport {
        let count = |s: TargetStatus| targets.iter().filter(|r| r.status == s).count();
        BulkUpdateReport {
            dry_run,
            summary: BulkSummary {
                targets: targets.len(),
                up_to_date: count(TargetStatus::UpToDate),
                updated: count(TargetStatus::Updated),
                packages_updated: 0,
                rolled_back: count(TargetStatus::RolledBack),
                failed: count(TargetStatus::Failed),
                skipped: count(TargetStatus::Skipped),
            },
            targets,
        }
    }

    #[test]
    fn rollbacks_and_failures_make_the_run_unsuccessful() {
        assert!(
            !report(false, vec![result("3.14.1/envs/a", TargetStatus::Updated)]).has_failures()
        );
        assert!(
            report(
                false,
                vec![result("3.14.1/envs/a", TargetStatus::RolledBack)]
            )
            .has_failures()
        );
        assert!(report(false, vec![result("3.14.1/envs/a", TargetStatus::Failed)]).has_failures());
        assert!(!report(false, vec![result("system", TargetStatus::Skipped)]).has_failures());
    }

    #[test]
    fn dry_run_text_lists_packages_and_the_apply_hint() {
        let text = render_text(&report(
            true,
            vec![result("3.14.1/envs/a", TargetStatus::Planned)],
        ))
        .join("\n");
        assert!(text.contains("requests 2.31.0 -> 2.32.3"));
        assert!(text.contains("1 of 1 environment(s) have updates"));
    }

    #[test]
    fn empty_selection_says_so() {
        assert_eq!(
            render_text(&report(false, vec![])),
            vec!["No runtimes or managed venvs matched.".to_string()]
        );
    }

    #[test]
    fn json_shape_is_stable() {
        let json = serde_json::to_value(report(
            true,
            vec![result("3.14.1/envs/a", TargetStatus::Planned)],
        ))
        .unwrap();
        assert_eq!(json["dry_run"], true);
        assert_eq!(json["targets"][0]["status"], "planned");
        assert_eq!(json["targets"][0]["kind"], "venv");
        assert_eq!(json["summary"]["targets"], 1);
    }
}
