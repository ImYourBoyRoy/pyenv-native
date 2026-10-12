// ./crates/pyenv-core/src/catalog/remote.rs
//! Live CPython catalog from the python.org release index.
//!
//! Purpose: discover current and future CPython revisions without a compiled-in list.
//! How to use: call `load_cpython_catalog_names` from install listing and version resolution.
//! Inputs: `AppContext` cache directory, or an HTML directory index for parsing.
//! Outputs: version names, including free-threaded `t` siblings for 3.13 and newer.
//! Notes: a fresh cache lasts 24 hours. A failed refresh reuses a stale cache. The
//! compiled `known_versions.txt` is the base seed. Discovered names are merged into
//! `PYENV_ROOT/cache/metadata/known_versions.txt` whenever a publisher cache is read.
//! Unit tests skip the network unless a cache file is seeded.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::context::AppContext;
use crate::error::PyenvError;
use crate::http::build_blocking_client;

use super::entries::{known_version_names, parse_known_version_lines};
use super::latest::compare_version_names;

const PYTHON_ORG_FTP_INDEX: &str = "https://www.python.org/ftp/python/";
const CPYTHON_INDEX_TTL: Duration = Duration::from_secs(60 * 60 * 24);
const FREE_THREADING_MAJOR: u32 = 3;
const FREE_THREADING_MINOR: u32 = 13;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CpythonReleaseIndex {
    versions: Vec<String>,
}

pub(crate) fn cpython_index_cache_path(ctx: &AppContext) -> PathBuf {
    ctx.cache_dir()
        .join("metadata")
        .join("cpython")
        .join("releases.json")
}

pub(crate) fn write_cpython_index_cache(
    path: &Path,
    versions: &[String],
) -> Result<(), PyenvError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| PyenvError::Io(format!("pyenv: {error}")))?;
    }
    let payload = serde_json::to_string_pretty(&CpythonReleaseIndex {
        versions: versions.to_vec(),
    })
    .map_err(|error| {
        PyenvError::Io(format!("pyenv: failed to serialize CPython index: {error}"))
    })?;
    fs::write(path, payload).map_err(|error| PyenvError::Io(format!("pyenv: {error}")))
}

pub(crate) fn known_versions_store_path(ctx: &AppContext) -> PathBuf {
    ctx.cache_dir().join("metadata").join("known_versions.txt")
}

/// Publisher release names plus synthesized free-threaded siblings.
pub(crate) fn load_cpython_catalog_names(ctx: &AppContext) -> Vec<String> {
    let names = expand_cpython_catalog(&resolve_release_dirs(ctx));
    sync_known_version_store(ctx, &names);
    names
}

/// Base seed plus every version discovered from publisher caches.
pub(crate) fn merged_known_version_names(ctx: &AppContext) -> Vec<String> {
    let discovered = load_cpython_catalog_names(ctx);
    sync_known_version_store(ctx, &discovered)
}

/// Merge `discovered` into the runtime known-versions file.
///
/// The compiled seed is always included. The shipped `known_versions.txt` is not rewritten.
pub(crate) fn sync_known_version_store(ctx: &AppContext, discovered: &[String]) -> Vec<String> {
    let stored = read_known_version_store(ctx);
    let mut names = known_version_names().to_vec();
    names.extend(stored);
    names.extend(discovered.iter().cloned());
    names.sort_by(|lhs, rhs| compare_version_names(lhs, rhs));
    names.dedup();
    let _ = write_known_version_store(&known_versions_store_path(ctx), &names);
    names
}

pub(crate) fn parse_python_org_ftp_index(html: &str) -> Vec<String> {
    let mut versions = Vec::new();
    let mut rest = html;
    while let Some(marker) = rest.find("href=\"") {
        rest = &rest[marker + 6..];
        let Some(end) = rest.find('"') else {
            break;
        };
        let href = &rest[..end];
        rest = &rest[end + 1..];
        if !href.ends_with('/') {
            continue;
        }
        let name = href
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(href);
        if is_cpython_release_dir(name) {
            versions.push(name.to_string());
        }
    }
    versions.sort_by(|lhs, rhs| compare_version_names(lhs, rhs));
    versions.dedup();
    versions
}

pub(crate) fn cpython_major_minor(version: &str) -> Option<(u32, u32)> {
    let base = version.trim_end_matches('t');
    let mut parts = base.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor_raw = parts.next().unwrap_or("0");
    let minor_digits: String = minor_raw
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    if minor_digits.is_empty() {
        return None;
    }
    Some((major, minor_digits.parse().ok()?))
}

