// ./crates/pyenv-core/src/meta/mod.rs
//! Command-surface helpers for parity-focused commands like help, commands, shims, and
//! completions. The public API stays stable while docs, completion logic, shim listing, and
//! plugin-help parsing live in focused submodules.

mod commands;
mod compat_docs;
mod completions;
mod docs;
mod help;
mod prompt;
mod public_docs;
mod shims;
mod status;
#[cfg(test)]
mod tests;

pub use self::commands::cmd_commands;
pub use self::completions::cmd_completions;
pub use self::help::cmd_help;
pub use self::prompt::cmd_prompt;
pub use self::shims::cmd_shims;
pub use self::status::{
    EnvironmentStatus, ManagedVenvSummary, build_environment_status, cmd_status,
};
