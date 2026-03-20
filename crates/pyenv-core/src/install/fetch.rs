// ./crates/pyenv-core/src/install/fetch.rs
//! Cached metadata retrieval for NuGet and PyPy indexes.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use reqwest::blocking::Client;

use crate::config::RuntimeArch;
use crate::context::AppContext;
use crate::error::PyenvError;

use super::platform::{pypy_manifest_arches, pypy_manifest_platform};
use super::report::io_error;
use super::types::{
    DEFAULT_NUGET_BASE_URL, NUGET_INDEX_TTL_SECS, NugetPackageIndex, PYPY_INDEX_TTL_SECS,
    PYPY_VERSIONS_URL, PypyReleaseFile, PypyReleaseManifest,
};

pub(super) fn is_stable_runtime_version(version: &str) -> bool {
    let probe = version.trim_end_matches('t');
    !probe.is_empty() && probe.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
}

pub(super) fn load_or_fetch_nuget_package_versions(
    ctx: &AppContext,
    package_name: &str,
) -> Result<Vec<String>, PyenvError> {
    let cache_path = nuget_index_cache_path(ctx, package_name);
    if cache_path.is_file() && cache_is_fresh_with_ttl(&cache_path, NUGET_INDEX_TTL_SECS) {
        return read_nuget_index_cache(&cache_path);
    }

    match fetch_nuget_package_versions(ctx, package_name) {
        Ok(versions) => {
            write_nuget_index_cache(&cache_path, &versions)?;
            Ok(versions)
        }
        Err(error) => {
            if cache_path.is_file() {
                read_nuget_index_cache(&cache_path).or(Err(error))
            } else {
                Err(error)
            }
        }
    }
}

fn fetch_nuget_package_versions(
    ctx: &AppContext,
    package_name: &str,
) -> Result<Vec<String>, PyenvError> {
    let base_url = ctx
        .config
        .install
        .source_base_url
        .as_deref()
        .unwrap_or(DEFAULT_NUGET_BASE_URL)
        .trim_end_matches('/')
        .to_string();
    let url = format!(
        "{base_url}/{}/index.json",
        package_name.to_ascii_lowercase()
    );
    let client = Client::builder()
        .user_agent(format!("pyenv-native/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to build HTTP client: {error}")))?;

    let response_body = client
        .get(&url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to query {url}: {error}")))?
        .text()
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to read {url}: {error}")))?;
    let index = serde_json::from_str::<NugetPackageIndex>(&response_body)
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to parse {url}: {error}")))?;

    Ok(index.versions)
}

pub(super) fn nuget_index_cache_path(ctx: &AppContext, package_name: &str) -> PathBuf {
    ctx.cache_dir()
        .join("metadata")
        .join("nuget")
        .join(format!("{}.index.json", package_name.to_ascii_lowercase()))
}

fn cache_is_fresh_with_ttl(path: &Path, ttl_secs: u64) -> bool {
    path.metadata()
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .is_some_and(|age| age.as_secs() <= ttl_secs)
}

fn read_nuget_index_cache(path: &Path) -> Result<Vec<String>, PyenvError> {
    let contents = fs::read_to_string(path).map_err(io_error)?;
    let index = serde_json::from_str::<NugetPackageIndex>(&contents).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to parse cached package index {}: {error}",
            path.display()
        ))
    })?;
    Ok(index.versions)
}

pub(super) fn write_nuget_index_cache(path: &Path, versions: &[String]) -> Result<(), PyenvError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    let payload = serde_json::to_string_pretty(&NugetPackageIndex {
        versions: versions.to_vec(),
    })
    .map_err(|error| {
        PyenvError::Io(format!("pyenv: failed to serialize package index: {error}"))
    })?;
    fs::write(path, payload).map_err(io_error)
}

pub(super) fn pypy_provider_names(
    releases: &[PypyReleaseManifest],
    arch: RuntimeArch,
    platform: &str,
) -> Vec<String> {
    if pypy_manifest_arches(arch, platform).is_empty() {
        return Vec::new();
    }

    let mut versions = releases
        .iter()
        .filter(|release| release.stable)
        .filter_map(|release| {
            release
                .files
                .iter()
                .find(|file| pypy_file_matches_target(file, platform, arch))
                .map(|_| {
                    normalize_pypy_provider_name(&release.python_version, &release.pypy_version)
                })
        })
        .collect::<Vec<_>>();
    versions.sort_by(|lhs, rhs| crate::catalog::compare_version_names(lhs, rhs).reverse());
    versions.dedup();
    versions
}

