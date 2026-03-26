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
