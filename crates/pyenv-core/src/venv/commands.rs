// ./crates/pyenv-core/src/venv/commands.rs
//! Public managed-venv commands for list/info/create/delete/rename/use operations.

use crate::process::PyenvCommandExt;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::fs_util::{remove_dir_all_with_retry, rename_with_retry};
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
    match create_managed_venv(ctx, requested_version, name, force, set_local, true) {
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
            "Warning: activate scripts, script shebangs, and (on Windows) console-script launchers inside the venv still reference the old path; prefer `pyenv venv upgrade` or recreate the venv if entry points such as `pip` stop working."
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
    check_name_collisions: bool,
) -> Result<(super::types::ManagedVenvInfo, bool, Vec<String>), PyenvError> {
    if !is_safe_env_name(name) {
        return Err(PyenvError::Io(format!(
            "pyenv: invalid managed venv name `{name}`; use letters, numbers, ., _, or -"
        )));
    }

    let resolved_version = resolve_installed_runtime_version(ctx, requested_version)?;
    if check_name_collisions {
        let collisions = find_env_name_matches(ctx, name)?;
        if !collisions.is_empty() {
            let exact_spec = managed_venv_spec(&resolved_version, name);
            if !(force && collisions.iter().all(|item| item.spec == exact_spec)) {
                return Err(PyenvError::Io(format_collision_error(name, &collisions)));
            }
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

/// Whether a venv upgrade may ask for confirmation on stdin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpgradeConfirmation {
    Prompt,
    AssumeYes,
}

/// Upgrade (migrate) a managed venv onto another installed runtime.
///
/// `force` skips the confirmation prompt and permits replacing an existing target venv.
pub fn cmd_venv_upgrade(
    ctx: &AppContext,
    spec: &str,
    new_runtime: &str,
    force: bool,
    set_local: bool,
) -> CommandReport {
    let confirmation = if force {
        UpgradeConfirmation::AssumeYes
    } else {
        UpgradeConfirmation::Prompt
    };
    venv_upgrade_impl(ctx, spec, new_runtime, force, set_local, confirmation)
}

/// Same as [`cmd_venv_upgrade`] but never reads from stdin. Use this from hosts whose stdin/stdout
/// are a protocol channel (the stdio MCP server) or that have no terminal (GUI). `replace_existing`
/// only controls whether an already-present target venv may be replaced via a rollback path.
pub fn cmd_venv_upgrade_noninteractive(
    ctx: &AppContext,
    spec: &str,
    new_runtime: &str,
    replace_existing: bool,
    set_local: bool,
) -> CommandReport {
    venv_upgrade_impl(
        ctx,
        spec,
        new_runtime,
        replace_existing,
        set_local,
        UpgradeConfirmation::AssumeYes,
    )
}

fn venv_upgrade_impl(
    ctx: &AppContext,
    spec: &str,
    new_runtime: &str,
    replace_existing: bool,
    set_local: bool,
    confirmation: UpgradeConfirmation,
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
    let destination_exists = destination.exists();
    if destination_exists && !replace_existing {
        return CommandReport::failure(
            vec![format!(
                "pyenv: target managed venv `{}` already exists at {}; use --force to replace it",
                managed_venv_spec(&resolved_new_version, &info.name),
                destination.display()
            )],
            1,
        );
    }

    if confirmation == UpgradeConfirmation::Prompt {
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

    let mut stdout = vec!["Progress:".to_string()];
    stdout.push(format!(
        "  - backup: captured {} custom packages from {}",
        package_count, info.spec
    ));

    // The new environment is built directly at its final path. A venv bakes its own absolute path
    // into `activate*`, script shebangs and (on Windows) the console-script `.exe` launchers, so
    // building it elsewhere and renaming the directory would leave every entry point broken.
    // An existing target is moved aside first so it can be restored if anything fails.
    let backup_path = if destination_exists {
        let backup = match allocate_replacement_backup_path(ctx, &resolved_new_version, &info.name)
        {
            Ok(path) => path,
            Err(error) => {
                stdout.push(format!(
                    "pyenv: cannot prepare the existing target for replacement: {error}"
                ));
                stdout.push("The source venv was left unchanged.".to_string());
                return CommandReport::failure(stdout, 1);
            }
        };
        if let Err(error) = rename_with_retry(&destination, &backup) {
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
    let backup_ref = backup_path.as_deref();

    let (mut new_info, _local_written, create_steps) =
        match create_managed_venv(ctx, &resolved_new_version, &info.name, false, false, false) {
            Ok(res) => res,
            Err(e) => {
                return abort_upgrade(
                    stdout,
                    &destination,
                    backup_ref,
                    Some(format!("pyenv: failed to create migration venv: {e}")),
                );
            }
        };
    stdout.extend(create_steps.into_iter().map(|s| format!("  - {s}")));

    let new_py_path = match &new_info.python_path {
        Some(p) if p.exists() => p.clone(),
        _ => {
            let reason = format!(
                "pyenv: new python interpreter was not found on disk at {:?}",
                new_info.python_path
            );
            return abort_upgrade(stdout, &destination, backup_ref, Some(reason));
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
            stdout.push(format!(
                "  - verification failed for the fresh environment: {exact_error}"
            ));
            return abort_upgrade(
                stdout,
                &destination,
                backup_ref,
                Some("pyenv: dependency verification failed.".to_string()),
            );
        }

        stdout.push(format!(
            "  - compatibility: exact package restore was not usable on runtime {}: {exact_error}",
            resolved_new_version
        ));
        if let Err(cleanup_error) = remove_dir_all_with_retry(&new_info.path) {
            let reason = format!(
                "pyenv: could not reset the migration staging environment: {cleanup_error}"
            );
            return abort_upgrade(stdout, &destination, backup_ref, Some(reason));
        }

        let (refreshed_info, _local_written, refresh_steps) = match create_managed_venv(
            ctx,
            &resolved_new_version,
            &info.name,
            false,
            false,
            false,
        ) {
            Ok(res) => res,
            Err(error) => {
                let reason = format!(
                    "pyenv: could not recreate the compatibility staging environment: {error}"
                );
                return abort_upgrade(stdout, &destination, backup_ref, Some(reason));
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
                let reason =
                    "pyenv: compatibility staging environment has no usable Python interpreter."
                        .to_string();
                return abort_upgrade(stdout, &destination, backup_ref, Some(reason));
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
            let reason = format!(
                "pyenv: compatible package restore and verification failed: {compatible_error}"
            );
            return abort_upgrade(stdout, &destination, backup_ref, Some(reason));
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
    stdout.push(format!(
        "  - finalize: activated migrated venv at {}",
        destination.display()
    ));

    // The migration itself has succeeded. Point selections at the new venv *before* cleanup so a
    // locked file on the old environment (common on Windows) can never leave `.python-version`
    // pointing at a stale spec.
    let final_spec = managed_venv_spec(&resolved_new_version, &info.name);
    retarget_selection_files(ctx, &info.spec, &final_spec, &mut stdout);

    let mut selection_error = None;
    if set_local {
        let report = cmd_local(ctx, std::slice::from_ref(&final_spec), false, true);
        if report.exit_code != 0 {
            stdout.extend(report.stderr);
            selection_error = Some(format!(
                "pyenv: migrated to {final_spec}, but writing .python-version failed."
            ));
        } else {
            stdout.push(format!(
                "  - selection: wrote local .python-version for {final_spec}"
            ));
        }
    }

    // Cleanup is best-effort: failures are reported as warnings, not as a failed migration.
    let mut warnings = Vec::new();
    stdout.push("  - cleanup: removing old virtual environment...".to_string());
    match remove_dir_all_with_retry(&info.path) {
        Ok(()) => stdout.push(format!(
            "    - removed old managed venv at {}",
            info.path.display()
        )),
        Err(error) => {
            warnings.push(format!(
                "the old venv at {} could not be fully removed ({error}); close anything using it (editors, shells, language servers) and run `pyenv venv delete {} --force`",
                info.path.display(),
                info.spec
            ));
        }
    }

    if let Some(backup) = backup_path {
        match remove_dir_all_with_retry(&backup) {
            Ok(()) => stdout.push(format!(
                "  - target: removed rollback copy {}",
                backup.display()
            )),
            Err(error) => warnings.push(format!(
                "the previous target rollback copy at {} could not be removed ({error}); delete it manually",
                backup.display()
            )),
        }
    }

    for warning in &warnings {
        stdout.push(format!("pyenv: warning: {warning}"));
    }

    if let Some(message) = selection_error {
        stdout.push(message);
        return CommandReport::failure(stdout, 1);
    }

    stdout.push(format!(
        "Managed venv successfully upgraded to: {final_spec}"
    ));
    CommandReport::success(stdout)
}

/// Roll back a failed upgrade: drop the partially built target and restore any replaced target.
/// The source venv is never touched before this point, so it is always left unchanged.
pub(super) fn abort_upgrade(
    mut stdout: Vec<String>,
    destination: &Path,
    backup: Option<&Path>,
    reason: Option<String>,
) -> CommandReport {
    if let Some(reason) = reason {
        stdout.push(reason);
    }
    if destination.exists()
        && let Err(error) = remove_dir_all_with_retry(destination)
    {
        stdout.push(format!(
            "pyenv: could not remove the partially built environment at {}: {error}",
            destination.display()
        ));
    }
    if let Some(backup) = backup {
        if destination.exists() {
            stdout.push(format!(
                "pyenv: the previous target venv was kept at {} because {} could not be cleared",
                backup.display(),
                destination.display()
            ));
        } else if let Err(error) = rename_with_retry(backup, destination) {
            stdout.push(format!(
                "pyenv: failed to restore the previous target venv from {} to {}: {error}",
                backup.display(),
                destination.display()
            ));
        } else {
            stdout.push("  - target: restored the previous target venv".to_string());
        }
    }
    stdout.push("The source venv was left unchanged.".to_string());
    CommandReport::failure(stdout, 1)
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
