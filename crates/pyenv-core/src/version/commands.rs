// ./crates/pyenv-core/src/version/commands.rs
//! Public version-related command implementations for reading/writing version files and
//! rendering the active selection/origin state.

use std::fs;
use std::path::Path;

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::plugin::{parse_hook_actions, run_hook_scripts};

use super::files::{
    find_local_version_file, parse_version_file, read_version_file, version_file_path,
    write_version_file,
};
use super::selection::{ensure_versions_exist, resolve_selected_versions, version_origin};
use super::types::{GLOBAL_VERSION_FILE, LOCAL_VERSION_FILE};

pub fn cmd_root(ctx: &AppContext) -> CommandReport {
    CommandReport::success_one(ctx.root.display().to_string())
}

pub fn cmd_version_file(ctx: &AppContext, target_dir: Option<&Path>) -> CommandReport {
    CommandReport::success_one(version_file_path(ctx, target_dir).display().to_string())
}

pub fn cmd_version_file_write(
    ctx: &AppContext,
    path: &Path,
    versions: &[String],
    force: bool,
) -> CommandReport {
    if versions.is_empty() {
        return CommandReport::failure(
            vec!["Usage: pyenv version-file-write [-f|--force] <file> <version> [...]".to_string()],
            1,
        );
    }

    match ensure_versions_exist(ctx, versions, force, &path.display().to_string())
        .and_then(|_| write_version_file(path, versions))
    {
        Ok(_) => CommandReport::empty_success(),
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

pub fn cmd_version_file_read(path: &Path) -> CommandReport {
    match parse_version_file(path) {
        Ok(parsed) => CommandReport {
            stdout: vec![parsed.versions.join(":")],
            stderr: render_nonempty_errors(parsed.warnings),
            exit_code: 0,
        },
        Err(errors) => CommandReport {
            stdout: Vec::new(),
            stderr: render_nonempty_errors(errors),
            exit_code: 1,
        },
    }
}

pub fn cmd_version_origin(ctx: &AppContext) -> CommandReport {
    let default_origin = version_origin(ctx).to_string();
    let hook_results = match run_hook_scripts(
        ctx,
        "version-origin",
        &[("PYENV_VERSION_ORIGIN", default_origin.clone())],
    ) {
        Ok(results) => results,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };
    let actions = parse_hook_actions(
        &hook_results
            .into_iter()
            .flat_map(|result| result.stdout)
            .collect::<Vec<_>>(),
    );
    let origin = actions
        .env_pairs
        .into_iter()
        .find_map(|(key, value)| {
            key.eq_ignore_ascii_case("PYENV_VERSION_ORIGIN")
                .then_some(value)
        })
        .or_else(|| {
            actions
                .passthrough_lines
                .into_iter()
                .find(|line| !line.is_empty())
        })
        .unwrap_or(default_origin);
    CommandReport::success_one(origin)
}

pub fn cmd_version_name(ctx: &AppContext, force: bool) -> CommandReport {
    let mut selected = resolve_selected_versions(ctx, force);
    let origin = selected.origin.to_string();
    let selected_value = selected.versions.join(":");
    let hook_results = match run_hook_scripts(
        ctx,
        "version-name",
        &[
            ("PYENV_VERSION", selected_value.clone()),
            ("PYENV_VERSION_ORIGIN", origin.clone()),
        ],
    ) {
        Ok(results) => results,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };
    let actions = parse_hook_actions(
        &hook_results
            .into_iter()
            .flat_map(|result| result.stdout)
            .collect::<Vec<_>>(),
    );
    if let Some(overridden) = actions
        .env_pairs
        .into_iter()
        .find_map(|(key, value)| key.eq_ignore_ascii_case("PYENV_VERSION").then_some(value))
        .or_else(|| {
            actions
                .passthrough_lines
                .into_iter()
                .find(|line| !line.is_empty())
        })
    {
        let raw = overridden
            .split(':')
            .flat_map(|segment| segment.split_whitespace())
            .filter(|segment| !segment.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if !raw.is_empty() {
            selected.versions = raw;
            selected.missing.clear();
        }
    }

    let stderr = selected
        .missing
        .iter()
        .map(|version| PyenvError::VersionNotInstalled(version.clone(), origin.clone()).to_string())
        .collect::<Vec<_>>();

    CommandReport {
        stdout: vec![selected.versions.join(":")],
        stderr,
        exit_code: if selected.missing.is_empty() { 0 } else { 1 },
    }
}

pub fn cmd_version(ctx: &AppContext, bare: bool) -> CommandReport {
    let selected = resolve_selected_versions(ctx, false);
    let origin = selected.origin.to_string();
    let stderr = selected
        .missing
        .iter()
        .map(|version| PyenvError::VersionNotInstalled(version.clone(), origin.clone()).to_string())
        .collect::<Vec<_>>();

    let stdout = if bare {
        selected.versions.clone()
    } else {
        selected
            .versions
            .iter()
            .map(|version| format!("{version} (set by {origin})"))
            .collect()
    };

    CommandReport {
        stdout,
        stderr,
        exit_code: if selected.missing.is_empty() { 0 } else { 1 },
    }
}

pub fn cmd_global(ctx: &AppContext, versions: &[String], unset: bool) -> CommandReport {
    let path = ctx.root.join(GLOBAL_VERSION_FILE);

    if unset {
        remove_version_file(&path)
    } else if versions.is_empty() {
        show_global_versions(ctx)
    } else {
        write_requested_versions(ctx, &path, versions, false)
    }
}

pub fn cmd_local(ctx: &AppContext, versions: &[String], unset: bool, force: bool) -> CommandReport {
    let path = ctx.dir.join(LOCAL_VERSION_FILE);

    if unset {
        remove_version_file(&path)
    } else if versions.is_empty() {
        show_local_versions(ctx)
    } else {
        write_requested_versions(ctx, &path, versions, force)
    }
}

fn render_nonempty_errors(errors: Vec<PyenvError>) -> Vec<String> {
    errors
        .into_iter()
        .filter_map(|error| {
            let message = error.to_string();
            if message.is_empty() {
                None
            } else {
                Some(message)
            }
        })
        .collect()
}

fn remove_version_file(path: &Path) -> CommandReport {
    match fs::remove_file(path) {
        Ok(_) => CommandReport::empty_success(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            CommandReport::empty_success()
        }
        Err(error) => CommandReport::failure(vec![format!("pyenv: {error}")], 1),
    }
}

fn show_global_versions(ctx: &AppContext) -> CommandReport {
    for fallback in ["version", "global", "default"] {
        let candidate = ctx.root.join(fallback);
        if let Ok(found_versions) = read_version_file(&candidate) {
            return CommandReport::success(found_versions);
        }
    }

    CommandReport::success(vec!["system".to_string()])
}

fn show_local_versions(ctx: &AppContext) -> CommandReport {
    if let Some(local_path) = find_local_version_file(&ctx.dir) {
        cmd_version_file_read(&local_path)
    } else {
        CommandReport::failure(vec![PyenvError::NoLocalVersion.to_string()], 1)
    }
}

fn write_requested_versions(
    ctx: &AppContext,
    path: &Path,
    versions: &[String],
    force: bool,
) -> CommandReport {
    match ensure_versions_exist(ctx, versions, force, &path.display().to_string())
        .and_then(|_| write_version_file(path, versions))
    {
        Ok(_) => CommandReport::empty_success(),
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}
