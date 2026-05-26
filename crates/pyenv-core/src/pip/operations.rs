// ./crates/pyenv-core/src/pip/operations.rs
//! Core business logic for running Pip operations inside target environments.
//!
//! Handles subprocess execution for listing, outdated scanning, conflict check
//! validation, and remote requirements.txt fetching and verification.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::process::PyenvCommandExt;
use crate::runtime::find_command_in_prefix;
use crate::venv::{resolve_installed_runtime_version, resolve_managed_venv};
use crate::version::installed_version_dir;

use super::types::{DependencyConflict, OutdatedPackage, PipPackage};

/// Resolves the absolute path to the Python interpreter for a given target spec (runtime version or managed venv name).
pub fn resolve_interpreter_path(ctx: &AppContext, target: &str) -> Result<PathBuf, String> {
    let clean_target = target.strip_prefix("venv:").unwrap_or(target);

    // Try resolving as a managed venv first
    if let Ok(info) = resolve_managed_venv(ctx, clean_target) {
        if let Some(py_path) = info.python_path.filter(|p| p.exists()) {
            return Ok(py_path);
        }
        return Err(format!(
            "pyenv: interpreter for managed venv '{}' is missing from disk.",
            target
        ));
    }

    // Try resolving as a base Python runtime version
    match resolve_installed_runtime_version(ctx, target) {
        Ok(ver) => {
            let prefix = installed_version_dir(ctx, &ver);
            for command in ["python", "python3", "pypy3"] {
                if let Some(path) = find_command_in_prefix(&prefix, command, None) {
                    return Ok(path);
                }
            }
            Err(format!(
                "pyenv: failed to locate a Python interpreter under prefix '{}'",
                prefix.display()
            ))
        }
        Err(e) => Err(format!(
            "pyenv: target '{}' is neither an installed runtime nor a managed venv. Error: {}",
            target, e
        )),
    }
}

/// Executes `python -m pip list --format=json` to fetch installed packages.
pub fn cmd_pip_list(ctx: &AppContext, target: &str, json: bool) -> CommandReport {
    let py_path = match resolve_interpreter_path(ctx, target) {
        Ok(p) => p,
        Err(e) => return CommandReport::failure(vec![e], 1),
    };

    let output = match Command::new(&py_path)
        .headless()
        .args(["-m", "pip", "list", "--format=json"])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            return CommandReport::failure(
                vec![format!("pyenv: failed to run pip list process: {e}")],
                1,
            );
        }
    };

    if !output.status.success() {
        return CommandReport::failure(
            vec![
                format!("pyenv: pip list process exited with failure."),
                String::from_utf8_lossy(&output.stderr).to_string(),
            ],
            1,
        );
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let packages: Vec<PipPackage> = match serde_json::from_str(&stdout_str) {
        Ok(pkgs) => pkgs,
        Err(e) => {
            return CommandReport::failure(
                vec![format!(
                    "pyenv: failed to parse pip list output as JSON: {e}"
                )],
                1,
            );
        }
    };

    if json {
        match serde_json::to_string_pretty(&packages) {
            Ok(payload) => CommandReport::success(payload.lines().map(ToOwned::to_owned).collect()),
            Err(e) => CommandReport::failure(
                vec![format!("pyenv: failed to serialize packages list: {e}")],
                1,
            ),
        }
    } else {
        let mut lines = vec![format!("Installed packages for '{target}':")];
        for pkg in packages {
            lines.push(format!("  - {} ({})", pkg.name, pkg.version));
        }
        CommandReport::success(lines)
    }
}

