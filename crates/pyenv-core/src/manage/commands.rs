// ./crates/pyenv-core/src/manage/commands.rs
//! Public management commands for prefix lookup, version listing, and uninstall flows.

use std::collections::HashSet;
use std::fs;

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::plugin::run_hook_scripts;
use crate::runtime::{collect_shim_names_from_prefix, inventory_roots_for_version};
use crate::shim::rehash_shims;
use crate::venv_paths::managed_venv_entries_for_base;
use crate::version::{installed_version_dir, resolve_selected_versions};

use super::helpers::{
    confirm_uninstall, current_version_origin, join_prefixes, list_version_entries,
    render_version_line, resolve_prefix_path, system_prefix,
};
use super::types::VersionsCommandOptions;

pub fn cmd_prefix(ctx: &AppContext, versions: &[String]) -> CommandReport {
    let requested = if versions.is_empty() {
        let selected = resolve_selected_versions(ctx, false);
        if !selected.missing.is_empty() {
            let origin = selected.origin.to_string();
            let stderr = selected
                .missing
                .into_iter()
                .map(|version| PyenvError::VersionNotInstalled(version, origin.clone()).to_string())
                .collect();
            return CommandReport::failure(stderr, 1);
        }
        selected.versions
    } else {
        versions.to_vec()
    };

    let mut prefixes = Vec::new();
    for requested_version in requested {
        match resolve_prefix_path(ctx, &requested_version) {
            Ok(path) => prefixes.push(path),
            Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
        }
    }

    CommandReport::success_one(join_prefixes(&prefixes))
}

pub fn cmd_versions(ctx: &AppContext, options: &VersionsCommandOptions) -> CommandReport {
    if options.executables {
        return cmd_versions_executables(ctx);
    }

    let entries = match list_version_entries(ctx, options.skip_aliases, options.skip_envs) {
        Ok(entries) => entries,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };

    if options.bare {
        return CommandReport::success(entries.into_iter().map(|entry| entry.name).collect());
    }

    let origin = current_version_origin(ctx);
    let current = resolve_selected_versions(ctx, false)
        .versions
        .into_iter()
        .map(|value| value.to_ascii_lowercase())
        .collect::<HashSet<_>>();

    let mut stdout = Vec::new();
    let mut num_versions = 0usize;

    if system_prefix(ctx).is_some() {
        stdout.push(render_version_line(
            "system",
            None,
            current.contains("system"),
            &origin,
        ));
        num_versions += 1;
    }

    for entry in entries {
        stdout.push(render_version_line(
            &entry.name,
            entry.link_target.as_ref(),
            current.contains(&entry.name.to_ascii_lowercase()),
            &origin,
        ));
        num_versions += 1;
    }

    if num_versions == 0 {
        return CommandReport::failure(
            vec!["Warning: no Python detected on the system".to_string()],
            1,
        );
    }

    CommandReport::success(stdout)
}

pub fn cmd_uninstall(ctx: &AppContext, versions: &[String], force: bool) -> CommandReport {
    if versions.is_empty() {
        return CommandReport::failure(
            vec!["pyenv: uninstall operation requires at least one version argument".to_string()],
            1,
        );
    }

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut removed_any = false;

    for version in versions {
        if version.trim().is_empty() || version.starts_with('-') {
            stderr.push(format!("pyenv: invalid version argument `{version}`"));
            continue;
        }

        let version_dir = installed_version_dir(ctx, version);
        if !version_dir.exists() {
            if !force {
                stderr.push(format!("pyenv: version `{version}` not installed"));
            }
            continue;
        }

        let dependent_envs = managed_venv_entries_for_base(ctx, version).unwrap_or_default();
        if !dependent_envs.is_empty() && !force {
            stderr.push(format!(
                "pyenv: version `{version}` has managed venvs that depend on it: {}",
                dependent_envs
                    .iter()
                    .map(|(spec, _)| format!("`{spec}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            stderr.push(
                "hint: remove or repoint those managed venvs first, or rerun with `--force` to remove the runtime and its dependent venvs"
                    .to_string(),
            );
            continue;
        }

        if !force && !confirm_uninstall(&version_dir) {
            stderr.push(format!("pyenv: uninstall cancelled for `{version}`"));
            continue;
        }

        if let Err(error) = run_hook_scripts(
            ctx,
            "uninstall",
            &[
                ("PYENV_VERSION_NAME", version.to_string()),
                ("PYENV_VERSION", version.to_string()),
                ("PYENV_PREFIX", version_dir.display().to_string()),
                ("PYENV_HOOK_STAGE", "before".to_string()),
            ],
        ) {
            stderr.push(error.to_string());
            continue;
        }

        let mut dependency_failure = false;
        for (spec, path) in &dependent_envs {
            if let Err(error) = fs::remove_dir_all(path) {
                stderr.push(format!(
                    "pyenv: failed to remove dependent managed venv {} at {}: {error}",
                    spec,
                    path.display()
                ));
                dependency_failure = true;
            } else {
                stdout.push(format!("pyenv: removed dependent managed venv {spec}"));
            }
        }
        if dependency_failure {
            continue;
        }

        match fs::remove_dir_all(&version_dir) {
            Ok(_) => {
                removed_any = true;
                stdout.push(format!("pyenv: {version} uninstalled"));
                if let Err(error) = run_hook_scripts(
                    ctx,
                    "uninstall",
                    &[
                        ("PYENV_VERSION_NAME", version.to_string()),
                        ("PYENV_VERSION", version.to_string()),
                        ("PYENV_PREFIX", version_dir.display().to_string()),
                        ("PYENV_HOOK_STAGE", "after".to_string()),
                    ],
                ) {
                    stderr.push(error.to_string());
                }
            }
            Err(error) => stderr.push(format!(
                "pyenv: failed to remove {}: {error}",
                version_dir.display()
            )),
        }
    }

    if removed_any && let Err(error) = rehash_shims(ctx) {
        stderr.push(error.to_string());
    }

    let exit_code = if stderr.is_empty() { 0 } else { 1 };

    CommandReport {
        stdout,
        stderr,
        exit_code,
    }
}

fn cmd_versions_executables(ctx: &AppContext) -> CommandReport {
    let versions = match crate::catalog::installed_version_names(ctx) {
        Ok(versions) => versions,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };

    let mut names = HashSet::new();
    for version in versions {
        for prefix in inventory_roots_for_version(ctx, &version) {
            match collect_shim_names_from_prefix(&prefix, ctx.path_ext.as_deref()) {
                Ok(found) => names.extend(found),
                Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
            }
        }
    }

    let mut stdout = names.into_iter().collect::<Vec<_>>();
    stdout.sort_by_key(|value| value.to_ascii_lowercase());
    CommandReport::success(stdout)
}
