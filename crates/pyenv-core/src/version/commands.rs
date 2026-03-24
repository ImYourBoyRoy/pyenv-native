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
        use std::io::IsTerminal;
        if !cfg!(test) && std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
            return prompt_interactive_selection(ctx, true);
        }
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
        use std::io::IsTerminal;
        if !cfg!(test) && std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
            return prompt_interactive_selection(ctx, false);
        }
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
        Ok(_) => {
            let scope =
                if path.file_name().and_then(|name| name.to_str()) == Some(".python-version") {
                    "locally"
                } else {
                    "globally"
                };
            let formatted_versions = versions.join(":");
            let verb = if formatted_versions.contains("venv:")
                || crate::venv::resolve_managed_venv(
                    ctx,
                    versions.first().unwrap_or(&String::new()),
                )
                .is_ok()
            {
                "activated"
            } else {
                "set"
            };
            CommandReport::success(vec![format!(
                "pyenv: {} {} {}",
                scope, verb, formatted_versions
            )])
        }
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

fn prompt_interactive_selection(ctx: &AppContext, is_global: bool) -> CommandReport {
    use std::io::Write;
    let mut stdout = std::io::stdout();

    let selected = resolve_selected_versions(ctx, false);
    let mut global_version = String::new();
    let mut local_version = String::new();

    for fallback in ["version", "global", "default"] {
        if let Ok(found) = read_version_file(&ctx.root.join(fallback)) {
            global_version = found.join(":");
            break;
        }
    }
    if let Some(local_path) = find_local_version_file(&ctx.dir)
        && let Ok(found) = read_version_file(&local_path)
    {
        local_version = found.join(":");
    }

    let _ = writeln!(stdout, "Current State:");
    let _ = writeln!(
        stdout,
        "  Global: {}",
        if global_version.is_empty() {
            "system"
        } else {
            &global_version
        }
    );
    let _ = writeln!(
        stdout,
        "  Local:  {}",
        if local_version.is_empty() {
            "(none)"
        } else {
            &local_version
        }
    );
    let _ = writeln!(
        stdout,
        "  Active: {} (set by {})\n",
        selected.versions.join(":"),
        selected.origin
    );

    let mut options = Vec::new();
    options.push("system".to_string());

    if let Ok(installed) = crate::catalog::installed_version_names(ctx) {
        options.extend(installed);
    }

    if let Ok(venvs) = crate::venv::list_managed_venvs(ctx) {
        let mut venv_names: Vec<_> = venvs.into_iter().map(|info| info.spec).collect();
        venv_names.sort_by_key(|name| name.to_ascii_lowercase());
        options.extend(venv_names);
    }

    options.push("[Install a new Python version]".to_string());
    options.push("[Create a new virtual environment]".to_string());

    let current_label = if is_global { "global" } else { "local" };
    let _ = writeln!(stdout, "Available Options:");
    for (i, v) in options.iter().enumerate() {
        let _ = writeln!(stdout, "  {}) {}", i + 1, v);
    }
    let _ = write!(
        stdout,
        "Select a version to set {} [1-{} or 'q' to quit]: ",
        current_label,
        options.len()
    );
    let _ = stdout.flush();

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return CommandReport::failure(
            vec!["pyenv: failed to read interactive input".to_string()],
            1,
        );
    }

    let input = input.trim();
    if input == "q" || input == "quit" || input.is_empty() {
        return CommandReport::failure(
            vec![format!("pyenv: {} selection cancelled", current_label)],
            1,
        );
    }

    match input.parse::<usize>() {
        Ok(idx) if idx > 0 && idx <= options.len() => {
            let selected_opt = options[idx - 1].clone();
            let _ = writeln!(stdout);

            if selected_opt == "[Install a new Python version]" {
                let options = crate::install::InstallCommandOptions {
                    list: false,
                    force: false,
                    dry_run: false,
                    json: false,
                    known: false,
                    family: None,
                    versions: vec![],
                };
                let report = crate::install::cmd_install(ctx, &options);
                if report.exit_code == 0 {
                    let _ = writeln!(
                        stdout,
                        "\nHint: Run `pyenv {}` again to select your newly installed version.",
                        current_label
                    );
                }
                return report;
            } else if selected_opt == "[Create a new virtual environment]" {
                return prompt_interactive_venv_create(ctx, is_global);
            }

            let versions = vec![selected_opt];
            if is_global {
                cmd_global(ctx, &versions, false)
            } else {
                cmd_local(ctx, &versions, false, false)
            }
        }
        _ => CommandReport::failure(vec!["pyenv: invalid selection".to_string()], 1),
    }
}

fn prompt_interactive_venv_create(ctx: &AppContext, is_global: bool) -> CommandReport {
    let installed = crate::catalog::installed_version_names(ctx).unwrap_or_default();
    if installed.is_empty() {
        return CommandReport::failure(
            vec![
                "pyenv: no Python versions installed to base a venv on. Please install one first."
                    .to_string(),
            ],
            1,
        );
    }
    use std::io::Write;
    let mut stdout = std::io::stdout();
    let _ = writeln!(
        stdout,
        "Select a base installed Python version for your new venv:"
    );
    for (i, v) in installed.iter().enumerate() {
        let _ = writeln!(stdout, "  {}) {}", i + 1, v);
    }
    let _ = write!(
        stdout,
        "Select base version [1-{} or 'q' to quit]: ",
        installed.len()
    );
    let _ = stdout.flush();
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return CommandReport::failure(vec!["pyenv: input failed".to_string()], 1);
    }
    let input = input.trim();
    if input == "q" || input == "quit" || input.is_empty() {
        return CommandReport::failure(vec!["pyenv: cancelled venv creation".to_string()], 1);
    }
    let base_version = match input.parse::<usize>() {
        Ok(idx) if idx > 0 && idx <= installed.len() => installed[idx - 1].clone(),
        _ => return CommandReport::failure(vec!["pyenv: invalid selection".to_string()], 1),
    };

    let _ = writeln!(stdout);
    let _ = write!(
        stdout,
        "Enter a name for your new virtual environment (e.g., 'my-project'): "
    );
    let _ = stdout.flush();
    let mut name = String::new();
    if std::io::stdin().read_line(&mut name).is_err() {
        return CommandReport::failure(vec!["pyenv: input failed".to_string()], 1);
    }
    let name = name.trim();
    if name.is_empty() {
        return CommandReport::failure(vec!["pyenv: venv name cannot be empty".to_string()], 1);
    }

    let _ = writeln!(
        stdout,
        "\nCreating managed venv '{}' hosted by {}...\n",
        name, base_version
    );

    let report = crate::venv::cmd_venv_create(ctx, &base_version, name, false, false);

    if report.exit_code == 0 {
        let _ = writeln!(stdout, "Successfully created managed venv {}\n", name);
        let venv_spec = format!("venv:{}", name);
        if is_global {
            return cmd_global(ctx, &[venv_spec], false);
        } else {
            return cmd_local(ctx, &[venv_spec], false, false);
        }
    }

    report
}
