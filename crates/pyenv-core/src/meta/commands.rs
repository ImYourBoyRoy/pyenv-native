// ./crates/pyenv-core/src/meta/commands.rs
//! Command inventory rendering for built-in and plugin-discovered command names.

use std::collections::BTreeSet;

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::plugin::discover_plugin_commands;

use super::compat_docs::COMPATIBILITY_COMMAND_DOCS;
use super::public_docs::PUBLIC_COMMAND_DOCS;

const SHELL_HELPER_COMMANDS: &[&str] = &["activate", "cmd", "deactivate", "rehash", "shell"];

pub fn cmd_commands(ctx: &AppContext, shell_only: bool, no_shell: bool) -> CommandReport {
    if shell_only && no_shell {
        return CommandReport::failure(
            vec!["pyenv: choose either `--sh` or `--no-sh`, not both".to_string()],
            1,
        );
    }

    let mut commands = BTreeSet::new();
    if shell_only {
        for name in SHELL_HELPER_COMMANDS {
            commands.insert((*name).to_string());
        }
    } else {
        for doc in PUBLIC_COMMAND_DOCS.iter() {
            commands.insert(doc.name.to_string());
        }
        for doc in COMPATIBILITY_COMMAND_DOCS {
            commands.insert(doc.name.to_string());
        }
    }

    for command in discover_plugin_commands(ctx) {
        let is_shell = command.starts_with("sh-");
        if shell_only && !is_shell {
            continue;
        }
        if no_shell && is_shell {
            continue;
        }

        let visible = command.strip_prefix("sh-").unwrap_or(&command).to_string();
        if !visible.is_empty() {
            commands.insert(visible);
        }
    }

    CommandReport::success(commands.into_iter().collect())
}
