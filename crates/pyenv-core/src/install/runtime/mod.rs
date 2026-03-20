// ./crates/pyenv-core/src/install/runtime/mod.rs
//! Runtime installation execution, validation, shim refresh, and base-venv creation.

mod native;
mod python_build;
mod shared;
mod source;

use crate::context::AppContext;
use crate::error::PyenvError;

use super::types::{InstallOutcome, InstallPlan};

pub(super) fn install_runtime(
    ctx: &AppContext,
    plan: &InstallPlan,
    force: bool,
) -> Result<InstallOutcome, PyenvError> {
    if plan.provider.ends_with("-cpython-source") {
        return source::install_runtime_via_cpython_source(ctx, plan, force);
    }

    if plan.provider.ends_with("-python-build") {
        return python_build::install_runtime_via_python_build(ctx, plan, force);
    }

    native::install_runtime_via_archive(ctx, plan, force)
}
