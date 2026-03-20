// ./crates/pyenv-core/src/version/selection.rs
//! Runtime and managed-env selection helpers that resolve active versions from environment
//! variables or version files.

use std::path::PathBuf;

use crate::catalog::latest_installed_version;
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::venv_paths::managed_venv_dir_from_spec;

use super::files::{read_version_file, version_file_path};
use super::types::{SelectedVersions, VersionOrigin};

pub fn version_origin(ctx: &AppContext) -> VersionOrigin {
    if ctx.env_version.is_some() {
        VersionOrigin::Environment
    } else {
        VersionOrigin::File(version_file_path(ctx, None))
    }
}

pub fn installed_version_dir(ctx: &AppContext, version: &str) -> PathBuf {
    ctx.versions_dir().join(version)
}

pub fn resolve_selected_versions(ctx: &AppContext, force: bool) -> SelectedVersions {
    let raw_versions = if let Some(env_version) = &ctx.env_version {
        parse_env_versions(env_version)
    } else {
        let version_file = version_file_path(ctx, None);
        match read_version_file(&version_file) {
            Ok(versions) => versions,
            Err(_) => vec!["system".to_string()],
        }
    };

    if raw_versions.is_empty() {
        return SelectedVersions {
            versions: vec!["system".to_string()],
            missing: Vec::new(),
            origin: version_origin(ctx),
        };
    }

    let origin = version_origin(ctx);
    let mut versions = Vec::new();
    let mut missing = Vec::new();

    for raw_version in raw_versions {
        let normalized = normalize_version_name(&raw_version);
        if version_exists(ctx, &raw_version) {
            versions.push(raw_version);
        } else if version_exists(ctx, &normalized) {
            versions.push(normalized);
        } else if let Some(resolved) = latest_installed_version(ctx, &raw_version) {
            versions.push(resolved);
        } else if let Some(resolved) = latest_installed_version(ctx, &normalized) {
            versions.push(resolved);
        } else if force {
            versions.push(normalized);
        } else {
            missing.push(raw_version);
        }
    }

    if versions.is_empty() && missing.is_empty() {
        versions.push("system".to_string());
    }

    SelectedVersions {
        versions,
        missing,
        origin,
    }
}

pub(super) fn ensure_versions_exist(
    ctx: &AppContext,
    versions: &[String],
    force: bool,
    origin: &str,
) -> Result<(), PyenvError> {
    for version in versions {
        let normalized = normalize_version_name(version);
        if force
            || version_exists(ctx, version)
            || version_exists(ctx, &normalized)
            || latest_installed_version(ctx, version).is_some()
            || latest_installed_version(ctx, &normalized).is_some()
        {
            continue;
        }

        return Err(PyenvError::VersionNotInstalled(
            version.clone(),
            origin.to_string(),
        ));
    }

    Ok(())
}

fn version_exists(ctx: &AppContext, version: &str) -> bool {
    version == "system"
        || installed_version_dir(ctx, version).is_dir()
        || managed_venv_dir_from_spec(ctx, version).is_some_and(|path| path.is_dir())
}

fn normalize_version_name(version: &str) -> String {
    version
        .strip_prefix("python-")
        .unwrap_or(version)
        .to_string()
}

fn parse_env_versions(value: &str) -> Vec<String> {
    value
        .split(':')
        .flat_map(|segment| segment.split_whitespace())
        .filter(|segment| !segment.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
