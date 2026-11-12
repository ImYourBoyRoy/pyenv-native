// ./crates/pyenv-core/src/installer/commands.rs
//! `CommandReport` adapters for install/uninstall, for hosts that collect output (GUI, MCP, tests).

use crate::command::CommandReport;
use crate::context::AppContext;

use super::install::{SelfInstallOptions, run_self_install};
use super::uninstall::{SelfUninstallOptions, run_self_uninstall};

/// Run an install and collect its output. The CLI streams instead; see `run_self_install`.
pub fn cmd_self_install(options: &SelfInstallOptions) -> CommandReport {
    let mut lines = Vec::new();
    let mut sink = |line: &str| lines.push(line.to_string());
    match run_self_install(options, &mut sink) {
        Ok(_) => CommandReport::success(lines),
        Err(message) => {
            let mut stderr = lines;
            stderr.push(message);
            CommandReport::failure(stderr, 1)
        }
    }
}

/// Remove pyenv-native (and its data) from the context's root.
pub fn cmd_self_uninstall(ctx: &AppContext, yes: bool) -> CommandReport {
    let mut options = SelfUninstallOptions::new(ctx.root.clone());
    options.yes = yes;
    cmd_self_uninstall_with(&options)
}

/// Run an uninstall with explicit options and collect its output.
pub fn cmd_self_uninstall_with(options: &SelfUninstallOptions) -> CommandReport {
    let mut lines = Vec::new();
    let mut sink = |line: &str| lines.push(line.to_string());
    match run_self_uninstall(options, &mut sink) {
        Ok(()) => CommandReport::success(lines),
        Err(message) => CommandReport::failure(vec![message], 1),
    }
}