pub(super) fn find_pypy_release_by_provider_name<'a>(
    releases: &'a [PypyReleaseManifest],
    provider_name: &str,
    arch: RuntimeArch,
    platform: &str,
) -> Option<(&'a PypyReleaseManifest, &'a PypyReleaseFile)> {
    if pypy_manifest_arches(arch, platform).is_empty() {
        return None;
    }

    releases.iter().find_map(|release| {
        if !release.stable
            || normalize_pypy_provider_name(&release.python_version, &release.pypy_version)
                != provider_name
        {
            return None;
        }

        release
            .files
            .iter()
            .find(|file| pypy_file_matches_target(file, platform, arch))
            .map(|file| (release, file))
    })
}

fn pypy_file_matches_target(file: &PypyReleaseFile, platform: &str, arch: RuntimeArch) -> bool {
    let Some(expected_platform) = pypy_manifest_platform(platform) else {
        return false;
    };
    file.platform.eq_ignore_ascii_case(expected_platform)
        && pypy_manifest_arches(arch, platform)
            .iter()
            .any(|candidate| file.arch.eq_ignore_ascii_case(candidate))
}

fn normalize_pypy_provider_name(python_version: &str, pypy_version: &str) -> String {
    let major_minor = python_version
        .split('.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".");
    format!("pypy{major_minor}-{pypy_version}")
}

pub(super) fn load_or_fetch_pypy_releases(
    ctx: &AppContext,
) -> Result<Vec<PypyReleaseManifest>, PyenvError> {
    let cache_path = pypy_index_cache_path(ctx);
    if cache_path.is_file() && cache_is_fresh_with_ttl(&cache_path, PYPY_INDEX_TTL_SECS) {
        return read_pypy_index_cache(&cache_path);
    }

    match fetch_pypy_releases() {
        Ok(releases) => {
            write_pypy_index_cache(&cache_path, &releases)?;
            Ok(releases)
        }
        Err(error) => {
            if cache_path.is_file() {
                read_pypy_index_cache(&cache_path).or(Err(error))
            } else {
                Err(error)
            }
        }
    }
}

fn fetch_pypy_releases() -> Result<Vec<PypyReleaseManifest>, PyenvError> {
    let client = Client::builder()
        .user_agent(format!("pyenv-native/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to build HTTP client: {error}")))?;

    let response = client.get(PYPY_VERSIONS_URL).send().map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to query {PYPY_VERSIONS_URL}: {error}"
        ))
    })?;

    let response = response.error_for_status().map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to query {PYPY_VERSIONS_URL}: {error}"
        ))
    })?;

    let response_body = response.text().map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to read {PYPY_VERSIONS_URL}: {error}"
        ))
    })?;

    serde_json::from_str::<Vec<PypyReleaseManifest>>(&response_body).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to parse {PYPY_VERSIONS_URL}: {error}"
        ))
    })
}

pub(super) fn pypy_index_cache_path(ctx: &AppContext) -> PathBuf {
    ctx.cache_dir()
        .join("metadata")
        .join("pypy")
        .join("versions.json")
}

fn read_pypy_index_cache(path: &Path) -> Result<Vec<PypyReleaseManifest>, PyenvError> {
    let contents = fs::read_to_string(path).map_err(io_error)?;
    serde_json::from_str::<Vec<PypyReleaseManifest>>(&contents).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to parse cached PyPy index {}: {error}",
            path.display()
        ))
    })
}

pub(super) fn write_pypy_index_cache(
    path: &Path,
    releases: &[PypyReleaseManifest],
) -> Result<(), PyenvError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    let payload = serde_json::to_string_pretty(releases).map_err(|error| {
        PyenvError::Io(format!("pyenv: failed to serialize PyPy index: {error}"))
    })?;
    fs::write(path, payload).map_err(io_error)
}
