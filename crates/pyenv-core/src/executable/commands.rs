// ./crates/pyenv-core/src/executable/commands.rs
//! Public `which` and `whence` command handlers with hook support and helpful fallback advice.

use crate::catalog::installed_version_names;
use crate::command::CommandReport;
use crate::context::AppContext;
use crate::plugin::{parse_hook_actions, run_hook_scripts};
use crate::version::resolve_selected_versions;

use super::lookup::{find_command_in_version, find_system_command};

pub fn cmd_which(
    ctx: &AppContext,
    command: &str,
    no_system: bool,
    skip_advice: bool,
) -> CommandReport {
    let selected = resolve_selected_versions(ctx, false);
    let origin = selected.origin.to_string();
    let mut searched_system = false;
    let selected_value = selected.versions.join(":");
    let mut resolved_version_name = None;
    let mut found_path = None;

    for version in &selected.versions {
        if version == "system" {
            if no_system {
                continue;
            }

            searched_system = true;
            if let Some(path) = find_system_command(ctx, command) {
                resolved_version_name = Some("system".to_string());
                found_path = Some(path);
                break;
            }
            continue;
        }

        if let Some(path) = find_command_in_version(ctx, version, command) {
            resolved_version_name = Some(version.clone());
            found_path = Some(path);
            break;
        }
    }

    if found_path.is_none()
        && !no_system
        && !searched_system
        && let Some(path) = find_system_command(ctx, command)
    {
        resolved_version_name = Some("system".to_string());
        found_path = Some(path);
    }

    let hook_results = match run_hook_scripts(
        ctx,
        "which",
        &[
            ("PYENV_COMMAND", command.to_string()),
            (
                "PYENV_COMMAND_PATH",
                found_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
            ),
            ("PYENV_VERSION", selected_value),
            (
                "PYENV_VERSION_RESOLVED",
                resolved_version_name
                    .clone()
                    .unwrap_or_else(|| "system".to_string()),
            ),
            ("PYENV_VERSION_ORIGIN", origin.clone()),
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
    if hook_actions.command_path.is_some() {
        found_path = hook_actions.command_path;
    }

    if let Some(path) = found_path.as_ref().filter(|path| path.is_file()) {
        return CommandReport::success_one(path.display().to_string());
    }

    let mut stderr = selected
        .missing
        .into_iter()
        .map(|version| format!("pyenv: version `{version}' is not installed (set by {origin})"))
        .collect::<Vec<_>>();
    stderr.push(format!("pyenv: {command}: command not found"));

    if !skip_advice {
        let advice = collect_whence(ctx, command, false);
        if !advice.is_empty() {
            stderr.push(String::new());
            stderr.push(format!(
                "The `{command}' command exists in these Python versions:"
            ));
            stderr.extend(advice.into_iter().map(|version| format!("  {version}")));
            stderr.push(String::new());
            stderr.push("Note: See 'pyenv help global' for tips on allowing both".to_string());
            stderr.push("      python2 and python3 to be found.".to_string());
        }
    }

    CommandReport::failure(stderr, 127)
}

pub fn cmd_whence(ctx: &AppContext, command: &str, print_paths: bool) -> CommandReport {
    let matches = collect_whence(ctx, command, print_paths);
    if matches.is_empty() {
        CommandReport::failure(Vec::new(), 1)
    } else {
        CommandReport::success(matches)
    }
}

fn collect_whence(ctx: &AppContext, command: &str, print_paths: bool) -> Vec<String> {
    let versions = match installed_version_names(ctx) {
        Ok(versions) => versions,
        Err(_) => return Vec::new(),
    };

    versions
        .into_iter()
        .filter_map(|version| {
            let path = find_command_in_version(ctx, &version, command)?;
            Some(if print_paths {
                path.display().to_string()
            } else {
                version
            })
        })
        .collect()
}