/// Executes `python -m pip list --outdated --format=json` to check for upgrades.
pub fn cmd_pip_outdated(ctx: &AppContext, target: &str, json: bool) -> CommandReport {
    let py_path = match resolve_interpreter_path(ctx, target) {
        Ok(p) => p,
        Err(e) => return CommandReport::failure(vec![e], 1),
    };

    let output = match Command::new(&py_path)
        .headless()
        .args(["-m", "pip", "list", "--outdated", "--format=json"])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            return CommandReport::failure(
                vec![format!("pyenv: failed to run pip outdated process: {e}")],
                1,
            );
        }
    };

    if !output.status.success() {
        return CommandReport::failure(
            vec![
                format!("pyenv: pip outdated process exited with failure."),
                String::from_utf8_lossy(&output.stderr).to_string(),
            ],
            1,
        );
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let packages: Vec<OutdatedPackage> = match serde_json::from_str(&stdout_str) {
        Ok(pkgs) => pkgs,
        Err(e) => {
            return CommandReport::failure(
                vec![format!(
                    "pyenv: failed to parse pip outdated output as JSON: {e}"
                )],
                1,
            );
        }
    };

    if json {
        match serde_json::to_string_pretty(&packages) {
            Ok(payload) => CommandReport::success(payload.lines().map(ToOwned::to_owned).collect()),
            Err(e) => CommandReport::failure(
                vec![format!("pyenv: failed to serialize outdated list: {e}")],
                1,
            ),
        }
    } else {
        if packages.is_empty() {
            return CommandReport::success(vec![format!(
                "All packages in '{target}' are up to date."
            )]);
        }

        let mut lines = vec![format!("Outdated packages in '{target}':")];
        for pkg in packages {
            lines.push(format!(
                "  - {} (current {}, latest {})",
                pkg.name, pkg.version, pkg.latest_version
            ));
        }
        CommandReport::success(lines)
    }
}

/// Executes `python -m pip check` to flag broken package dependencies.
pub fn cmd_pip_check(ctx: &AppContext, target: &str, json: bool) -> CommandReport {
    let py_path = match resolve_interpreter_path(ctx, target) {
        Ok(p) => p,
        Err(e) => return CommandReport::failure(vec![e], 1),
    };

    let output = match Command::new(&py_path)
        .headless()
        .args(["-m", "pip", "check"])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            return CommandReport::failure(
                vec![format!("pyenv: failed to run pip check process: {e}")],
                1,
            );
        }
    };

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut conflicts = Vec::new();

    // Parse standard pip check lines:
    // "package version has requirement req, but you have installed."
    for line in stdout_str.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.contains("No broken requirements found") {
            continue;
        }

        // Quick fallback parsing split
        let parts: Vec<&str> = trimmed.split(" has requirement ").collect();
        if parts.len() == 2 {
            let left: Vec<&str> = parts[0].split_whitespace().collect();
            let right: Vec<&str> = parts[1].split(", but you have ").collect();
            if left.len() >= 2 && right.len() == 2 {
                conflicts.push(DependencyConflict {
                    package: left[0].to_string(),
                    requirement: right[0].to_string(),
                    installed: right[1].trim_end_matches('.').to_string(),
                    message: trimmed.to_string(),
                });
                continue;
            }
        }

        // Generic conflict mapping if parsing fails
        conflicts.push(DependencyConflict {
            package: "unknown".to_string(),
            requirement: "unknown".to_string(),
            installed: "unknown".to_string(),
            message: trimmed.to_string(),
        });
    }

    if json {
        match serde_json::to_string_pretty(&conflicts) {
            Ok(payload) => CommandReport::success(payload.lines().map(ToOwned::to_owned).collect()),
            Err(e) => CommandReport::failure(
                vec![format!("pyenv: failed to serialize conflicts list: {e}")],
                1,
            ),
        }
    } else {
        if conflicts.is_empty() {
            return CommandReport::success(vec![format!(
                "No broken requirements found in '{target}'."
            )]);
        }

        let mut lines = vec![format!("Broken requirements found in '{target}':")];
        for conflict in conflicts {
            lines.push(format!("  - {}", conflict.message));
        }
        CommandReport::success(lines)
    }
}

/// Runs our embedded, robust requirements parser and static conflict checker using the target environment interpreter.
pub fn cmd_pip_precheck_requirements(
    ctx: &AppContext,
    target: &str,
    path_or_url: &str,
) -> CommandReport {
    let py_path = match resolve_interpreter_path(ctx, target) {
        Ok(p) => p,
        Err(e) => return CommandReport::failure(vec![e], 1),
    };

    let helper_script = include_str!("helper.py");
    let temp_script_path =
        std::env::temp_dir().join(format!("pyenv-precheck-{}.py", std::process::id()));

    if let Err(e) = fs::write(&temp_script_path, helper_script) {
        return CommandReport::failure(
            vec![format!(
                "pyenv: failed to write precheck script helper to temp folder: {e}"
            )],
            1,
        );
    }

    let output = match Command::new(&py_path)
        .headless()
        .arg(&temp_script_path)
        .arg(path_or_url)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            let _ = fs::remove_file(&temp_script_path);
            return CommandReport::failure(
                vec![format!(
                    "pyenv: failed to run requirements precheck process: {e}"
                )],
                1,
            );
        }
    };

    let _ = fs::remove_file(&temp_script_path);

    if !output.status.success() {
        return CommandReport::failure(
            vec![
                format!("pyenv: requirements precheck process failed."),
                String::from_utf8_lossy(&output.stderr).to_string(),
            ],
            1,
        );
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    CommandReport::success(stdout_str.lines().map(ToOwned::to_owned).collect())
}

