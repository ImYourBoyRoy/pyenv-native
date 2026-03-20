// ./crates/pyenv-core/src/plugin/commands.rs
//! Public plugin command entrypoints plus completion helpers for plugin-backed commands.

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::error::PyenvError;

use super::discovery::{discover_hook_scripts, find_plugin_command};
use super::hooks::{DEFAULT_HOOK_COMMANDS, parse_hook_actions, run_hook_scripts};
use super::process::run_process;

pub fn cmd_hooks(ctx: &AppContext, hook: &str) -> CommandReport {
    if hook == "--complete" {
        return CommandReport::success(
            DEFAULT_HOOK_COMMANDS
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
        );
    }

    match discover_hook_scripts(ctx, hook) {
        Ok(scripts) => CommandReport::success(
            scripts
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
        ),
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

pub fn cmd_external(ctx: &AppContext, args: &[String]) -> CommandReport {
    let Some((command, rest)) = args.split_first() else {
        return CommandReport::failure(vec!["pyenv: no external command specified".to_string()], 1);
    };

    let Some(command_path) = find_plugin_command(ctx, command) else {
        return CommandReport::failure(vec![format!("pyenv: no such command `{command}`")], 1);
    };

    match run_process(&command_path, rest, ctx, &[], false) {
        Ok((exit_code, _, _)) => CommandReport {
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit_code,
        },
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

pub fn complete_plugin_command(
    ctx: &AppContext,
    command: &str,
    args: &[String],
) -> Result<Option<Vec<String>>, PyenvError> {
    let Some(command_path) = find_plugin_command(ctx, command) else {
        return Ok(None);
    };

    let mut completion_args = vec!["--complete".to_string()];
    completion_args.extend(args.iter().cloned());
    let (exit_code, stdout, stderr) = run_process(&command_path, &completion_args, ctx, &[], true)?;
    if exit_code != 0 {
        let detail = if !stderr.is_empty() {
            stderr.join("\n")
        } else {
            format!("exit code {exit_code}")
        };
        return Err(PyenvError::Io(format!(
            "pyenv: completion failed for {}: {detail}",
            command_path.display()
        )));
    }

    Ok(Some(
        stdout
            .into_iter()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect(),
    ))
}

pub fn collect_rehash_hook_names(
    ctx: &AppContext,
    extra_env: &[(&str, String)],
) -> Result<Vec<String>, PyenvError> {
    let mut names = std::collections::HashSet::new();
    for result in run_hook_scripts(ctx, "rehash", extra_env)? {
        for line in parse_hook_actions(&result.stdout).passthrough_lines {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                names.insert(trimmed.to_string());
            }
        }
    }
    let mut values = names.into_iter().collect::<Vec<_>>();
    values.sort_by_key(|value| value.to_ascii_lowercase());
    Ok(values)
}

pub fn discover_plugin_commands(ctx: &AppContext) -> Vec<String> {
    super::discovery::discover_plugin_commands(ctx)
}
