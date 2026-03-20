// ./crates/pyenv-core/src/install/types.rs
//! Shared install constants, request/response models, and provider catalog records.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub(super) const DEFAULT_NUGET_BASE_URL: &str = "https://api.nuget.org/v3-flatcontainer";
pub(super) const DEFAULT_CPYTHON_SOURCE_BASE_URL: &str = "https://www.python.org/ftp/python";
pub(super) const INSTALL_RECEIPT_FILE: &str = ".pyenv-install.json";
pub(super) const NUGET_INDEX_TTL_SECS: u64 = 60 * 60 * 24;
pub(super) const PYPY_VERSIONS_URL: &str = "https://downloads.python.org/pypy/versions.json";
pub(super) const PYPY_INDEX_TTL_SECS: u64 = 60 * 60 * 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallCommandOptions {
    pub list: bool,
    pub force: bool,
    pub dry_run: bool,
    pub json: bool,
    pub known: bool,
    pub family: Option<String>,
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallPlan {
    pub requested_version: String,
    pub resolved_version: String,
    pub family: String,
    pub provider: String,
    pub architecture: String,
    pub runtime_version: String,
    pub free_threaded: bool,
    pub package_name: String,
    pub package_version: String,
    pub download_url: String,
    pub cache_path: PathBuf,
    pub install_dir: PathBuf,
    pub python_executable: PathBuf,
    pub bootstrap_pip: bool,
    pub create_base_venv: bool,
    pub base_venv_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallOutcome {
    pub plan: InstallPlan,
    pub receipt_path: PathBuf,
    pub pip_bootstrapped: bool,
    pub base_venv_created: bool,
    pub progress_steps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct InstallReceipt {
    pub requested_version: String,
    pub resolved_version: String,
    pub provider: String,
    pub family: String,
    pub architecture: String,
    pub runtime_version: String,
    pub package_name: String,
    pub package_version: String,
    pub download_url: String,
    pub cache_path: PathBuf,
    pub python_executable: PathBuf,
    pub bootstrap_pip: bool,
    pub base_venv_path: Option<PathBuf>,
    pub installed_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct NugetPackageIndex {
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct PypyReleaseManifest {
    pub pypy_version: String,
    pub python_version: String,
    pub stable: bool,
    #[serde(default)]
    pub latest_pypy: bool,
    pub files: Vec<PypyReleaseFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct PypyReleaseFile {
    pub filename: String,
    pub arch: String,
    pub platform: String,
    pub download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct ProviderCatalogGroup {
    pub family: String,
    pub family_slug: String,
    pub provider: String,
    pub architecture: String,
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProviderCatalogEntry {
    pub family: String,
    pub family_slug: String,
    pub provider: String,
    pub architecture: String,
    pub version: String,
}
