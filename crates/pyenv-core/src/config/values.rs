// ./crates/pyenv-core/src/config/values.rs
//! Key-based config value normalization, lookup, and mutation helpers.

use std::path::PathBuf;

use crate::error::PyenvError;

use super::types::{AppConfig, RegistryMode, RuntimeArch};

pub fn get_config_value(config: &AppConfig, key: &str) -> Result<String, PyenvError> {
    match key {
        "storage.versions_dir" => Ok(config
            .storage
            .versions_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default()),
        "storage.cache_dir" => Ok(config
            .storage
            .cache_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default()),
        "windows.registry_mode" => Ok(config.windows.registry_mode.as_str().to_string()),
        "install.arch" => Ok(config.install.arch.as_str().to_string()),
        "install.source_base_url" => Ok(config.install.source_base_url.clone().unwrap_or_default()),
        "install.python_build_path" => Ok(config
            .install
            .python_build_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default()),
        "install.bootstrap_pip" => Ok(config.install.bootstrap_pip.to_string()),
        "venv.auto_create_base_venv" => Ok(config.venv.auto_create_base_venv.to_string()),
        "venv.auto_use_base_venv" => Ok(config.venv.auto_use_base_venv.to_string()),
        _ => Err(PyenvError::UnknownConfigKey(key.to_string())),
    }
}

pub fn set_config_value(config: &mut AppConfig, key: &str, value: &str) -> Result<(), PyenvError> {
    match key {
        "storage.versions_dir" => {
            config.storage.versions_dir = normalize_path_option(value);
            Ok(())
        }
        "storage.cache_dir" => {
            config.storage.cache_dir = normalize_path_option(value);
            Ok(())
        }
        "windows.registry_mode" => {
            config.windows.registry_mode =
                RegistryMode::parse(value).ok_or_else(|| PyenvError::InvalidConfigValue {
                    key: key.to_string(),
                    value: value.to_string(),
                })?;
            Ok(())
        }
        "install.arch" => {
            config.install.arch =
                RuntimeArch::parse(value).ok_or_else(|| PyenvError::InvalidConfigValue {
                    key: key.to_string(),
                    value: value.to_string(),
                })?;
            Ok(())
        }
        "install.source_base_url" => {
            config.install.source_base_url = normalize_string_option(value);
            Ok(())
        }
        "install.python_build_path" => {
            config.install.python_build_path = normalize_path_option(value);
            Ok(())
        }
        "install.bootstrap_pip" => {
            config.install.bootstrap_pip = parse_bool(key, value)?;
            Ok(())
        }
        "venv.auto_create_base_venv" => {
            config.venv.auto_create_base_venv = parse_bool(key, value)?;
            Ok(())
        }
        "venv.auto_use_base_venv" => {
            config.venv.auto_use_base_venv = parse_bool(key, value)?;
            Ok(())
        }
        _ => Err(PyenvError::UnknownConfigKey(key.to_string())),
    }
}

fn parse_bool(key: &str, value: &str) -> Result<bool, PyenvError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(PyenvError::InvalidConfigValue {
            key: key.to_string(),
            value: value.to_string(),
        }),
    }
}

fn normalize_path_option(value: &str) -> Option<PathBuf> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

fn normalize_string_option(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
