// ./crates/pyenv-core/src/executable/lookup.rs
//! Managed-runtime and system-PATH executable lookup helpers.

use std::env;
use std::path::{Path, PathBuf};

use crate::context::AppContext;
use crate::runtime::{find_command_in_prefix, search_path_entries};

pub(crate) fn find_system_command(ctx: &AppContext, command: &str) -> Option<PathBuf> {
    let mut path_entries = ctx
        .path_env
        .as_ref()
        .map(env::split_paths)
        .into_iter()
        .flatten()
        .filter(|entry| !entry.as_os_str().is_empty())
        .collect::<Vec<_>>();

    let mut removal_targets = vec![ctx.shims_dir()];
    if let Some(extra_paths) = env::var_os(program_specific_shim_paths_env(command)) {
        removal_targets.extend(env::split_paths(&extra_paths));
    }
    path_entries.retain(|entry| {
        !removal_targets
            .iter()
            .any(|target| paths_equal(entry, target))
    });

    search_path_entries(&path_entries, command, ctx.path_ext.as_deref())
}

pub(crate) fn find_system_python_command(ctx: &AppContext) -> Option<PathBuf> {
    for command in ["python", "python3", "python2"] {
        if let Some(path) = find_system_command(ctx, command) {
            return Some(path);
        }
    }
    None
}

pub(crate) fn find_command_in_version(
    ctx: &AppContext,
    version: &str,
    command: &str,
) -> Option<PathBuf> {
    for prefix in crate::runtime::managed_search_roots_for_version(ctx, version) {
        if let Some(path) = find_command_in_prefix(&prefix, command, ctx.path_ext.as_deref()) {
            return Some(path);
        }
    }
    None
}

fn program_specific_shim_paths_env(command: &str) -> String {
    let sanitized = command
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("_PYENV_SHIM_PATHS_{sanitized}")
}

fn paths_equal(lhs: &Path, rhs: &Path) -> bool {
    if cfg!(windows) {
        lhs.to_string_lossy()
            .replace('/', "\\")
            .eq_ignore_ascii_case(&rhs.to_string_lossy().replace('/', "\\"))
    } else {
        lhs == rhs
    }
}
