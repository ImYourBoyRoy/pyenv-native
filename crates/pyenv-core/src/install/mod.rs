// ./crates/pyenv-core/src/install/mod.rs
//! Install planning, provider discovery, download/extraction, and runtime provisioning.

mod archive;
mod fetch;
mod plans;
mod platform;
mod providers;
mod report;
mod runtime;
mod runtime_support;
mod types;

pub use plans::{cmd_available, cmd_install, resolve_install_plan};
pub use types::{InstallCommandOptions, InstallOutcome, InstallPlan};

use crate::context::AppContext;
use crate::error::PyenvError;

use self::runtime::install_runtime;

pub fn install_runtime_plan(
    ctx: &AppContext,
    plan: &InstallPlan,
    force: bool,
) -> Result<InstallOutcome, PyenvError> {
    install_runtime(ctx, plan, force)
}

pub(crate) use providers::resolve_python_build_path;
#[cfg(test)]
mod tests;
