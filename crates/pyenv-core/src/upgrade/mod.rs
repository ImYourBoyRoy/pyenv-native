// ./crates/pyenv-core/src/upgrade/mod.rs
//! Patch-level runtime upgrades: move an installed CPython `3.14.1` to the newest `3.14.x`,
//! migrate the managed venvs that sat on it, and re-point global/local pins.
//!
//! Minor-line moves (3.14 -> 3.15) stay explicit through `pyenv venv upgrade`; this module only
//! ever stays inside one `major.minor` line, which is why it can be run unattended.

use std::cmp::Ordering;
use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::catalog::{compare_version_names, installed_version_names};
use crate::command::CommandReport;
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::install::{install_runtime_plan_with_progress, resolve_install_plan};
use crate::manage::cmd_uninstall;
use crate::venv::{cmd_venv_upgrade_noninteractive, list_managed_venvs};
use crate::version::version_file_path;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpgradeOptions {
    /// Installed runtimes or `major.minor` lines to consider. Empty means every installed CPython.
    pub targets: Vec<String>,
    /// Report what would change without installing or touching anything.
    pub dry_run: bool,
    /// Skip migrating the managed venvs that sit on the old patch.
    pub skip_venvs: bool,
    /// Leave global/local version files pointing at the old patch.
    pub skip_repin: bool,
    /// Uninstall the old patch once every venv has moved off it.
    pub remove_old: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpgradeStatus {
    /// Already on the newest patch of its line.
    UpToDate,
    /// A newer patch exists (plan only).
    Available,
    /// Installed and migrated.
    Upgraded,
    /// Installed, but something after the install failed; see `message` and `venvs`.
    Partial,
    Failed,
    /// Not a runtime this command handles (PyPy, free-threaded, pre-release, `system`).
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VenvMigration {
    pub spec: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeUpgrade {
    pub installed: String,
    pub latest: Option<String>,
    pub status: UpgradeStatus,
    pub message: String,
    pub venvs: Vec<VenvMigration>,
    pub repinned: Vec<String>,
    pub removed_old: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpgradeReport {
    pub dry_run: bool,
    pub items: Vec<RuntimeUpgrade>,
}

impl UpgradeReport {
    pub fn has_failures(&self) -> bool {
        self.items
            .iter()
            .any(|item| matches!(item.status, UpgradeStatus::Failed | UpgradeStatus::Partial))
    }
}

/// `3.14.1` -> `Some("3.14")`. Anything that is not a plain release triple is `None`.
fn plain_patch_line(name: &str) -> Option<String> {
    let mut parts = name.split('.');
    let (major, minor, patch) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    let numeric = |part: &str| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit());
    (numeric(major) && numeric(minor) && numeric(patch)).then(|| format!("{major}.{minor}"))
}

fn skip_reason(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    if name == "system" || name.contains('/') {
        Some("not a managed runtime")
    } else if lower.contains("pypy") {
        Some("PyPy releases are versioned independently; install the one you want explicitly")
    } else if plain_patch_line(name).is_some() {
        None
    } else if lower.ends_with('t') {
        Some("free-threaded builds are not auto-upgraded")
    } else {
        Some("pre-release or non-standard name; install the final release explicitly")
    }
}

fn target_matches(targets: &[String], installed: &str) -> bool {
    targets.is_empty()
        || targets.iter().any(|target| {
            let target = target.trim();
            installed == target
                || installed
                    .strip_prefix(target)
                    .is_some_and(|rest| rest.starts_with('.'))
        })
}

/// Look up the newest patch for every selected runtime. No side effects beyond the catalog fetch.
pub fn plan_upgrades(
    ctx: &AppContext,
    targets: &[String],
) -> Result<Vec<RuntimeUpgrade>, PyenvError> {
    let mut items = Vec::new();
    for installed in installed_version_names(ctx)? {
        if !target_matches(targets, &installed) {
            continue;
        }
        let mut item = RuntimeUpgrade {
            installed: installed.clone(),
            latest: None,
            status: UpgradeStatus::Skipped,
            message: String::new(),
            venvs: Vec::new(),
            repinned: Vec::new(),
            removed_old: false,
        };
        if let Some(reason) = skip_reason(&installed) {
            item.message = reason.to_string();
            items.push(item);
            continue;
        }
        let line = plain_patch_line(&installed).expect("checked by skip_reason");
        match resolve_install_plan(ctx, &line) {
            Ok(plan) => {
                let newer =
                    compare_version_names(&plan.resolved_version, &installed) == Ordering::Greater;
                item.latest = Some(plan.resolved_version.clone());
                if newer {
                    item.status = UpgradeStatus::Available;
                    item.message = format!("{} is available", plan.resolved_version);
                } else {
                    item.status = UpgradeStatus::UpToDate;
                    item.message = "already the newest patch".to_string();
                }
            }
            Err(error) => {
                item.status = UpgradeStatus::Failed;
                item.message = format!("could not look up the newest {line}.x: {error}");
            }
        }
        items.push(item);
    }
    Ok(items)
}

/// Plan, then (unless `dry_run`) install, migrate venvs, re-pin and optionally remove the old patch.
/// `on_progress` receives one human-readable line per step.
pub fn run_upgrades(
    ctx: &AppContext,
    options: &UpgradeOptions,
    on_progress: &mut dyn FnMut(&str),
) -> Result<UpgradeReport, PyenvError> {
    let mut items = plan_upgrades(ctx, &options.targets)?;
    if options.dry_run {
        return Ok(UpgradeReport {
            dry_run: true,
            items,
        });
    }

    for item in items
        .iter_mut()
        .filter(|item| item.status == UpgradeStatus::Available)
    {
        upgrade_one(ctx, options, item, on_progress);
    }
    Ok(UpgradeReport {
        dry_run: false,
        items,
    })
}

fn upgrade_one(
    ctx: &AppContext,
    options: &UpgradeOptions,
    item: &mut RuntimeUpgrade,
    on_progress: &mut dyn FnMut(&str),
) {
    let old = item.installed.clone();
    let new = item.latest.clone().expect("Available items carry a target");
    on_progress(&format!("{old} -> {new}: resolving"));

    let line = plain_patch_line(&old).expect("planned runtimes are plain releases");
    let plan = match resolve_install_plan(ctx, &line) {
        Ok(plan) => plan,
        Err(error) => {
            item.status = UpgradeStatus::Failed;
            item.message = error.to_string();
            return;
        }
    };

    let already_installed = installed_version_names(ctx)
        .map(|names| names.iter().any(|name| name == &new))
        .unwrap_or(false);
    if !already_installed {
        on_progress(&format!("{old} -> {new}: installing {new}"));
        let mut forward = |line: &str| on_progress(&format!("{new}: {line}"));
        if let Err(error) =
            install_runtime_plan_with_progress(ctx, &plan, false, Some(&mut forward))
        {
            item.status = UpgradeStatus::Failed;
            item.message = format!("install of {new} failed: {error}");
            return;
        }
    }

    let mut problems = 0usize;
    if !options.skip_venvs {
        let venvs = list_managed_venvs(ctx).unwrap_or_default();
        for venv in venvs.into_iter().filter(|venv| venv.base_version == old) {
            on_progress(&format!("{old} -> {new}: migrating venv {}", venv.spec));
            let report = cmd_venv_upgrade_noninteractive(ctx, &venv.spec, &new, false, false);
            let ok = report.exit_code == 0;
            if !ok {
                problems += 1;
            }
            let message = if ok {
                format!("moved to {new}")
            } else {
                report
                    .stderr
                    .iter()
                    .chain(report.stdout.iter())
                    .rev()
                    .find(|line| !line.trim().is_empty())
                    .cloned()
                    .unwrap_or_else(|| "venv upgrade failed".to_string())
            };
            item.venvs.push(VenvMigration {
                spec: venv.spec,
                ok,
                message,
            });
        }
    }

    if !options.skip_repin {
        let mut files = vec![ctx.root.join("version")];
        let local = version_file_path(ctx, None);
        if !files.contains(&local) {
            files.push(local);
        }
        for file in files {
            if replace_pin_in_file(&file, &old, &new) {
                item.repinned.push(file.display().to_string());
            }
        }
    }

    if options.remove_old {
        let still_used = list_managed_venvs(ctx)
            .map(|venvs| venvs.iter().any(|venv| venv.base_version == old))
            .unwrap_or(true);
        if problems == 0 && !still_used {
            // `force` only skips the prompt here: the check above guarantees no venv depends on it.
            let removal = cmd_uninstall(ctx, std::slice::from_ref(&old), true);
            item.removed_old = removal.exit_code == 0;
            if !item.removed_old {
                problems += 1;
            }
        } else {
            problems += 1;
        }
    }

    if problems == 0 {
        item.status = UpgradeStatus::Upgraded;
        item.message = format!("upgraded to {new}");
    } else {
        item.status = UpgradeStatus::Partial;
        item.message =
            format!("{new} is installed, but {problems} follow-up step(s) did not complete");
    }
    on_progress(&format!("{old} -> {new}: {}", item.message));
}

/// Replace exact `old` lines in a version file. Returns whether anything changed.
fn replace_pin_in_file(path: &Path, old: &str, new: &str) -> bool {
    let Ok(contents) = fs::read_to_string(path) else {
        return false;
    };
    let mut changed = false;
    let lines: Vec<String> = contents
        .lines()
        .map(|line| {
            if line.trim() == old {
                changed = true;
                new.to_string()
            } else {
                line.to_string()
            }
        })
        .collect();
    if !changed {
        return false;
    }
    let mut next = lines.join("\n");
    if contents.ends_with('\n') {
        next.push('\n');
    }
    fs::write(path, next).is_ok()
}

/// `pyenv upgrade` entry point shared by the CLI, GUI and MCP server (no stdin, no prompts).
pub fn cmd_upgrade(
    ctx: &AppContext,
    options: &UpgradeOptions,
    json: bool,
    on_progress: &mut dyn FnMut(&str),
) -> CommandReport {
    let report = match run_upgrades(ctx, options, on_progress) {
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

fn render_text(report: &UpgradeReport) -> Vec<String> {
    if report.items.is_empty() {
        return vec![
            "No managed runtimes installed. Install one with `pyenv install <version>`.".into(),
        ];
    }
    let width = report
        .items
        .iter()
        .map(|item| item.installed.len())
        .max()
        .unwrap_or(0);
    let mut out = Vec::new();
    let mut available = 0usize;
    for item in &report.items {
        let target = item.latest.as_deref().unwrap_or("-");
        let line = match item.status {
            UpgradeStatus::UpToDate => format!("  {:<width$}  up to date", item.installed),
            UpgradeStatus::Available => {
                available += 1;
                format!("  {:<width$}  -> {target}  available", item.installed)
            }
            UpgradeStatus::Upgraded => {
                format!("  {:<width$}  -> {target}  upgraded", item.installed)
            }
            UpgradeStatus::Partial => format!(
                "  {:<width$}  -> {target}  partial: {}",
                item.installed, item.message
            ),
            UpgradeStatus::Failed => {
                format!("  {:<width$}  failed: {}", item.installed, item.message)
            }
            UpgradeStatus::Skipped => {
                format!("  {:<width$}  skipped ({})", item.installed, item.message)
            }
        };
        out.push(line);
        for venv in &item.venvs {
            out.push(format!(
                "    {} venv {}: {}",
                if venv.ok { "ok  " } else { "FAIL" },
                venv.spec,
                venv.message
            ));
        }
        for file in &item.repinned {
            out.push(format!("    re-pinned {file}"));
        }
        if item.removed_old {
            out.push(format!("    removed {}", item.installed));
        }
    }
    if report.dry_run && available > 0 {
        out.push(String::new());
        out.push(format!(
            "{available} runtime(s) can be upgraded. Run `pyenv upgrade` to apply."
        ));
    }
    out
}

#[cfg(test)]
mod tests;
