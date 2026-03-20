// ./crates/pyenv-core/src/install/runtime/python_build.rs
//! python-build installation flow for non-native runtime families and fallback backends.

use std::fs;

use crate::context::AppContext;
use crate::error::PyenvError;

use super::super::archive::validate_python;
use super::super::providers::resolve_python_build_path;
use super::super::report::{io_error, progress_step};
use super::super::runtime_support::{ensure_pip_available, run_python_build_install};
use super::super::types::{InstallOutcome, InstallPlan};
use super::shared::{
    cleanup_paths, create_base_venv_if_requested, finalize_install, remove_existing_install_dir,
    run_before_install_hooks,
};

pub(super) fn install_runtime_via_python_build(
    ctx: &AppContext,
    plan: &InstallPlan,
    force: bool,
) -> Result<InstallOutcome, PyenvError> {
    remove_existing_install_dir(plan, force)?;
    run_before_install_hooks(ctx, plan)?;

    let mut progress_steps = vec![
        progress_step(
            "plan",
            format!(
                "resolved {} -> {} via {} [{}]",
                plan.requested_version, plan.resolved_version, plan.provider, plan.architecture
            ),
        ),
        progress_step("backend", "resolving python-build backend"),
    ];

    let outcome = (|| {
        let python_build = resolve_python_build_path(ctx)?;
        progress_steps.push(progress_step(
            "backend",
            format!("using python-build backend at {}", python_build.display()),
        ));
        if let Some(parent) = plan.install_dir.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }

        progress_steps.push(progress_step(
            "build",
            format!(
                "building runtime {} into {}",
                plan.resolved_version,
                plan.install_dir.display()
            ),
        ));
        run_python_build_install(
            ctx,
            &python_build,
            &plan.resolved_version,
            &plan.install_dir,
        )?;
        validate_python(&plan.python_executable)?;
        progress_steps.push(progress_step(
            "verify",
            format!(
                "validated interpreter at {}",
                plan.python_executable.display()
            ),
        ));

        let pip_bootstrapped = if plan.bootstrap_pip {
            progress_steps.push(progress_step("pip", "ensuring pip is available"));
            ensure_pip_available(&plan.python_executable)?
        } else {
            false
        };

        let base_venv_created = create_base_venv_if_requested(plan, &mut progress_steps)?;
        finalize_install(
            ctx,
            plan,
            pip_bootstrapped,
            base_venv_created,
            &mut progress_steps,
        )
    })();

    if outcome.is_err() {
        cleanup_paths(&[plan.install_dir.as_path()]);
    }

    outcome
}
