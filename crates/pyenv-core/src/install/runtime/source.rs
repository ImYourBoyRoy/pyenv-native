// ./crates/pyenv-core/src/install/runtime/source.rs
//! Source-build installation flow for native CPython source providers.

use std::fs;

use crate::context::AppContext;
use crate::error::PyenvError;

use super::super::archive::{download_package, extract_archive, validate_python};
use super::super::report::io_error;
use super::super::runtime_support::{
    build_cpython_source_install, ensure_pip_available, ensure_unix_runtime_aliases,
};
use super::super::types::{InstallOutcome, InstallPlan};
use super::shared::{
    ProgressTracker, cleanup_paths, create_base_venv_if_requested, finalize_install,
    remove_existing_install_dir, run_before_install_hooks, seed_progress, staging_dir,
    versions_dir,
};

pub(super) fn install_runtime_via_cpython_source(
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
    let source_dir = staging_dir(versions_dir, plan, "building-source");
    let build_dir = staging_dir(versions_dir, plan, "building-work");
    let mut progress = ProgressTracker::new(on_progress);
    seed_progress(
        &mut progress,
        plan,
        format!("fetching source archive from {}", plan.download_url),
    );

    let outcome = (|| {
        extract_archive(plan, &source_dir)?;
        progress.push(
            "extract",
            format!("unpacked source archive into {}", source_dir.display()),
        );
        fs::create_dir_all(&build_dir).map_err(io_error)?;
        progress.push(
            "workspace",
            format!("created build workspace at {}", build_dir.display()),
        );
        progress.push(
            "build",
            format!(
                "configuring and compiling source for {} (this can take several minutes on macOS/Linux/Android)",
                plan.resolved_version
            ),
        );
        build_cpython_source_install(plan, &source_dir, &build_dir)?;
        progress.push(
            "install",
            format!(
                "installed compiled runtime into {}",
                plan.install_dir.display()
            ),
        );
        ensure_unix_runtime_aliases(&plan.install_dir, &plan.runtime_version)?;
        progress.push(
            "aliases",
            "ensured python/pip aliases exist in the runtime bin directory",
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
            ensure_pip_available(&plan.python_executable)?
        } else {
            false
        };

        let base_venv_created = create_base_venv_if_requested(plan, &mut progress)?;
        finalize_install(ctx, plan, pip_bootstrapped, base_venv_created, progress)
    })();

    cleanup_paths(&[source_dir.as_path(), build_dir.as_path()]);
    if outcome.is_err() {
        cleanup_paths(&[plan.install_dir.as_path()]);
    }

    outcome
}
