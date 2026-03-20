// ./crates/pyenv-core/src/venv_paths.rs
//! Shared managed-venv spec and path helpers for runtime lookup, uninstall safety, and UX.

use std::fs;
use std::path::PathBuf;

use crate::context::AppContext;
use crate::error::PyenvError;

pub const MANAGED_VENVS_DIR_NAME: &str = "venvs";

pub fn managed_venvs_root(ctx: &AppContext) -> PathBuf {
    ctx.root.join(MANAGED_VENVS_DIR_NAME)
}

pub fn managed_venv_spec(base_version: &str, name: &str) -> String {
    format!("{base_version}/envs/{name}")
}

pub fn split_managed_venv_spec(spec: &str) -> Option<(String, String)> {
    let normalized = spec.replace('\\', "/");
    let marker = "/envs/";
    let (base, name) = normalized.split_once(marker)?;
    let trimmed_base = base.trim().trim_matches('/');
    let trimmed_name = name.trim().trim_matches('/');
    if trimmed_base.is_empty() || trimmed_name.is_empty() {
        return None;
    }
    Some((trimmed_base.to_string(), trimmed_name.to_string()))
}

pub fn managed_venv_dir(ctx: &AppContext, base_version: &str, name: &str) -> PathBuf {
    managed_venvs_root(ctx).join(base_version).join(name)
}

pub fn managed_venv_dir_from_spec(ctx: &AppContext, spec: &str) -> Option<PathBuf> {
    let (base_version, name) = split_managed_venv_spec(spec)?;
    Some(managed_venv_dir(ctx, &base_version, &name))
}

pub fn managed_venv_entries_for_base(
    ctx: &AppContext,
    base_version: &str,
) -> Result<Vec<(String, PathBuf)>, PyenvError> {
    let base_dir = managed_venvs_root(ctx).join(base_version);
    if !base_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut results = fs::read_dir(&base_dir)
        .map_err(io_error)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter_map(|path| {
            path.file_name()
                .map(|value| value.to_string_lossy().to_string())
                .map(|name| (managed_venv_spec(base_version, &name), path))
        })
        .collect::<Vec<_>>();

    results.sort_by(|lhs, rhs| lhs.0.to_ascii_lowercase().cmp(&rhs.0.to_ascii_lowercase()));
    Ok(results)
}

fn io_error(error: std::io::Error) -> PyenvError {
    PyenvError::Io(format!("pyenv: {error}"))
}
