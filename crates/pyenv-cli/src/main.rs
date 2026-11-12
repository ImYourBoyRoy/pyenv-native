// ./crates/pyenv-cli/src/main.rs
//! CLI entrypoint for the native-first pyenv implementation. This binary keeps startup small
//! by delegating clap definitions to `cli.rs` and command execution to `dispatch.rs`.

mod cli;
mod dispatch;

use std::process::ExitCode;
use std::thread;

/// Windows gives the main thread a fixed 1 MiB stack, and the command dispatcher's frame (one
/// arm per subcommand) overflows it in unoptimised builds. Running on a thread with an explicit
/// stack makes every platform and build profile behave the same.
const STACK_BYTES: usize = 16 * 1024 * 1024;

fn main() -> ExitCode {
    let worker = thread::Builder::new()
        .name("pyenv".into())
        .stack_size(STACK_BYTES)
        .spawn(dispatch::run);
    match worker {
        Ok(handle) => match handle.join() {
            Ok(code) => code,
            // The panic message was already printed by the panic hook.
            Err(_) => ExitCode::from(101),
        },
        // Could not create the thread (extreme resource pressure): run in place.
        Err(_) => dispatch::run(),
    }
}
