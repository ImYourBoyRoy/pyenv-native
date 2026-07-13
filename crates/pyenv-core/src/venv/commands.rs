// ./crates/pyenv-core/src/venv/commands.rs
//! Public managed-venv commands for list/info/create/delete/rename/use operations.

use crate::process::PyenvCommandExt;
use std::fs;
use std::process::Command;

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::venv_paths::{managed_venv_dir, managed_venv_spec, managed_venvs_root};
use crate::version::{cmd_global, cmd_local, installed_version_dir};

use super::helpers::{
    confirm_action, format_collision_error, io_error, is_safe_env_name, json_success,
};
use super::inventory::{
    build_managed_venv_info, find_env_name_matches, list_managed_venvs,
    resolve_installed_runtime_version, resolve_managed_venv,
};
use super::types::VenvUseScope;

pub fn cmd_venv_list(ctx: &AppContext, bare: bool, json: bool) -> CommandReport {
    match list_managed_venvs(ctx) {
        Ok(venvs) => {
            if json {
                return json_success(&venvs);
            }

            if bare {
                return CommandReport::success(venvs.into_iter().map(|info| info.spec).collect());
            }

            if venvs.is_empty() {
                return CommandReport::success(vec![
                    "No managed virtual environments found.".to_string(),
                    "Create one with `pyenv venv create <runtime> <name>`.".to_string(),
                ]);
            }

            let mut stdout = vec!["Managed virtual environments:".to_string()];
            stdout.extend(venvs.into_iter().map(|info| {
                format!(
                    "  - {} (base {}, python {})",
                    info.spec,
                    info.base_version,
                    info.python_path
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "missing".to_string())
                )
            }));
            CommandReport::success(stdout)
        }
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

pub fn cmd_venv_info(ctx: &AppContext, spec: &str, json: bool) -> CommandReport {
    match resolve_managed_venv(ctx, spec) {
        Ok(info) => {
            if json {
                return json_success(&info);
            }

            let mut stdout = vec![
                format!("Name: {}", info.name),
                format!("Spec: {}", info.spec),
                format!("Base runtime: {}", info.base_version),
                format!("Location: {}", info.path.display()),
            ];
            stdout.push(format!(
                "Python: {}",
                info.python_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "missing".to_string())
            ));
            stdout.push(format!(
                "Pip: {}",
                info.pip_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "missing".to_string())
            ));
            CommandReport::success(stdout)
        }
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

pub fn cmd_venv_create(
    ctx: &AppContext,
    requested_version: &str,
    name: &str,
    force: bool,
    set_local: bool,
) -> CommandReport {
    match create_managed_venv(ctx, requested_version, name, force, set_local) {
        Ok((info, local_written, progress_steps)) => {
            let mut stdout = vec!["Progress:".to_string()];
            stdout.extend(progress_steps.into_iter().map(|step| format!("  - {step}")));
            stdout.push(format!("Managed venv created: {}", info.spec));
            stdout.push(format!("Location: {}", info.path.display()));
            stdout.push(format!(
                "Python: {}",
                info.python_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "missing".to_string())
            ));
            stdout.push(format!(
                "Pip: {}",
                info.pip_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "missing".to_string())
            ));
            stdout.push(format!("Local version updated: {local_written}"));
            CommandReport::success(stdout)
        }
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

pub fn cmd_venv_delete(ctx: &AppContext, spec: &str, force: bool) -> CommandReport {
    let info = match resolve_managed_venv(ctx, spec) {
        Ok(info) => info,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };

    if !force && !confirm_action(&format!("pyenv: remove managed venv {}? [y/N] ", info.spec)) {
        return CommandReport::failure(
            vec!["pyenv: managed venv removal cancelled".to_string()],
            1,
        );
    }

    match fs::remove_dir_all(&info.path) {
        Ok(_) => CommandReport::success(vec![
            format!("Removed managed venv {}", info.spec),
            "Hint: update any `.python-version` files that pointed at this venv.".to_string(),
        ]),
        Err(error) => CommandReport::failure(
            vec![format!(
                "pyenv: failed to remove managed venv {}: {error}",
                info.path.display()
            )],
            1,
        ),
    }
}

pub fn cmd_venv_rename(ctx: &AppContext, spec: &str, new_name: &str) -> CommandReport {
    let info = match resolve_managed_venv(ctx, spec) {
        Ok(info) => info,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };

    if !is_safe_env_name(new_name) {
        return CommandReport::failure(
            vec![format!(
                "pyenv: invalid managed venv name `{new_name}`; use letters, numbers, ., _, or -"
            )],
            1,
        );
    }

    let collisions = find_env_name_matches(ctx, new_name).unwrap_or_default();
    if collisions.iter().any(|item| item.spec != info.spec) {
        return CommandReport::failure(vec![format_collision_error(new_name, &collisions)], 1);
    }

    let target = managed_venv_dir(ctx, &info.base_version, new_name);
    if target.exists() {
        return CommandReport::failure(
            vec![format!(
                "pyenv: managed venv target already exists at {}",
                target.display()
            )],
            1,
        );
    }

    match fs::rename(&info.path, &target) {
        Ok(_) => CommandReport::success(vec![
            format!(
                "Renamed managed venv {} -> {}/envs/{}",
                info.spec, info.base_version, new_name
            ),
            "Hint: update any `.python-version` files that referenced the old venv spec."
                .to_string(),
        ]),
        Err(error) => CommandReport::failure(
            vec![format!(
                "pyenv: failed to rename managed venv {}: {error}",
                info.spec
            )],
            1,
        ),
    }
}

pub fn cmd_venv_use(ctx: &AppContext, spec: &str, scope: VenvUseScope) -> CommandReport {
    let info = match resolve_managed_venv(ctx, spec) {
        Ok(info) => info,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };

    let report = match scope {
        VenvUseScope::Local => cmd_local(ctx, std::slice::from_ref(&info.spec), false, true),
        VenvUseScope::Global => cmd_global(ctx, std::slice::from_ref(&info.spec), false),
    };

    if report.exit_code != 0 {
        return report;
    }

    let scope_label = match scope {
        VenvUseScope::Local => "local",
        VenvUseScope::Global => "global",
    };
    CommandReport::success(vec![
        format!("Selected managed venv {} for {scope_label} use.", info.spec),
        format!(
            "Hint: `python` and `pip` will now resolve from {} once shims are active.",
            info.spec
        ),
    ])
}

fn create_managed_venv(
    ctx: &AppContext,
    requested_version: &str,
    name: &str,
    force: bool,
    set_local: bool,
) -> Result<(super::types::ManagedVenvInfo, bool, Vec<String>), PyenvError> {
    if !is_safe_env_name(name) {
        return Err(PyenvError::Io(format!(
            "pyenv: invalid managed venv name `{name}`; use letters, numbers, ., _, or -"
        )));
    }

    let resolved_version = resolve_installed_runtime_version(ctx, requested_version)?;
    let collisions = find_env_name_matches(ctx, name)?;
    if !collisions.is_empty() {
        let exact_spec = managed_venv_spec(&resolved_version, name);
        if !(force && collisions.iter().all(|item| item.spec == exact_spec)) {
            return Err(PyenvError::Io(format_collision_error(name, &collisions)));
        }
    }

    let base_prefix = installed_version_dir(ctx, &resolved_version);
    let interpreter_path =
        super::helpers::interpreter_for_prefix(&base_prefix).ok_or_else(|| {
            PyenvError::Io(format!(
                "pyenv: failed to locate a Python interpreter under {}",
                base_prefix.display()
            ))
        })?;

    let registry_root = managed_venvs_root(ctx);
    fs::create_dir_all(&registry_root).map_err(io_error)?;
    let venv_path = managed_venv_dir(ctx, &resolved_version, name);

    let mut progress_steps = vec![
        format!(
            "plan: resolved base runtime {} -> {}",
            requested_version, resolved_version
        ),
        format!("venv: target managed env path {}", venv_path.display()),
    ];

    if let Some(parent) = venv_path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }

    if venv_path.exists() {
        if !force {
            let spec = managed_venv_spec(&resolved_version, name);
            return Err(PyenvError::Io(format!(
                "pyenv: managed venv `{}` already exists; use --force to recreate it",
                spec
            )));
        }
        fs::remove_dir_all(&venv_path).map_err(io_error)?;
        progress_steps.push(format!(
            "cleanup: removed existing managed env at {}",
            venv_path.display()
        ));
    }

    let status = Command::new(&interpreter_path)
        .headless()
        .arg("-m")
        .arg("venv")
        .arg(&venv_path)
        .status()
        .map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to run '{}' -m venv {}: {error}",
                interpreter_path.display(),
                venv_path.display()
            ))
        })?;
    if !status.success() {
        return Err(PyenvError::Io(format!(
            "pyenv: '{}' -m venv {} exited with status {:?}",
            interpreter_path.display(),
            venv_path.display(),
            status.code()
        )));
    }
    progress_steps.push(format!("venv: created managed env {}", venv_path.display()));

    let info = build_managed_venv_info(resolved_version.clone(), name.to_string(), venv_path);

    if let Some(python_path) = info.python_path.as_ref() {
        progress_steps.push("pip: upgrading pip to the latest release".to_string());
        let pip_upgrade = Command::new(python_path)
            .headless()
            .args(["-m", "pip", "install", "-U", "pip"])
            .status();
        match pip_upgrade {
            Ok(status) if status.success() => {
                progress_steps.push("pip: pip upgraded successfully".to_string());
            }
            _ => progress_steps.push(
                "pip: pip upgrade skipped or failed; continuing with bundled pip".to_string(),
            ),
        }
    }

    let local_written = if set_local {
        let report = cmd_local(ctx, std::slice::from_ref(&info.spec), false, true);
        if report.exit_code != 0 {
            return Err(PyenvError::Io(report.stderr.join("\n")));
        }
        progress_steps.push(format!(
            "selection: wrote local .python-version for {}",
            info.spec
        ));
        true
    } else {
        false
    };

    Ok((info, local_written, progress_steps))
}

