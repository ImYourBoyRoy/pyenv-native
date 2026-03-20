// ./crates/pyenv-core/src/plugin/mod.rs
//! Plugin command discovery, completion, hook execution, and process helpers.

mod commands;
mod discovery;
mod hooks;
mod process;
mod tests;
mod types;

pub use commands::{
    cmd_external, cmd_hooks, collect_rehash_hook_names, complete_plugin_command,
    discover_plugin_commands,
};
pub use discovery::find_plugin_command;
pub use hooks::{DEFAULT_HOOK_COMMANDS, parse_hook_actions, run_hook_scripts};
pub use types::HookResult;
