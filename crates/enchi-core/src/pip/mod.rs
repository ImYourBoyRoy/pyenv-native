// ./crates/enchi-core/src/pip/mod.rs
//! Module entrypoint for premium Pip package explorer and conflict-safe dependency management.
//!
//! Groups structures and functions under a central package boundary.

pub mod bulk;
pub mod operations;
pub mod types;

#[cfg(test)]
mod tests;

pub use bulk::{
    BulkScope, BulkSummary, BulkUpdateOptions, BulkUpdateReport, TargetKind, TargetResult,
    TargetStatus, cmd_pip_update_all, update_all_packages,
};
pub use operations::{
    cmd_pip_analyze_imports, cmd_pip_check, cmd_pip_install, cmd_pip_list, cmd_pip_outdated,
    cmd_pip_precheck_requirements, cmd_pip_update, plan_pip_install, plan_pip_update,
    resolve_interpreter_path,
};
pub use types::{DependencyConflict, OutdatedPackage, PipPackage, PrecheckResult};