/// Executes our embedded python helper script with `--scan <dir>` to extract codebase imports and detect missing dependencies.
pub fn cmd_pip_analyze_imports(ctx: &AppContext, target: &str, dir_path: &str) -> CommandReport {
    let py_path = match resolve_interpreter_path(ctx, target) {
        Ok(p) => p,
        Err(e) => return CommandReport::failure(vec![e], 1),
    };

    let helper_script = include_str!("helper.py");
    let temp_script_path =
        std::env::temp_dir().join(format!("pyenv-scan-{}.py", std::process::id()));

    if let Err(e) = fs::write(&temp_script_path, helper_script) {
        return CommandReport::failure(
            vec![format!(
                "pyenv: failed to write scan script helper to temp folder: {e}"
            )],
            1,
        );
    }

    let output = match Command::new(&py_path)
        .headless()
        .arg(&temp_script_path)
        .arg("--scan")
        .arg(dir_path)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            let _ = fs::remove_file(&temp_script_path);
            return CommandReport::failure(
                vec![format!("pyenv: failed to run codebase scan process: {e}")],
                1,
            );
        }
    };

    let _ = fs::remove_file(&temp_script_path);

    if !output.status.success() {
        return CommandReport::failure(
            vec![
                format!("pyenv: codebase scanner process failed."),
                String::from_utf8_lossy(&output.stderr).to_string(),
            ],
            1,
        );
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    CommandReport::success(stdout_str.lines().map(ToOwned::to_owned).collect())
}

/// Installs requirements inside the target environment from a requirements.txt file or URL.
pub fn cmd_pip_install(ctx: &AppContext, target: &str, path_or_url: &str) -> CommandReport {
    let py_path = match resolve_interpreter_path(ctx, target) {
        Ok(p) => p,
        Err(e) => return CommandReport::failure(vec![e], 1),
    };

    // If it's a URL, we will download it using our Python script helper or save it locally first.
    let is_url = path_or_url.starts_with("http://") || path_or_url.starts_with("https://");
    let resolved_path = if is_url {
        // Download it to a temporary file via standard fetch first
        let helper_script = include_str!("helper.py");
        let temp_script_path =
            std::env::temp_dir().join(format!("pyenv-dl-{}.py", std::process::id()));
        let temp_reqs_path =
            std::env::temp_dir().join(format!("pyenv-reqs-{}.txt", std::process::id()));

        let download_code = format!(
            "{}\n\
             import sys\n\
             try:\n\
                 reqs = get_requirements(sys.argv[1])\n\
                 with open(sys.argv[2], 'w', encoding='utf-8') as f:\n\
                     for r in reqs:\n\
                         f.write(r['original'] + '\\n')\n\
             except Exception as e:\n\
                 print('ERROR:', e)\n\
                 sys.exit(1)\n",
            helper_script
        );

        if let Err(e) = fs::write(&temp_script_path, download_code) {
            return CommandReport::failure(
                vec![format!("pyenv: failed to write download helper: {e}")],
                1,
            );
        }

        let output = match Command::new(&py_path)
            .headless()
            .arg(&temp_script_path)
            .arg(path_or_url)
            .arg(&temp_reqs_path)
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                let _ = fs::remove_file(&temp_script_path);
                return CommandReport::failure(
                    vec![format!(
                        "pyenv: failed to run requirements downloader process: {e}"
                    )],
                    1,
                );
            }
        };

        let _ = fs::remove_file(&temp_script_path);

        if !output.status.success() || !temp_reqs_path.exists() {
            let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
            let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
            let _ = fs::remove_file(&temp_reqs_path);
            return CommandReport::failure(
                vec![
                    format!(
                        "pyenv: failed to download remote requirements.txt from {path_or_url}."
                    ),
                    stdout_str,
                    stderr_str,
                ],
                1,
            );
        }

        temp_reqs_path
    } else {
        PathBuf::from(path_or_url)
    };

    let output = match Command::new(&py_path)
        .headless()
        .args([
            "-m",
            "pip",
            "install",
            "-r",
            resolved_path.to_string_lossy().as_ref(),
        ])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            if is_url {
                let _ = fs::remove_file(&resolved_path);
            }
            return CommandReport::failure(
                vec![format!(
                    "pyenv: failed to run pip install requirements process: {e}"
                )],
                1,
            );
        }
    };

    if is_url {
        let _ = fs::remove_file(&resolved_path);
    }

    if !output.status.success() {
        return CommandReport::failure(
            vec![
                format!("pyenv: pip install requirements process failed."),
                String::from_utf8_lossy(&output.stderr).to_string(),
            ],
            1,
        );
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    CommandReport::success(stdout_str.lines().map(ToOwned::to_owned).collect())
}

