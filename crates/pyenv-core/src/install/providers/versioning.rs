// ./crates/pyenv-core/src/install/providers/versioning.rs
//! Provider-version normalization and package-name helpers for install planning.

use crate::catalog::latest_version_from_names;
use crate::config::RuntimeArch;
use crate::context::AppContext;
use crate::error::PyenvError;

use super::super::fetch::load_or_fetch_nuget_package_versions;

pub(crate) fn resolve_provider_version(
    ctx: &AppContext,
    package_name: &str,
    requested: &str,
    free_threaded: bool,
) -> Result<String, PyenvError> {
    let versions = available_package_versions(ctx, package_name, free_threaded)?;
    if versions.iter().any(|version| version == requested) {
        return Ok(requested.to_string());
    }

    latest_version_from_names(requested, &versions)
        .ok_or_else(|| PyenvError::UnknownVersion(requested.to_string()))
}

pub(crate) fn ensure_supported_cpython_version(version: &str) -> Result<(), PyenvError> {
    if is_supported_cpython_version(version) {
        Ok(())
    } else {
        Err(PyenvError::UnsupportedInstallTarget(version.to_string()))
    }
}

fn is_supported_cpython_version(version: &str) -> bool {
    let probe = version.trim_end_matches('t');
    !probe.is_empty() && probe.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

pub(crate) fn is_free_threaded(version: &str) -> bool {
    version.len() > 1
        && version.ends_with('t')
        && version
            .chars()
            .nth_back(1)
            .is_some_and(|ch| ch.is_ascii_digit())
}

pub(crate) fn is_pypy_request(version: &str) -> bool {
    version.to_ascii_lowercase().starts_with("pypy")
}

pub(crate) fn normalize_requested_version(version: &str) -> String {
    let trimmed = version.trim();
    let stripped = trimmed
        .strip_prefix("python-")
        .or_else(|| trimmed.strip_prefix("cpython-"))
        .unwrap_or(trimmed);

    if stripped.to_ascii_lowercase().starts_with("pypy") {
        stripped.replace("-v", "-")
    } else {
        stripped.to_string()
    }
}

pub(crate) fn nuget_package_name(arch: RuntimeArch, free_threaded: bool) -> &'static str {
    match (arch, free_threaded) {
        (RuntimeArch::X64 | RuntimeArch::Auto, false) => "python",
        (RuntimeArch::X64 | RuntimeArch::Auto, true) => "python-freethreaded",
        (RuntimeArch::X86, false) => "pythonx86",
        (RuntimeArch::X86, true) => "pythonx86-freethreaded",
        (RuntimeArch::Arm64, false) => "pythonarm64",
        (RuntimeArch::Arm64, true) => "pythonarm64-freethreaded",
    }
}

pub(super) fn available_package_versions(
    ctx: &AppContext,
    package_name: &str,
    free_threaded: bool,
) -> Result<Vec<String>, PyenvError> {
    let mut versions = load_or_fetch_nuget_package_versions(ctx, package_name)?;
    if free_threaded {
        versions = versions
            .into_iter()
            .map(|version| format!("{version}t"))
            .collect();
    }
    Ok(versions)
}
