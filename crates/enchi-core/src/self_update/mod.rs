// ./crates/enchi-core/src/self_update/mod.rs
//! Self-update helpers for upgrading portable Enchi installs in place.
//!
//! Updating downloads and verifies the release bundle for the target tag, then runs the bundled
//! `enchi self-install` (see the `installer` module) to swap the binaries.

mod bundle;
mod github;
mod runner;
mod tests;
mod types;
mod versioning;

pub use runner::{cmd_self_update, inspect_self_update};
pub use types::{SelfUpdateCheck, SelfUpdateCheckStatus, SelfUpdateOptions};