fn resolve_release_dirs(ctx: &AppContext) -> Vec<String> {
    let cache_path = cpython_index_cache_path(ctx);
    if let Some(versions) = read_fresh_cache(&cache_path) {
        return versions;
    }

    // Tests stay offline unless they seed a cache. The release binary refreshes.
    if !cfg!(test)
        && let Some(versions) = fetch_and_cache(&cache_path)
    {
        return versions;
    }

    if let Some(versions) = read_cache(&cache_path) {
        return versions;
    }

    embedded_release_dirs()
}

fn fetch_and_cache(cache_path: &Path) -> Option<Vec<String>> {
    let versions = fetch_python_org_index().ok()?;
    if versions.is_empty() {
        return None;
    }
    let _ = write_cpython_index_cache(cache_path, &versions);
    Some(versions)
}

fn fetch_python_org_index() -> Result<Vec<String>, PyenvError> {
    let client = build_blocking_client()
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to build HTTP client: {error}")))?;
    let body = client
        .get(PYTHON_ORG_FTP_INDEX)
        .timeout(Duration::from_secs(20))
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to query {PYTHON_ORG_FTP_INDEX}: {error}"
            ))
        })?
        .text()
        .map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to read {PYTHON_ORG_FTP_INDEX}: {error}"
            ))
        })?;
    Ok(parse_python_org_ftp_index(&body))
}

fn read_fresh_cache(path: &Path) -> Option<Vec<String>> {
    if path.is_file() && cache_is_fresh(path) {
        read_cache(path)
    } else {
        None
    }
}

fn read_cache(path: &Path) -> Option<Vec<String>> {
    let contents = fs::read_to_string(path).ok()?;
    let index = serde_json::from_str::<CpythonReleaseIndex>(&contents).ok()?;
    let versions = index
        .versions
        .into_iter()
        .filter(|version| is_cpython_release_dir(version.trim_end_matches('t')))
        .collect::<Vec<_>>();
    if versions.is_empty() {
        None
    } else {
        Some(versions)
    }
}

fn cache_is_fresh(path: &Path) -> bool {
    path.metadata()
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .is_some_and(|age| age <= CPYTHON_INDEX_TTL)
}

fn read_known_version_store(ctx: &AppContext) -> Vec<String> {
    let path = known_versions_store_path(ctx);
    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    parse_known_version_lines(&text)
}

fn write_known_version_store(path: &Path, names: &[String]) -> Result<(), PyenvError> {
    let payload = if names.is_empty() {
        String::new()
    } else {
        format!("{}\n", names.join("\n"))
    };
    if path.is_file()
        && fs::read_to_string(path)
            .ok()
            .is_some_and(|current| current == payload)
    {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| PyenvError::Io(format!("pyenv: {error}")))?;
    }
    fs::write(path, payload).map_err(|error| PyenvError::Io(format!("pyenv: {error}")))
}

fn embedded_release_dirs() -> Vec<String> {
    known_version_names()
        .iter()
        .filter(|version| is_cpython_release_dir(version))
        .cloned()
        .collect()
}

fn expand_cpython_catalog(release_dirs: &[String]) -> Vec<String> {
    let mut names = Vec::with_capacity(release_dirs.len() * 2);
    for version in release_dirs {
        let base = version.trim_end_matches('t');
        if !is_cpython_release_dir(base) {
            continue;
        }
        names.push(base.to_string());
        if supports_free_threading(base) {
            names.push(format!("{base}t"));
        }
    }
    names.sort_by(|lhs, rhs| compare_version_names(lhs, rhs));
    names.dedup();
    names
}

fn supports_free_threading(version: &str) -> bool {
    cpython_major_minor(version).is_some_and(|(major, minor)| {
        (major, minor) >= (FREE_THREADING_MAJOR, FREE_THREADING_MINOR)
    })
}

pub(crate) fn is_cpython_release_dir(name: &str) -> bool {
    if name.is_empty() || !name.starts_with(|ch: char| ch.is_ascii_digit()) {
        return false;
    }
    let bytes = name.as_bytes();
    let mut index = 0;
    if !consume_digits(bytes, &mut index) {
        return false;
    }
    while index < bytes.len() && bytes[index] == b'.' {
        index += 1;
        if !consume_digits(bytes, &mut index) {
            return false;
        }
    }
    if index == bytes.len() {
        return true;
    }
    let rest = &name[index..];
    let digits = if let Some(digits) = rest.strip_prefix("rc") {
        digits
    } else if rest.starts_with(['a', 'b']) {
        &rest[1..]
    } else {
        return false;
    };
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn consume_digits(bytes: &[u8], index: &mut usize) -> bool {
    let start = *index;
    while *index < bytes.len() && bytes[*index].is_ascii_digit() {
        *index += 1;
    }
    *index > start
}
