// ./crates/pyenv-core/src/shim/exec.rs
//! Shim-backed executable dispatch for selected runtimes and managed envs.

use std::path::PathBuf;
use std::process::Command;

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::executable::{find_command_in_version, find_system_command};
use crate::plugin::{parse_hook_actions, run_hook_scripts};
use crate::runtime::{managed_search_roots_for_version, prefix_bin_dirs};
use crate::version::resolve_selected_versions;

use super::paths::adjusted_path;
use super::types::ExecTarget;

pub fn cmd_exec(ctx: &AppContext, command: &str, args: &[String]) -> CommandReport {
    let target = match resolve_exec_target(ctx, command) {
        Ok(target) => target,
        Err(report) => return report,
    };

    let origin = crate::version::version_origin(ctx).to_string();
    let selected = resolve_selected_versions(ctx, false);
    let selected_value = selected.versions.join(":");
    let hook_results = match run_hook_scripts(
        ctx,
        "exec",
        &[
            ("PYENV_COMMAND", command.to_string()),
            (
                "PYENV_COMMAND_PATH",
                target.executable.display().to_string(),
            ),
            ("PYENV_VERSION", selected_value),
            ("PYENV_VERSION_ORIGIN", origin),
            (
                "PYENV_VERSION_RESOLVED",
                target
                    .version_name
                    .clone()
                    .unwrap_or_else(|| "system".to_string()),
            ),
        ],
    ) {
        Ok(results) => results,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };
    let hook_actions = parse_hook_actions(
        &hook_results
            .into_iter()
            .flat_map(|result| result.stdout)
            .collect::<Vec<_>>(),
    );

    let executable = hook_actions.command_path.unwrap_or(target.executable);
    let mut prefix_dirs = hook_actions.prepend_paths;
    prefix_dirs.extend(target.prefix_dirs);

    let mut child = Command::new(&executable);
    child.args(args);
    child.current_dir(&ctx.dir);
    child.env("PYENV_COMMAND", command);

    if let Some(path) = adjusted_path(ctx, &prefix_dirs) {
        child.env("PATH", path);
    }

    for (key, value) in hook_actions.env_pairs {
        child.env(key, value);
    }

    match child.status() {
        Ok(status) => CommandReport {
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit_code: status.code().unwrap_or(1),
        },
        Err(error) => CommandReport::failure(
            vec![format!(
                "pyenv: failed to execute {}: {error}",
                executable.display()
            )],
            1,
        ),
    }
}

fn resolve_exec_target(ctx: &AppContext, command: &str) -> Result<ExecTarget, CommandReport> {
    let selected = resolve_selected_versions(ctx, false);
    let origin = selected.origin.to_string();
    let mut searched_system = false;

    for version in &selected.versions {
        if version == "system" {
            searched_system = true;
            if let Some(path) = find_system_command(ctx, command) {
                return Ok(ExecTarget {
                    executable: path,
                    prefix_dirs: Vec::new(),
                    version_name: Some("system".to_string()),
                });
            }
            continue;
        }

        if let Some(path) = find_command_in_version(ctx, version, command) {
            return Ok(ExecTarget {
                executable: path,
                prefix_dirs: managed_search_roots_for_version(ctx, version)
                    .into_iter()
                    .flat_map(|prefix| prefix_bin_dirs(&prefix))
                    .collect::<Vec<PathBuf>>(),
                version_name: Some(version.clone()),
            });
        }
    }

    if !searched_system && let Some(path) = find_system_command(ctx, command) {
        return Ok(ExecTarget {
            executable: path,
            prefix_dirs: Vec::new(),
            version_name: Some("system".to_string()),
        });
    }

    let mut stderr = selected
        .missing
        .into_iter()
        .map(|version| format!("pyenv: version `{version}' is not installed (set by {origin})"))
        .collect::<Vec<_>>();
    stderr.push(format!("pyenv: {command}: command not found"));
    Err(CommandReport::failure(stderr, 127))
}
