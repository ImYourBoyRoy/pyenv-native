// ./crates/pyenv-core/src/meta/completions.rs
//! Completion helpers for built-in commands and plugin completion passthrough.

use std::collections::BTreeSet;

use crate::catalog::{VersionFamily, installed_version_names, known_version_names};
use crate::command::CommandReport;
use crate::context::AppContext;
use crate::plugin::{DEFAULT_HOOK_COMMANDS, complete_plugin_command, discover_plugin_commands};
use crate::venv::list_managed_venvs;

use super::compat_docs::COMPATIBILITY_COMMAND_DOCS;
use super::docs::command_doc;
use super::public_docs::PUBLIC_COMMAND_DOCS;
use super::shims::cmd_shims;

pub fn cmd_completions(ctx: &AppContext, command: &str, args: &[String]) -> CommandReport {
    let requested = command.trim();
    if requested.is_empty() {
        return CommandReport::failure(
            vec!["Usage: pyenv completions <command> [arg1 arg2...]".to_string()],
            1,
        );
    }

    if requested == "--complete" {
        return super::commands::cmd_commands(ctx, false, false);
    }

    let mut values = BTreeSet::new();
    values.insert("--help".to_string());

    if let Some(doc) = command_doc(requested) {
        values.extend(doc.completions.iter().map(|value| (*value).to_string()));
        values.extend(dynamic_builtin_completions(ctx, requested, args));
    } else if let Some(plugin_values) = match complete_plugin_command(ctx, requested, args) {
        Ok(values) => values,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    } {
        values.extend(plugin_values);
    } else {
        return CommandReport::failure(vec![format!("pyenv: no such command `{requested}`")], 1);
    }

    CommandReport::success(values.into_iter().collect())
}

fn dynamic_builtin_completions(ctx: &AppContext, command: &str, args: &[String]) -> Vec<String> {
    match command {
        "help" | "commands" => PUBLIC_COMMAND_DOCS
            .iter()
            .chain(COMPATIBILITY_COMMAND_DOCS.iter())
            .map(|doc| doc.name.to_string())
            .chain(
                discover_plugin_commands(ctx)
                    .into_iter()
                    .map(|name| name.strip_prefix("sh-").unwrap_or(&name).to_string()),
            )
            .collect(),
        "activate" | "global" | "local" | "prefix" | "shell" | "uninstall"
        | "virtualenv-delete" | "virtualenv-prefix" => {
            let mut values = installed_version_names(ctx).unwrap_or_default();
            values.extend(
                list_managed_venvs(ctx)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|info| info.spec),
            );
            if matches!(command, "global" | "prefix" | "shell") {
                values.push("system".to_string());
            }
            values
        }
        "latest" => known_version_names().to_vec(),
        "hooks" => DEFAULT_HOOK_COMMANDS
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        "install" | "available" => {
            if args
                .iter()
                .rev()
                .find(|value| !value.trim().is_empty())
                .is_some_and(|value| value == "--family")
            {
                return known_family_slugs();
            }

            let mut values = known_family_slugs();
            values.extend(known_version_names().iter().cloned());
            values
        }
        "venv" | "virtualenv" | "virtualenvs" => vec![
            "list".to_string(),
            "info".to_string(),
            "create".to_string(),
            "delete".to_string(),
            "rename".to_string(),
            "use".to_string(),
            "--bare".to_string(),
            "--json".to_string(),
            "--force".to_string(),
            "--set-local".to_string(),
            "--global".to_string(),
        ],
        "which" | "whence" | "exec" => cmd_shims(ctx, true).stdout,
        _ => Vec::new(),
    }
}

fn known_family_slugs() -> Vec<String> {
    let mut families = BTreeSet::new();
    for version in known_version_names() {
        families.insert(VersionFamily::classify(version).slug());
    }
    families.into_iter().collect()
}
