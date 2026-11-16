// ./crates/enchi-core/src/manage/mod.rs
//! Management commands for prefixes, installed-version listings, and uninstall operations.

mod commands;
mod helpers;
mod tests;
mod types;

pub use commands::{cmd_prefix, cmd_uninstall, cmd_versions, plan_uninstall};
pub use types::VersionsCommandOptions;
