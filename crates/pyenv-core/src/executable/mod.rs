// ./crates/pyenv-core/src/executable/mod.rs
//! Executable discovery for `which` and `whence` across managed runtimes and the system path.

mod commands;
mod lookup;
mod tests;

pub use commands::{cmd_whence, cmd_which};
pub(crate) use lookup::{find_command_in_version, find_system_command, find_system_python_command};
