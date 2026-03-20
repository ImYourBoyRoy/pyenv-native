// ./crates/pyenv-core/src/install/runtime/native.rs
//! Archive-based runtime installation flow used by prebuilt runtime providers.

use std::fs;

use crate::context::AppContext;
use crate::error::PyenvError;

use super::super::archive::{
    download_package, ensure_pip_wrappers, extract_archive, move_directory, validate_python,
};
use super::super::report::io_error;
use super::super::runtime_support::ensure_pip_available;
use super::super::types::{InstallOutcome, InstallPlan};
use super::shared::{
    ProgressTracker, cleanup_paths, create_base_venv_if_requested, finalize_install,
    remove_existing_install_dir, run_before_install_hooks, seed_progress, staging_dir,
    versions_dir,
};

pub(super) fn install_runtime_via_archive(
    ctx: &AppContext,
    plan: &InstallPlan,
    force: bool,
    on_progress: Option<&mut dyn FnMut(&str)>,
) -> Result<InstallOutcome, PyenvError> {
    remove_existing_install_dir(plan, force)?;
    run_before_install_hooks(ctx, plan)?;
    download_package(plan)?;

    let versions_dir = versions_dir(plan)?;
    fs::create_dir_all(versions_dir).map_err(io_error)?;
    let staging_dir = staging_dir(versions_dir, plan, "installing");
    let mut progress = ProgressTracker::new(on_progress);
    seed_progress(
        &mut progress,
        plan,
        format!("fetching package from {}", plan.download_url),
    );

    let outcome = (|| {
        extract_archive(plan, &staging_dir)?;
        progress.push(
            "extract",
            format!("unpacked archive into {}", staging_dir.display()),
        );
        move_directory(&staging_dir, &plan.install_dir)?;
        progress.push(
            "install",
            format!("moved runtime files into {}", plan.install_dir.display()),
        );
        validate_python(&plan.python_executable)?;
        progress.push(
            "verify",
            format!(
                "validated interpreter at {}",
                plan.python_executable.display()
            ),
        );

        let pip_bootstrapped = if plan.bootstrap_pip {
            progress.push("pip", "ensuring pip is available");
            let pip_available = ensure_pip_available(&plan.python_executable)?;
            if plan.provider.starts_with("windows-") {
                ensure_pip_wrappers(plan)?;
            }
            pip_available
        } else {
            false
        };

        let base_venv_created = create_base_venv_if_requested(plan, &mut progress)?;
        finalize_install(ctx, plan, pip_bootstrapped, base_venv_created, progress)
    })();

    if outcome.is_err() {
        cleanup_paths(&[staging_dir.as_path(), plan.install_dir.as_path()]);
    }

    outcome
}
