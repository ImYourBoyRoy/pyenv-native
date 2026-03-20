// ./crates/pyenv-core/src/self_update/mod.rs
//! Self-update helpers for upgrading portable pyenv-native installs in place.

mod github;
mod runner;
mod tests;
mod types;
mod versioning;

pub use runner::cmd_self_update;
pub use types::SelfUpdateOptions;
