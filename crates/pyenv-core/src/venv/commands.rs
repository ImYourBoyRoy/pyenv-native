// ./crates/pyenv-core/src/venv/commands.rs
//! Public managed-venv commands for list/info/create/delete/rename/use operations.

use crate::process::PyenvCommandExt;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PackageRestoreMode {
    Exact,
    Compatible,
}

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
    let info = match resolve_managed_venv(ctx, spec) {
        Ok(i) => i,
        Err(e) => return CommandReport::failure(vec![e.to_string()], 1),
    };

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

    if info.base_version == resolved_new_version {
        return CommandReport::failure(
            vec![format!(
                "pyenv: managed venv {} is already using runtime {}",
                info.spec, resolved_new_version
            )],
            1,
        );
    }

    let snapshot = match snapshot_packages(info.python_path.as_deref()) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return CommandReport::failure(
                vec![
                    "pyenv: cannot inventory packages in the source venv; refusing to migrate."
                        .to_string(),
                    error,
                ],
                1,
            );
        }
    };

    let exact_package_requirements = snapshot.exact_requirements;
    let compatible_package_requirements = snapshot.compatible_requirements;
    let package_count = snapshot.package_count;

    let destination = managed_venv_dir(ctx, &resolved_new_version, &info.name);
    if destination.exists() && !force {
        return CommandReport::failure(
            vec![format!(
                "pyenv: target managed venv `{}` already exists at {}; use --force to replace it",
                managed_venv_spec(&resolved_new_version, &info.name),
                destination.display()
            )],
            1,
        );
    }

    if !force {
        let msg = format!(
            "pyenv: migrate managed venv {} to runtime {}? (this will recreate the environment and reinstall {} packages) [y/N] ",
            info.spec, resolved_new_version, package_count
        );
        if !confirm_action(&msg) {
            return CommandReport::failure(
                vec!["pyenv: virtual environment upgrade cancelled".to_string()],
                1,
            );
        }
    }

    let temp_name = match allocate_migrating_name(ctx, &info.name) {
        Ok(name) => name,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };

    let mut stdout = vec!["Progress:".to_string()];
    stdout.push(format!(
        "  - backup: captured {} custom packages from {}",
        package_count, info.spec
    ));

    let (mut new_info, _local_written, create_steps) =
        match create_managed_venv(ctx, &resolved_new_version, &temp_name, false, false) {
            Ok(res) => res,
            Err(e) => {
                return CommandReport::failure(
                    vec![format!("pyenv: failed to create migration venv: {e}")],
                    1,
                );
            }
        };
    stdout.extend(create_steps.into_iter().map(|s| format!("  - {s}")));

    let new_py_path = match &new_info.python_path {
        Some(p) if p.exists() => p.clone(),
        _ => {
            let _ = fs::remove_dir_all(&new_info.path);
            return CommandReport::failure(
                vec![format!(
                    "pyenv: new python interpreter was not found on disk at {:?}",
                    new_info.python_path
                )],
                1,
            );
        }
    };

    stdout.push("  - verify: checking dependency constraints...".to_string());
    let mut restore_mode = PackageRestoreMode::Exact;
    let exact_restore = restore_packages_and_verify(
        &new_py_path,
        &new_info.path,
        &exact_package_requirements,
        false,
    );
    if let Err(exact_error) = exact_restore {
        if package_count == 0 {
            let _ = fs::remove_dir_all(&new_info.path);
            stdout.push(format!(
                "  - verification failed for the fresh environment: {exact_error}"
            ));
            stdout.push(
                "pyenv: dependency verification failed; the source venv was left unchanged."
                    .to_string(),
            );
            return CommandReport::failure(stdout, 1);
        }

        stdout.push(format!(
            "  - compatibility: exact package restore was not usable on runtime {}: {exact_error}",
            resolved_new_version
        ));
        if let Err(cleanup_error) = fs::remove_dir_all(&new_info.path) {
            stdout.push(format!(
                "pyenv: could not reset the migration staging environment: {cleanup_error}"
            ));
            stdout.push("The source venv was left unchanged.".to_string());
            return CommandReport::failure(stdout, 1);
        }

        let (refreshed_info, _local_written, refresh_steps) =
            match create_managed_venv(ctx, &resolved_new_version, &temp_name, true, false) {
                Ok(res) => res,
                Err(error) => {
                    stdout.push(format!(
                        "pyenv: could not recreate the compatibility staging environment: {error}"
                    ));
                    stdout.push("The source venv was left unchanged.".to_string());
                    return CommandReport::failure(stdout, 1);
                }
            };
        new_info = refreshed_info;
        stdout.extend(
            refresh_steps
                .into_iter()
                .map(|step| format!("  - compatibility: {step}")),
        );

        let refreshed_python = match &new_info.python_path {
            Some(path) if path.exists() => path.clone(),
            _ => {
                let _ = fs::remove_dir_all(&new_info.path);
                stdout.push(
                    "pyenv: compatibility staging environment has no usable Python interpreter."
                        .to_string(),
                );
                stdout.push("The source venv was left unchanged.".to_string());
                return CommandReport::failure(stdout, 1);
            }
        };

        stdout.push(format!(
            "  - compatibility: resolving {} package lower bounds for the target Python...",
            package_count
        ));
        if let Err(compatible_error) = restore_packages_and_verify(
            &refreshed_python,
            &new_info.path,
            &compatible_package_requirements,
            true,
        ) {
            let _ = fs::remove_dir_all(&new_info.path);
            stdout.push(format!(
                "pyenv: compatible package restore and verification failed: {compatible_error}"
            ));
            stdout.push("The source venv was left unchanged.".to_string());
            return CommandReport::failure(stdout, 1);
        }
        restore_mode = PackageRestoreMode::Compatible;
    }

    match restore_mode {
        PackageRestoreMode::Exact => stdout.push(format!(
            "  - restore: preserved {} package versions",
            package_count
        )),
        PackageRestoreMode::Compatible => stdout.push(format!(
            "  - restore: installed {} target-compatible package versions",
            package_count
        )),
    }
    stdout.push("    - verification success: all dependency constraints satisfied!".to_string());

    let backup_path = if destination.exists() {
        let backup = allocate_replacement_backup_path(ctx, &resolved_new_version, &info.name)
            .map_err(|error| error.to_string());
        let backup = match backup {
            Ok(path) => path,
            Err(error) => {
                let _ = fs::remove_dir_all(&new_info.path);
                stdout.push(format!(
                    "pyenv: cannot prepare the existing target for replacement: {error}"
                ));
                stdout.push("The source venv was left unchanged.".to_string());
                return CommandReport::failure(stdout, 1);
            }
        };
        if let Err(error) = fs::rename(&destination, &backup) {
            let _ = fs::remove_dir_all(&new_info.path);
            stdout.push(format!(
                "pyenv: could not move the existing target venv to a rollback path: {error}"
            ));
            stdout.push("The source venv was left unchanged.".to_string());
            return CommandReport::failure(stdout, 1);
        }
        stdout.push(format!(
            "  - target: moved existing {} to {} for rollback",
            managed_venv_spec(&resolved_new_version, &info.name),
            backup.display()
        ));
        Some(backup)
    } else {
        None
    };

    if let Err(error) = fs::rename(&new_info.path, &destination) {
        if let Some(backup) = backup_path.as_ref()
            && let Err(restore_error) = fs::rename(backup, &destination)
        {
            stdout.push(format!(
                "pyenv: failed to restore the previous target after rename failure: {restore_error}"
            ));
        }
        stdout.push(format!(
            "pyenv: could not activate the migrated venv at {}: {error}",
            destination.display()
        ));
        stdout.push("The source venv was left unchanged.".to_string());
        return CommandReport::failure(stdout, 1);
    }
    stdout.push(format!(
        "  - finalize: activated migrated venv at {}",
        destination.display()
    ));

    stdout.push("  - cleanup: removing old virtual environment...".to_string());
    if let Err(error) = fs::remove_dir_all(&info.path) {
        stdout.push(format!(
            "pyenv: the migrated venv is ready at {}, but the source venv could not be removed: {error}",
            destination.display()
        ));
        stdout.push(format!("    - source retained at {}", info.path.display()));
        return CommandReport::failure(stdout, 1);
    }
    stdout.push(format!(
        "    - removed old managed venv at {}",
        info.path.display()
    ));

    if let Some(backup) = backup_path {
        if let Err(error) = fs::remove_dir_all(&backup) {
            stdout.push(format!(
                "pyenv: migration completed, but the previous target rollback copy could not be removed: {} ({error})",
                backup.display()
            ));
            return CommandReport::failure(stdout, 1);
        }
        stdout.push(format!(
            "  - target: removed rollback copy {}",
            backup.display()
        ));
    }

    let final_spec = managed_venv_spec(&resolved_new_version, &info.name);
    retarget_selection_files(ctx, &info.spec, &final_spec, &mut stdout);

    if set_local {
        let report = cmd_local(ctx, std::slice::from_ref(&final_spec), false, true);
        if report.exit_code != 0 {
            stdout.extend(report.stderr);
            stdout.push(format!(
                "pyenv: migrated to {final_spec}, but writing .python-version failed."
            ));
            return CommandReport::failure(stdout, 1);
        }
        stdout.push(format!(
            "  - selection: wrote local .python-version for {final_spec}"
        ));
    }

    stdout.push(format!(
        "Managed venv successfully upgraded to: {final_spec}"
    ));
    CommandReport::success(stdout)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PackageSnapshot {
    exact_requirements: Vec<String>,
    compatible_requirements: Vec<String>,
    package_count: usize,
}

