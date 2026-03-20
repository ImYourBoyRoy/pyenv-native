// ./crates/pyenv-cli/src/main.rs
//! CLI entrypoint for the native-first pyenv implementation. This binary keeps startup small
//! by delegating clap definitions to `cli.rs` and command execution to `dispatch.rs`.

mod cli;
mod dispatch;

use std::process::ExitCode;

fn main() -> ExitCode {
    dispatch::run()
}
