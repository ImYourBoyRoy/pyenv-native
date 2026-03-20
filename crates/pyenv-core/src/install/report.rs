// ./crates/pyenv-core/src/install/report.rs
//! Human-readable and JSON install reporting helpers.

use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::command::CommandReport;
use crate::error::PyenvError;

use super::types::{InstallOutcome, InstallPlan};

pub(super) fn pip_wrapper_names(package_version: &str) -> Vec<String> {
    let mut names = vec!["pip".to_string()];
    let parts = package_version
        .split('.')
        .take(2)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if let Some(major) = parts.first() {
        names.push(format!("pip{major}"));
    }
    if parts.len() == 2 {
        names.push(format!("pip{}.{}", parts[0], parts[1]));
    }
    names
}

pub(super) fn render_plan_lines(plans: &[InstallPlan]) -> Vec<String> {
    let mut lines = Vec::new();
    for (index, plan) in plans.iter().enumerate() {
        if index > 0 {
            lines.push(String::new());
        }
        lines.push(format!("Requested: {}", plan.requested_version));
        lines.push(format!("Resolved: {}", plan.resolved_version));
        lines.push(format!("Provider: {}", plan.provider));
        lines.push(format!("Runtime: {}", plan.runtime_version));
        lines.push(format!(
            "Package: {} {}",
            plan.package_name, plan.package_version
        ));
        lines.push(format!("Architecture: {}", plan.architecture));
        lines.push(format!("Download: {}", plan.download_url));
        lines.push(format!("Cache: {}", plan.cache_path.display()));
        lines.push(format!("Install dir: {}", plan.install_dir.display()));
        lines.push(format!("Bootstrap pip: {}", plan.bootstrap_pip));
        lines.push(format!("Create base venv: {}", plan.create_base_venv));
    }
    lines
}

pub(super) fn progress_step<S: Into<String>, T: Into<String>>(phase: S, detail: T) -> String {
    format!("{}: {}", phase.into(), detail.into())
}

#[cfg(test)]
pub(super) fn render_outcome_lines(outcomes: &[InstallOutcome]) -> Vec<String> {
    render_outcome_lines_with_progress(outcomes, true)
}

pub(super) fn render_outcome_summary_lines(outcomes: &[InstallOutcome]) -> Vec<String> {
    render_outcome_lines_with_progress(outcomes, false)
}

fn render_outcome_lines_with_progress(
    outcomes: &[InstallOutcome],
    include_progress: bool,
) -> Vec<String> {
    let mut lines = Vec::new();
    for (index, outcome) in outcomes.iter().enumerate() {
        if index > 0 {
            lines.push(String::new());
        }
        if include_progress {
            lines.push("Progress:".to_string());
            lines.extend(
                outcome
                    .progress_steps
                    .iter()
                    .map(|step| format!("  - {step}")),
            );
        }
        lines.push(format!(
            "Installed {} -> {}",
            outcome.plan.requested_version, outcome.plan.resolved_version
        ));
        lines.push(format!("Location: {}", outcome.plan.install_dir.display()));
        lines.push(format!(
            "Python: {}",
            outcome.plan.python_executable.display()
        ));
        lines.push(format!("Runtime: {}", outcome.plan.runtime_version));
        lines.push(format!("Pip bootstrapped: {}", outcome.pip_bootstrapped));
        lines.push(format!("Base venv created: {}", outcome.base_venv_created));
        lines.push(format!("Receipt: {}", outcome.receipt_path.display()));
    }
    lines
}

pub(super) fn render_install_error_lines(error: &PyenvError, requested: &str) -> Vec<String> {
    if matches!(
        requested.trim().to_ascii_lowercase().as_str(),
        "-help" | "--help" | "/?"
    ) {
        return vec![
            "pyenv: `install` help was requested.".to_string(),
            "hint: run `pyenv install --help` or `pyenv help install`".to_string(),
        ];
    }

    match error {
        PyenvError::UnsupportedInstallTarget(_) => vec![
            "pyenv: no native install provider is configured for this platform/version."
                .to_string(),
            format!("hint: run `pyenv install --list {requested}`"),
            "hint: run `pyenv doctor` to inspect platform prerequisites and shell/path health"
                .to_string(),
        ],
        PyenvError::UnknownVersion(_) => vec![
            error.to_string(),
            format!("hint: run `pyenv install --list {requested}`"),
        ],
        _ => vec![error.to_string()],
    }
}

pub(super) fn render_json_lines<T: Serialize>(value: &T) -> Vec<String> {
    serde_json::to_string_pretty(value)
        .map(|json| json.lines().map(ToOwned::to_owned).collect())
        .unwrap_or_else(|error| vec![format!("pyenv: failed to serialize JSON output: {error}")])
}

pub(super) fn render_json_report<T: Serialize>(value: &T) -> CommandReport {
    match serde_json::to_string_pretty(value) {
        Ok(json) => CommandReport::success(json.lines().map(ToOwned::to_owned).collect()),
        Err(error) => CommandReport::failure(
            vec![format!("pyenv: failed to serialize JSON output: {error}")],
            1,
        ),
    }
}

pub(super) fn sanitize_for_fs(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

pub(super) fn unique_suffix() -> String {
    format!(
        "{}-{}",
        process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    )
}

pub(super) fn io_error(error: std::io::Error) -> PyenvError {
    PyenvError::Io(format!("pyenv: {error}"))
}

pub(super) fn format_command_output_suffix(stdout: &[u8], stderr: &[u8]) -> String {
    let mut details = Vec::new();

    let stdout_text = String::from_utf8_lossy(stdout).trim().to_string();
    if !stdout_text.is_empty() {
        details.push(format!(
            "; stdout: {}",
            summarize_command_text(&stdout_text)
        ));
    }

    let stderr_text = String::from_utf8_lossy(stderr).trim().to_string();
    if !stderr_text.is_empty() {
        details.push(format!(
            "; stderr: {}",
            summarize_command_text(&stderr_text)
        ));
    }

    details.concat()
}

pub(super) fn summarize_command_text(text: &str) -> String {
    let compact = text.lines().map(str::trim).collect::<Vec<_>>().join(" ");
    if compact.len() <= 220 {
        compact
    } else {
        format!("{}...", &compact[..220])
    }
}
