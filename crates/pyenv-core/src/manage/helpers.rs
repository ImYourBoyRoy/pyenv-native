// ./crates/pyenv-core/src/manage/helpers.rs
//! Shared prefix, listing, and prompt helpers for manage commands.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::catalog::compare_version_names;
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::executable::find_system_python_command;
use crate::runtime::managed_search_roots_for_version;
use crate::venv_paths::managed_venv_entries_for_base;
use crate::version::{VersionOrigin, find_local_version_file};

use super::types::VersionEntry;

pub(super) fn resolve_prefix_path(ctx: &AppContext, version: &str) -> Result<PathBuf, PyenvError> {
    if version == "system" {
        return system_prefix(ctx)
            .ok_or_else(|| PyenvError::Io("pyenv: system version not found in PATH".to_string()));
    }

    for prefix in managed_search_roots_for_version(ctx, version) {
        if prefix.is_dir() {
            return Ok(prefix);
        }
    }

    if let Some(resolved) = crate::catalog::latest_installed_version(ctx, version) {
        for prefix in managed_search_roots_for_version(ctx, &resolved) {
            if prefix.is_dir() {
                return Ok(prefix);
            }
        }
        return Err(PyenvError::Io(format!(
            "pyenv: version `{resolved}` not installed"
        )));
    }

    Err(PyenvError::Io(format!(
        "pyenv: version `{version}` not installed"
    )))
}

pub(super) fn join_prefixes(prefixes: &[PathBuf]) -> String {
    let separator = if cfg!(windows) { ";" } else { ":" };
    prefixes
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(separator)
}

pub(super) fn system_prefix(ctx: &AppContext) -> Option<PathBuf> {
    let python_path = find_system_python_command(ctx)?;
    system_prefix_from_python(&python_path)
}

fn system_prefix_from_python(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let parent_name = parent.file_name()?.to_string_lossy().to_ascii_lowercase();
    if matches!(parent_name.as_str(), "bin" | "sbin" | "scripts") {
        let prefix = parent
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(parent));
        if prefix.as_os_str().is_empty() {
            Some(PathBuf::from(std::path::MAIN_SEPARATOR.to_string()))
        } else {
            Some(prefix)
        }
    } else {
        Some(parent.to_path_buf())
    }
}

pub(super) fn current_version_origin(ctx: &AppContext) -> String {
    if ctx.env_version.is_some() {
        VersionOrigin::Environment.to_string()
    } else if let Some(local_file) = find_local_version_file(&ctx.dir) {
        VersionOrigin::File(local_file).to_string()
    } else {
        VersionOrigin::File(ctx.root.join("version")).to_string()
    }
}

pub(super) fn render_version_line(
    name: &str,
    link_target: Option<&PathBuf>,
    current: bool,
    origin: &str,
) -> String {
    let repr = link_target
        .map(|target| format!("{name} --> {}", target.display()))
        .unwrap_or_else(|| name.to_string());

    if current {
        format!("* {repr} (set by {origin})")
    } else {
        format!("  {repr}")
    }
}

pub(super) fn list_version_entries(
    ctx: &AppContext,
    skip_aliases: bool,
    skip_envs: bool,
) -> Result<Vec<VersionEntry>, PyenvError> {
    let versions_dir = ctx.versions_dir();
    if !versions_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(&versions_dir)
        .map_err(io_error)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            path.is_dir()
                .then(|| entry.file_name().to_string_lossy().to_string())
        })
        .collect::<Vec<_>>();
    entries.sort_by(|lhs, rhs| compare_version_names(lhs, rhs));

    let versions_dir_canonical = fs::canonicalize(&versions_dir).unwrap_or(versions_dir.clone());
    let mut results = Vec::new();

    for version in entries {
        let path = versions_dir.join(&version);
        let metadata = fs::symlink_metadata(&path).map_err(io_error)?;
        let link_target = if metadata.file_type().is_symlink() {
            fs::read_link(&path).ok()
        } else {
            None
        };

        if skip_aliases
            && metadata.file_type().is_symlink()
            && fs::canonicalize(&path)
                .ok()
                .is_some_and(|target| target.starts_with(&versions_dir_canonical))
        {
            continue;
        }

        results.push(VersionEntry {
            name: version.clone(),
            link_target,
        });

        if skip_envs {
            continue;
        }

        for (spec, env_path) in managed_venv_entries_for_base(ctx, &version)? {
            let env_metadata = fs::symlink_metadata(&env_path).map_err(io_error)?;
            let env_link_target = if env_metadata.file_type().is_symlink() {
                fs::read_link(&env_path).ok()
            } else {
                None
            };
            results.push(VersionEntry {
                name: spec,
                link_target: env_link_target,
            });
        }
    }

    results.sort_by(|lhs, rhs| compare_version_names(&lhs.name, &rhs.name));
    Ok(results)
}

pub(super) fn confirm_uninstall(prefix: &Path) -> bool {
    let _ = write!(io::stdout(), "pyenv: remove {}? (y/N) ", prefix.display());
    let _ = io::stdout().flush();

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes"),
        Err(_) => false,
    }
}

pub(super) fn io_error(error: std::io::Error) -> PyenvError {
    PyenvError::Io(format!("pyenv: {error}"))
}
