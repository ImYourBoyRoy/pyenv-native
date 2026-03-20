// ./crates/pyenv-core/src/config/storage.rs
//! Config file persistence and derived storage-path helpers for pyenv-native roots.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::PyenvError;

use super::types::AppConfig;

pub fn config_path(root: &Path) -> PathBuf {
    root.join("config.toml")
}

pub fn load_config(root: &Path) -> Result<AppConfig, PyenvError> {
    let path = config_path(root);
    if !path.is_file() {
        return Ok(AppConfig::default());
    }

    let contents = fs::read_to_string(&path).map_err(io_error)?;
    toml::from_str::<AppConfig>(&contents).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to parse {}: {error}",
            path.display()
        ))
    })
}

pub fn save_config(root: &Path, config: &AppConfig) -> Result<(), PyenvError> {
    let path = config_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }

    let contents = toml::to_string_pretty(config)
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to serialize config: {error}")))?;
    fs::write(path, contents).map_err(io_error)
}

pub fn resolve_versions_dir(root: &Path, config: &AppConfig) -> PathBuf {
    resolve_storage_path(root, config.storage.versions_dir.as_deref(), "versions")
}

pub fn resolve_cache_dir(root: &Path, config: &AppConfig) -> PathBuf {
    resolve_storage_path(root, config.storage.cache_dir.as_deref(), "cache")
}

fn resolve_storage_path(root: &Path, configured: Option<&Path>, default_name: &str) -> PathBuf {
    match configured {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => root.join(path),
        None => root.join(default_name),
    }
}

pub(super) fn io_error(error: std::io::Error) -> PyenvError {
    PyenvError::Io(format!("pyenv: {error}"))
}