/// Executes individual/batch package upgrades, optionally upgrading pip first if a pip update is available.
pub fn cmd_pip_update(
    ctx: &AppContext,
    target: &str,
    packages: &[String],
    all: bool,
) -> CommandReport {
    let py_path = match resolve_interpreter_path(ctx, target) {
        Ok(p) => p,
        Err(e) => return CommandReport::failure(vec![e], 1),
    };

    let mut pkgs_to_update = Vec::new();
    let mut update_pip_first = false;

    // Retrieve outdated packages
    let outdated_report = cmd_pip_outdated(ctx, target, true);
    if outdated_report.exit_code == 0 {
        let outdated_str = outdated_report.stdout.join("\n");
        if let Ok(outdated_pkgs) = serde_json::from_str::<Vec<OutdatedPackage>>(&outdated_str) {
            // Check if pip needs self-update
            for pkg in &outdated_pkgs {
                if pkg.name.to_lowercase() == "pip" {
                    update_pip_first = true;
                }
            }

            if all {
                for pkg in outdated_pkgs {
                    if pkg.name.to_lowercase() != "pip" {
                        pkgs_to_update.push(pkg.name);
                    }
                }
            } else {
                for requested in packages {
                    let req_lower = requested.to_lowercase();
                    if req_lower == "pip" {
                        update_pip_first = true;
                    } else if outdated_pkgs
                        .iter()
                        .any(|p| p.name.to_lowercase() == req_lower)
                    {
                        pkgs_to_update.push(requested.clone());
                    } else {
                        // Let them update regardless if they requested it explicitly
                        pkgs_to_update.push(requested.clone());
                    }
                }
            }
        }
    } else {
        // Fallback to updating requested packages if outdated check failed
        if !all {
            pkgs_to_update.extend(packages.iter().cloned());
        }
    }

    let mut stdout = Vec::new();

    // 1. Cozy self-update for pip if and only if pip has an update
    if update_pip_first {
        stdout.push("Self-updating pip first as an upgrade is available...".to_string());
        let pip_up_output = match Command::new(&py_path)
            .headless()
            .args(["-m", "pip", "install", "-U", "pip"])
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                return CommandReport::failure(
                    vec![format!("pyenv: failed to run pip self-update process: {e}")],
                    1,
                );
            }
        };

        if !pip_up_output.status.success() {
            return CommandReport::failure(
                vec![
                    format!("pyenv: pip self-update process failed."),
                    String::from_utf8_lossy(&pip_up_output.stderr).to_string(),
                ],
                1,
            );
        }
        stdout.push("pip self-update completed successfully.".to_string());
    }

    // If only pip was requested or there are no other packages to update
    if pkgs_to_update.is_empty() {
        if update_pip_first {
            return CommandReport::success(stdout);
        } else {
            return CommandReport::success(vec![
                "No packages requested or available for update.".to_string(),
            ]);
        }
    }

    // 2. Perform batch updates for libraries
    stdout.push(format!(
        "Updating packages: {} ...",
        pkgs_to_update.join(", ")
    ));

    let mut update_args = vec![
        "-m".to_string(),
        "pip".to_string(),
        "install".to_string(),
        "-U".to_string(),
    ];
    update_args.extend(pkgs_to_update);

    let output = match Command::new(&py_path)
        .headless()
        .args(&update_args)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            return CommandReport::failure(
                vec![format!(
                    "pyenv: failed to run pip batch update process: {e}"
                )],
                1,
            );
        }
    };

    if !output.status.success() {
        return CommandReport::failure(
            vec![
                format!("pyenv: pip batch update process failed."),
                String::from_utf8_lossy(&output.stderr).to_string(),
            ],
            1,
        );
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    stdout.extend(stdout_str.lines().map(ToOwned::to_owned));
    CommandReport::success(stdout)
}
