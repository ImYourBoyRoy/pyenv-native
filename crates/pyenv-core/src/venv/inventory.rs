// ./crates/pyenv-core/src/venv/inventory.rs
//! Managed-venv inventory and resolution helpers for listing envs, resolving short names, and
//! building stable metadata records for CLI, MCP, and future GUI consumers.

use std::fs;
use std::path::PathBuf;

use crate::catalog::{compare_version_names, latest_installed_version};
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::venv_paths::{
    managed_venv_dir, managed_venv_entries_for_base, managed_venv_spec, managed_venvs_root,
    split_managed_venv_spec,
};
use crate::version::installed_version_dir;

use super::helpers::{interpreter_for_prefix, io_error, pip_for_prefix};
use super::types::ManagedVenvInfo;

pub fn list_managed_venvs(ctx: &AppContext) -> Result<Vec<ManagedVenvInfo>, PyenvError> {
    let mut results = Vec::new();
    let registry_root = managed_venvs_root(ctx);
    if !registry_root.is_dir() {
        return Ok(results);
    }

    let mut versions = fs::read_dir(&registry_root)
        .map_err(io_error)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    versions.sort_by(|lhs, rhs| compare_version_names(lhs, rhs));

    for version in versions {
        for (spec, path) in managed_venv_entries_for_base(ctx, &version)? {
            if let Some((base_version, name)) = split_managed_venv_spec(&spec) {
                results.push(build_managed_venv_info(base_version, name, path));
            }
        }
    }

    results.sort_by(|lhs, rhs| compare_version_names(&lhs.spec, &rhs.spec));
    Ok(results)
}

pub fn resolve_managed_venv(ctx: &AppContext, spec: &str) -> Result<ManagedVenvInfo, PyenvError> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err(PyenvError::Io(
            "pyenv: managed venv spec cannot be empty".to_string(),
        ));
    }

    if let Some((base_version, name)) = split_managed_venv_spec(trimmed) {
        let info = build_managed_venv_info(
            base_version.clone(),
            name.clone(),
            managed_venv_dir(ctx, &base_version, &name),
        );
        if info.path.is_dir() {
            return Ok(info);
        }
        return Err(PyenvError::Io(format!(
            "pyenv: managed venv `{}` is not installed",
            info.spec
        )));
    }

    let matches = find_env_name_matches(ctx, trimmed)?;
    match matches.len() {
        0 => Err(PyenvError::Io(format!(
            "pyenv: no managed venv named `{trimmed}` was found"
        ))),
        1 => Ok(matches[0].clone()),
        _ => Err(PyenvError::Io(format!(
            "pyenv: managed venv name `{trimmed}` is ambiguous; use one of: {}",
            matches
                .iter()
                .map(|info| format!("`{}`", info.spec))
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

pub(super) fn find_env_name_matches(
    ctx: &AppContext,
    name: &str,
) -> Result<Vec<ManagedVenvInfo>, PyenvError> {
    let normalize = |value: &str| {
        if cfg!(windows) {
            value.to_ascii_lowercase()
        } else {
            value.to_string()
        }
    };
    let requested = normalize(name);
    Ok(list_managed_venvs(ctx)?
        .into_iter()
        .filter(|info| normalize(&info.name) == requested)
        .collect())
}

pub(super) fn build_managed_venv_info(
    base_version: String,
    name: String,
    path: PathBuf,
) -> ManagedVenvInfo {
    ManagedVenvInfo {
        spec: managed_venv_spec(&base_version, &name),
        base_version,
        name,
        python_path: interpreter_for_prefix(&path),
        pip_path: pip_for_prefix(&path),
        path,
    }
}

pub(super) fn resolve_installed_runtime_version(
    ctx: &AppContext,
    requested_version: &str,
) -> Result<String, PyenvError> {
    let normalized = requested_version
        .strip_prefix("python-")
        .unwrap_or(requested_version)
        .trim();

    if installed_version_dir(ctx, normalized).is_dir() {
        return Ok(normalized.to_string());
    }

    if let Some(resolved) = latest_installed_version(ctx, normalized) {
        return Ok(resolved);
    }

    Err(PyenvError::VersionNotInstalled(
        normalized.to_string(),
        "pyenv venv create".to_string(),
    ))
}
