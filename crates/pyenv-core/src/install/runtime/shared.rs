// ./crates/pyenv-core/src/install/runtime/shared.rs
//! Shared install-flow helpers for hooks, staging paths, progress, and cleanup.

use std::fs;
use std::path::{Path, PathBuf};

use crate::context::AppContext;
use crate::error::PyenvError;
use crate::plugin::run_hook_scripts;
use crate::shim::rehash_shims;

use super::super::archive::{run_python, write_install_receipt};
use super::super::report::{io_error, progress_step, sanitize_for_fs, unique_suffix};
use super::super::types::{InstallOutcome, InstallPlan};

pub(super) fn remove_existing_install_dir(
    plan: &InstallPlan,
    force: bool,
) -> Result<(), PyenvError> {
    if !plan.install_dir.exists() {
        return Ok(());
    }

    if force {
        fs::remove_dir_all(&plan.install_dir).map_err(io_error)
    } else {
        Err(PyenvError::VersionAlreadyInstalled(
            plan.resolved_version.clone(),
        ))
    }
}

pub(super) fn run_before_install_hooks(
    ctx: &AppContext,
    plan: &InstallPlan,
) -> Result<(), PyenvError> {
    run_hook_scripts(ctx, "install", &hook_env(plan, "before")).map(|_| ())
}

pub(super) fn run_after_install_hooks(
    ctx: &AppContext,
    plan: &InstallPlan,
) -> Result<(), PyenvError> {
    run_hook_scripts(ctx, "install", &hook_env(plan, "after")).map(|_| ())
}

pub(super) fn versions_dir(plan: &InstallPlan) -> Result<&Path, PyenvError> {
    plan.install_dir
        .parent()
        .ok_or_else(|| PyenvError::Io("pyenv: invalid install directory".to_string()))
}

pub(super) fn staging_dir(versions_dir: &Path, plan: &InstallPlan, label: &str) -> PathBuf {
    versions_dir.join(format!(
        ".{label}-{}-{}",
        sanitize_for_fs(&plan.resolved_version),
        unique_suffix()
    ))
}

pub(super) fn initial_progress_steps(plan: &InstallPlan, detail: String) -> Vec<String> {
    vec![
        progress_step(
            "plan",
            format!(
                "resolved {} -> {} via {} [{}]",
                plan.requested_version, plan.resolved_version, plan.provider, plan.architecture
            ),
        ),
        progress_step("download", detail),
    ]
}

pub(super) fn create_base_venv_if_requested(
    plan: &InstallPlan,
    progress_steps: &mut Vec<String>,
) -> Result<bool, PyenvError> {
    let mut base_venv_created = false;
    if plan.create_base_venv
        && let Some(base_venv_path) = &plan.base_venv_path
    {
        progress_steps.push(progress_step(
            "venv",
            format!(
                "creating base virtual environment at {}",
                base_venv_path.display()
            ),
        ));
        let base_venv_arg = base_venv_path.display().to_string();
        run_python(
            &plan.python_executable,
            &["-m", "venv", base_venv_arg.as_str()],
        )?;
        base_venv_created = true;
    }
    Ok(base_venv_created)
}

pub(super) fn finalize_install(
    ctx: &AppContext,
    plan: &InstallPlan,
    pip_bootstrapped: bool,
    base_venv_created: bool,
    progress_steps: &mut Vec<String>,
) -> Result<InstallOutcome, PyenvError> {
    let receipt_path = write_install_receipt(plan)?;
    progress_steps.push(progress_step(
        "receipt",
        format!("wrote install receipt to {}", receipt_path.display()),
    ));
    rehash_shims(ctx)?;
    progress_steps.push(progress_step(
        "shims",
        format!("refreshed shims under {}", ctx.shims_dir().display()),
    ));
    run_after_install_hooks(ctx, plan)?;

    Ok(InstallOutcome {
        plan: plan.clone(),
        receipt_path,
        pip_bootstrapped,
        base_venv_created,
        progress_steps: progress_steps.clone(),
    })
}

pub(super) fn cleanup_paths(paths: &[&Path]) {
    for path in paths {
        let _ = if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        };
    }
}

fn hook_env(plan: &InstallPlan, stage: &str) -> [(&'static str, String); 5] {
    [
        ("PYENV_VERSION_NAME", plan.resolved_version.clone()),
        ("PYENV_VERSION", plan.resolved_version.clone()),
        ("PYENV_PREFIX", plan.install_dir.display().to_string()),
        ("PYENV_HOOK_STAGE", stage.to_string()),
        ("PYENV_INSTALL_PROVIDER", plan.provider.clone()),
    ]
}