pub fn cmd_venv_upgrade(
    ctx: &AppContext,
    spec: &str,
    new_runtime: &str,
    force: bool,
    set_local: bool,
) -> CommandReport {
    // 1. Resolve old managed venv
    let info = match resolve_managed_venv(ctx, spec) {
        Ok(i) => i,
        Err(e) => return CommandReport::failure(vec![e.to_string()], 1),
    };

    // 2. Resolve target Python version
    let resolved_new_version = match resolve_installed_runtime_version(ctx, new_runtime) {
        Ok(ver) => ver,
        Err(e) => {
            return CommandReport::failure(
                vec![
                    format!("pyenv: target new runtime `{new_runtime}` is not installed."),
                    format!("Error: {e}"),
                    format!("Hint: install it first with `pyenv install {new_runtime}`"),
                ],
                1,
            );
        }
    };

    // 3. Query installed pip packages in old venv
    let py_path = match &info.python_path {
        Some(p) if p.exists() => p,
        _ => {
            return CommandReport::failure(
                vec![format!(
                    "pyenv: python interpreter for managed venv `{}` is missing from disk.",
                    info.spec
                )],
                1,
            );
        }
    };

    let mut packages = Vec::new();
    let mut backup_success = false;
    if let Some(output) = Command::new(py_path)
        .headless()
        .args(["-m", "pip", "list", "--format=json"])
        .output()
        .ok()
        .filter(|o| o.status.success())
    {
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        if let Ok(pkgs) = serde_json::from_str::<Vec<crate::pip::PipPackage>>(&stdout_str) {
            packages = pkgs;
            backup_success = true;
        }
    }

    let pkgs_to_install: Vec<String> = packages
        .iter()
        .filter(|pkg| {
            let name_lower = pkg.name.to_lowercase();
            name_lower != "pip"
                && name_lower != "setuptools"
                && name_lower != "wheel"
                && name_lower != "distribute"
        })
        .map(|pkg| format!("{}=={}", pkg.name, pkg.version))
        .collect();

    // 4. Confirm upgrade with user
    if !force {
        let msg = format!(
            "pyenv: migrate managed venv {} to runtime {}? (this will recreate the environment and attempt to reinstall {} packages) [y/N] ",
            info.spec,
            resolved_new_version,
            pkgs_to_install.len()
        );
        if !confirm_action(&msg) {
            return CommandReport::failure(
                vec!["pyenv: virtual environment upgrade cancelled".to_string()],
                1,
            );
        }
    }

    let mut stdout = vec!["Progress:".to_string()];
    if backup_success {
        stdout.push(format!(
            "  - backup: captured {} custom packages from {}",
            pkgs_to_install.len(),
            info.spec
        ));
    } else {
        stdout.push(
            "  - [WARNING] failed to query old packages list; creating clean environment"
                .to_string(),
        );
    }

    // 5. Recreate environment under new target runtime
    let (new_info, _local_written, create_steps) =
        match create_managed_venv(ctx, &resolved_new_version, &info.name, true, set_local) {
            Ok(res) => res,
            Err(e) => {
                return CommandReport::failure(
                    vec![format!("pyenv: failed to create new venv: {e}")],
                    1,
                );
            }
        };

    stdout.extend(create_steps.into_iter().map(|s| format!("  - {s}")));

    let new_py_path = match &new_info.python_path {
        Some(p) if p.exists() => p,
        _ => {
            return CommandReport::failure(
                vec![format!(
                    "pyenv: new python interpreter was not found on disk at {:?}",
                    new_info.python_path
                )],
                1,
            );
        }
    };

    // 6. Cozy self-update for pip in new environment
    stdout.push("  - pip: updating pip to the latest version...".to_string());
    let pip_upgrade = Command::new(new_py_path)
        .headless()
        .args(["-m", "pip", "install", "-U", "pip"])
        .status();
    match pip_upgrade {
        Ok(status) if status.success() => {
            stdout.push("    - pip updated successfully".to_string());
        }
        _ => {
            stdout.push(
                "    - [WARNING] pip update failed, continuing with default version".to_string(),
            );
        }
    }

    // 7. Reinstall the backed up packages progressively
    let total_pkgs = pkgs_to_install.len();
    if total_pkgs > 0 {
        stdout.push(format!(
            "  - restore: restoring {total_pkgs} custom packages..."
        ));
        for (i, pkg_spec) in pkgs_to_install.iter().enumerate() {
            stdout.push(format!(
                "    [{}/{}] installing {}...",
                i + 1,
                total_pkgs,
                pkg_spec
            ));
            let install_status = Command::new(new_py_path)
                .headless()
                .args(["-m", "pip", "install", pkg_spec])
                .status();
            match install_status {
                Ok(status) if status.success() => {
                    stdout.push(format!("      - installed {}", pkg_spec));
                }
                Ok(status) => {
                    stdout.push(format!(
                        "      - [WARNING] failed to install {} (exit code {:?})",
                        pkg_spec,
                        status.code()
                    ));
                }
                Err(e) => {
                    stdout.push(format!(
                        "      - [WARNING] failed to launch pip install for {}: {}",
                        pkg_spec, e
                    ));
                }
            }
        }
    }

    // 8. Verify dependency health via pip check
    stdout.push("  - verify: checking dependency constraints...".to_string());
    let check_status = Command::new(new_py_path)
        .headless()
        .args(["-m", "pip", "check"])
        .output();
    match check_status {
        Ok(output) if output.status.success() => {
            stdout.push(
                "    - verification success: all dependency constraints satisfied!".to_string(),
            );
        }
        Ok(output) => {
            stdout.push("    - [WARNING] dependency conflicts detected:".to_string());
            let err_str = String::from_utf8_lossy(&output.stdout);
            for line in err_str.lines() {
                stdout.push(format!("      ! {}", line));
            }
        }
        Err(e) => {
            stdout.push(format!("    - failed to run verification check: {}", e));
        }
    }

    // 9. Cleanup old venv if target is different from source
    if info.path != new_info.path {
        stdout.push("  - cleanup: removing old virtual environment...".to_string());
        match fs::remove_dir_all(&info.path) {
            Ok(_) => {
                stdout.push(format!(
                    "    - removed old managed venv at {}",
                    info.path.display()
                ));
            }
            Err(e) => {
                stdout.push(format!(
                    "    - [WARNING] failed to remove old venv at {}: {}",
                    info.path.display(),
                    e
                ));
            }
        }
    }

    stdout.push(format!(
        "Managed venv successfully upgraded to: {}",
        new_info.spec
    ));
    CommandReport::success(stdout)
}