fn snapshot_packages(python_path: Option<&Path>) -> Result<PackageSnapshot, String> {
    let python_path = python_path.ok_or_else(|| {
        "pyenv: source managed venv does not have a usable Python interpreter".to_string()
    })?;
    let output = Command::new(python_path)
        .headless()
        .env("PIP_DISABLE_PIP_VERSION_CHECK", "1")
        .env("PIP_NO_COLOR", "1")
        .env("NO_COLOR", "1")
        .args(["-m", "pip", "freeze", "--all"])
        .output()
        .map_err(|error| format!("pyenv: failed to run pip freeze: {error}"))?;
    if !output.status.success() {
        return Err(format_process_failure("pip freeze", &output));
    }

    let exact_requirements = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter(|line| !is_bootstrap_requirement(line))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let compatible_requirements = exact_requirements
        .iter()
        .map(|line| lower_bound_requirement(line))
        .collect::<Vec<_>>();
    let package_count = exact_requirements
        .iter()
        .filter(|line| !line.starts_with("--"))
        .count();

    Ok(PackageSnapshot {
        exact_requirements,
        compatible_requirements,
        package_count,
    })
}

fn is_bootstrap_requirement(line: &str) -> bool {
    let lowered = line.to_ascii_lowercase();
    if lowered.contains("#egg=pip")
        || lowered.contains("#egg=setuptools")
        || lowered.contains("#egg=wheel")
        || lowered.contains("#egg=distribute")
    {
        return true;
    }

    let candidate = line.strip_prefix("-e ").unwrap_or(line).trim();
    let name = candidate
        .split(['[', '=', '<', '>', '!', '~', '@', ' ', ';'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(name.as_str(), "pip" | "setuptools" | "wheel" | "distribute")
}

pub(super) fn lower_bound_requirement(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('-')
        || trimmed.starts_with("git+")
        || trimmed.contains(" @ ")
        || trimmed.contains("://")
    {
        return trimmed.to_string();
    }

    let Some((name, version)) = trimmed.split_once("==") else {
        return trimmed.to_string();
    };
    let safe_name = !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'));
    if safe_name && !version.is_empty() {
        format!("{name}>={version}")
    } else {
        trimmed.to_string()
    }
}

fn restore_packages_and_verify(
    python_path: &Path,
    staging_path: &Path,
    requirements: &[String],
    lower_bound: bool,
) -> Result<(), String> {
    let requirements_path = if requirements.is_empty() {
        None
    } else {
        Some(write_requirements_file(staging_path, requirements)?)
    };
    let install_result = if let Some(requirements_path) = requirements_path.as_ref() {
        run_pip_install_requirements(python_path, requirements_path, lower_bound)
    } else {
        Ok(())
    };
    if let Some(requirements_path) = requirements_path {
        let _ = fs::remove_file(requirements_path);
    }
    install_result?;
    run_pip_check(python_path)
}

fn write_requirements_file(
    staging_path: &Path,
    requirements: &[String],
) -> Result<std::path::PathBuf, String> {
    let path = staging_path.join(".pyenv-migration-requirements.txt");
    let mut contents = requirements.join("\n");
    contents.push('\n');
    fs::write(&path, contents)
        .map_err(|error| format!("pyenv: failed to write migration requirements: {error}"))?;
    Ok(path)
}

fn run_pip_install_requirements(
    python_path: &Path,
    requirements_path: &Path,
    lower_bound: bool,
) -> Result<(), String> {
    let mut command = Command::new(python_path);
    command.headless().args(["-m", "pip", "install"]);
    if lower_bound {
        command.args(["--upgrade", "--upgrade-strategy", "eager"]);
    }
    let output = command
        .args(["-r", &requirements_path.to_string_lossy()])
        .output()
        .map_err(|error| format!("failed to launch pip install: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format_process_failure("pip install", &output))
    }
}

fn run_pip_check(python_path: &Path) -> Result<(), String> {
    let output = Command::new(python_path)
        .headless()
        .args(["-m", "pip", "check"])
        .output()
        .map_err(|error| format!("failed to launch pip check: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format_process_failure("pip check", &output))
    }
}

fn format_process_failure(label: &str, output: &Output) -> String {
    let mut details = String::new();
    for text in [
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    ] {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !details.is_empty() {
            details.push_str(" | ");
        }
        details.push_str(trimmed);
    }

    if details.is_empty() {
        format!(
            "{label} exited with status {:?}",
            output.status.code().unwrap_or(1)
        )
    } else {
        format!(
            "{label} exited with status {:?}: {details}",
            output.status.code().unwrap_or(1)
        )
    }
}

fn allocate_replacement_backup_path(
    ctx: &AppContext,
    base_version: &str,
    name: &str,
) -> Result<std::path::PathBuf, PyenvError> {
    for suffix in 0..100u32 {
        let candidate = if suffix == 0 {
            format!("{name}-previous")
        } else {
            format!("{name}-previous-{suffix}")
        };
        if !is_safe_env_name(&candidate) {
            continue;
        }
        let path = managed_venv_dir(ctx, base_version, &candidate);
        if !path.exists() {
            return Ok(path);
        }
    }
    Err(PyenvError::Io(format!(
        "pyenv: could not allocate a rollback path for managed venv `{name}`"
    )))
}

fn allocate_migrating_name(ctx: &AppContext, original: &str) -> Result<String, PyenvError> {
    for suffix in 0..100u32 {
        let candidate = if suffix == 0 {
            format!("{original}-migrating")
        } else {
            format!("{original}-migrating-{suffix}")
        };
        if !is_safe_env_name(&candidate) {
            continue;
        }
        if find_env_name_matches(ctx, &candidate)?.is_empty() {
            return Ok(candidate);
        }
    }
    Err(PyenvError::Io(format!(
        "pyenv: could not allocate a temporary migration name for `{original}`"
    )))
}

fn retarget_selection_files(
    ctx: &AppContext,
    old_spec: &str,
    new_spec: &str,
    stdout: &mut Vec<String>,
) {
    let local = ctx.dir.join(".python-version");
    if replace_spec_in_file(&local, old_spec, new_spec) {
        stdout.push(format!(
            "  - selection: updated {} from {old_spec} to {new_spec}",
            local.display()
        ));
    }
    let global = ctx.root.join("version");
    if replace_spec_in_file(&global, old_spec, new_spec) {
        stdout.push(format!(
            "  - selection: updated global version file from {old_spec} to {new_spec}"
        ));
    }
}

fn replace_spec_in_file(path: &std::path::Path, old_spec: &str, new_spec: &str) -> bool {
    let Ok(contents) = fs::read_to_string(path) else {
        return false;
    };
    let mut changed = false;
    let mut lines = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed == old_spec {
            lines.push(new_spec.to_string());
            changed = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !changed {
        return false;
    }
    let mut next = lines.join("\n");
    if contents.ends_with('\n') {
        next.push('\n');
    }
    fs::write(path, next).is_ok()
}
