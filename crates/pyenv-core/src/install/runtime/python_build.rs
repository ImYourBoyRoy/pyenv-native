// ./crates/pyenv-core/src/install/runtime/python_build.rs
//! python-build installation flow for non-native runtime families and fallback backends.

use std::fs;

use crate::context::AppContext;
use crate::error::PyenvError;

use super::super::archive::{download_package, validate_python};
use super::super::checksum::{python_build_cache_is_cpython, verify_python_build_cache};
use super::super::providers::resolve_python_build_path;
use super::super::report::io_error;
use super::super::runtime_support::run_python_build_install;
use super::super::types::{DEFAULT_CPYTHON_SOURCE_BASE_URL, InstallOutcome, InstallPlan};
use super::shared::{
    ProgressTracker, bootstrap_pip_with_upgrade, cleanup_paths, create_base_venv_if_requested,
    finalize_install, remove_existing_install_dir, run_before_install_hooks,
};

pub(super) fn install_runtime_via_python_build(
    ctx: &AppContext,
    plan: &InstallPlan,
    force: bool,
    on_progress: Option<&mut dyn FnMut(&str)>,
) -> Result<InstallOutcome, PyenvError> {
    remove_existing_install_dir(plan, force)?;
    run_before_install_hooks(ctx, plan)?;

    let mut progress = ProgressTracker::new(on_progress);
    progress.push(
        "plan",
        format!(
            "resolved {} -> {} via {} [{}]",
            plan.requested_version, plan.resolved_version, plan.provider, plan.architecture
        ),
    );
    progress.push("backend", "resolving python-build backend");

    let outcome = (|| {
        let python_build = resolve_python_build_path(ctx)?;
        progress.push(
            "backend",
            format!("using python-build backend at {}", python_build.display()),
        );
        if let Some(parent) = plan.install_dir.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }

        if python_build_cache_is_cpython(plan) {
            progress.push(
                "verify",
                "prefetching official CPython source into PYTHON_BUILD_CACHE_PATH",
            );
            prefetch_cpython_source_for_python_build(ctx, plan, &mut progress)?;
        }

        progress.push(
            "build",
            format!(
                "building runtime {} into {} (this can take several minutes on slower systems)",
                plan.resolved_version,
                plan.install_dir.display()
            ),
        );
        run_python_build_install(
            ctx,
            &python_build,
            &plan.resolved_version,
            &plan.install_dir,
        )?;
        match verify_python_build_cache(ctx, plan) {
            Ok(Some(digest)) => progress.push(
                "verify",
                format!("verified python-build cache tarball via {}", digest.source),
            ),
            Ok(None) if python_build_cache_is_cpython(plan) => {
                return Err(PyenvError::MissingChecksum(format!(
                    "python-build CPython {} left no verifiable tarball in PYTHON_BUILD_CACHE_PATH",
                    plan.resolved_version
                )));
            }
            Ok(None) => progress.push(
                "verify",
                "python-build cache tarball not found; publisher checksum not applied for this non-CPython definition",
            ),
            Err(error) => return Err(error),
        }
        validate_python(&plan.python_executable)?;
        progress.push(
            "verify",
            format!(
                "validated interpreter at {}",
                plan.python_executable.display()
            ),
        );

        let pip_bootstrapped = bootstrap_pip_with_upgrade(plan, &mut progress)?;

        let base_venv_created = create_base_venv_if_requested(plan, &mut progress)?;
        finalize_install(ctx, plan, pip_bootstrapped, base_venv_created, progress)
    })();

    if outcome.is_err() {
        cleanup_paths(&[plan.install_dir.as_path()]);
    }

    outcome
}

fn prefetch_cpython_source_for_python_build(
    ctx: &AppContext,
    plan: &InstallPlan,
    progress: &mut ProgressTracker<'_>,
) -> Result<(), PyenvError> {
    let cache_dir = ctx.cache_dir().join("python-build");
    fs::create_dir_all(&cache_dir).map_err(io_error)?;
    let version = plan.resolved_version.trim();
    let names = [
        format!("Python-{version}.tar.xz"),
        format!("Python-{version}.tgz"),
        format!("Python-{version}.tar.bz2"),
    ];
    let mut last_error = None;
    for name in names {
        let mut probe = plan.clone();
        probe.provider = format!("{}-cpython-source", plan.architecture);
        probe.package_name = name.clone();
        probe.package_version = version.to_string();
        probe.cache_path = cache_dir.join(&name);
        probe.download_url = format!("{DEFAULT_CPYTHON_SOURCE_BASE_URL}/{version}/{name}");
        let mut download_progress = |step: String| {
            progress.push("download", step);
        };
        match download_package(ctx, &probe, Some(&mut download_progress)) {
            Ok(()) => {
                progress.push("verify", format!("cached verified {name} for python-build"));
                return Ok(());
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        PyenvError::MissingChecksum(format!(
            "python-build CPython {version} could not prefetch a publisher-checksummed source archive"
        ))
    }))
}
