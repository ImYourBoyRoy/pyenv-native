// ./crates/pyenv-core/src/config.rs
//! Configuration loading, storage, and mutation for the native pyenv runtime.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::error::PyenvError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub windows: WindowsConfig,
    #[serde(default)]
    pub install: InstallConfig,
    #[serde(default)]
    pub venv: VenvConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StorageConfig {
    #[serde(default)]
    pub versions_dir: Option<PathBuf>,
    #[serde(default)]
    pub cache_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RegistryMode {
    #[default]
    Disabled,
    Pep514,
}

impl RegistryMode {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "disabled" | "none" | "off" | "false" => Some(Self::Disabled),
            "pep514" | "pep-514" => Some(Self::Pep514),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Pep514 => "pep514",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeArch {
    #[default]
    Auto,
    X64,
    X86,
    Arm64,
}

impl RuntimeArch {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" | "default" => Some(Self::Auto),
            "x64" | "amd64" | "x86_64" => Some(Self::X64),
            "x86" | "win32" | "i386" | "i686" => Some(Self::X86),
            "arm64" | "aarch64" => Some(Self::Arm64),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::X64 => "x64",
            Self::X86 => "x86",
            Self::Arm64 => "arm64",
        }
    }

    pub fn effective(self) -> Self {
        match self {
            Self::Auto => match std::env::consts::ARCH {
                "x86_64" => Self::X64,
                "x86" | "i686" => Self::X86,
                "aarch64" => Self::Arm64,
                _ => Self::X64,
            },
            explicit => explicit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowsConfig {
    #[serde(default)]
    pub registry_mode: RegistryMode,
}

impl Default for WindowsConfig {
    fn default() -> Self {
        Self {
            registry_mode: RegistryMode::Disabled,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallConfig {
    #[serde(default)]
    pub arch: RuntimeArch,
    #[serde(default)]
    pub source_base_url: Option<String>,
    #[serde(default)]
    pub python_build_path: Option<PathBuf>,
    #[serde(default = "default_true")]
    pub bootstrap_pip: bool,
}

impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            arch: RuntimeArch::Auto,
            source_base_url: None,
            python_build_path: None,
            bootstrap_pip: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VenvConfig {
    #[serde(default)]
    pub auto_create_base_venv: bool,
    #[serde(default)]
    pub auto_use_base_venv: bool,
}

fn default_true() -> bool {
    true
}

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
    resolve_storage_path(root, config.storage.versions_dir.as_ref(), "versions")
}

pub fn resolve_cache_dir(root: &Path, config: &AppConfig) -> PathBuf {
    resolve_storage_path(root, config.storage.cache_dir.as_ref(), "cache")
}

fn resolve_storage_path(root: &Path, configured: Option<&PathBuf>, default_name: &str) -> PathBuf {
    match configured {
        Some(path) if path.is_absolute() => path.clone(),
        Some(path) => root.join(path),
        None => root.join(default_name),
    }
}

pub fn cmd_config_path(ctx: &AppContext) -> CommandReport {
    CommandReport::success_one(config_path(&ctx.root).display().to_string())
}

pub fn cmd_config_show(ctx: &AppContext) -> CommandReport {
    match toml::to_string_pretty(&ctx.config) {
        Ok(contents) => CommandReport::success(contents.lines().map(ToOwned::to_owned).collect()),
        Err(error) => CommandReport::failure(
            vec![format!("pyenv: failed to serialize config: {error}")],
            1,
        ),
    }
}

pub fn cmd_config_get(ctx: &AppContext, key: &str) -> CommandReport {
    match get_config_value(&ctx.config, key) {
        Ok(value) => CommandReport::success_one(value),
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

pub fn cmd_config_set(ctx: &mut AppContext, key: &str, value: &str) -> CommandReport {
    let mut config = ctx.config.clone();
    match set_config_value(&mut config, key, value).and_then(|_| save_config(&ctx.root, &config)) {
        Ok(_) => {
            ctx.config = config;
            CommandReport::empty_success()
        }
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

fn io_error(error: std::io::Error) -> PyenvError {
    PyenvError::Io(format!("pyenv: {error}"))
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

fn normalize_path_value(value: &str) -> PathBuf {
    PathBuf::from(value.trim())
}

fn normalize_string_option(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn get_config_value(config: &AppConfig, key: &str) -> Result<String, PyenvError> {
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

fn set_config_value(config: &mut AppConfig, key: &str, value: &str) -> Result<(), PyenvError> {
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

fn normalize_path_option(value: &str) -> Option<PathBuf> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(normalize_path_value(trimmed))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{
        AppConfig, RuntimeArch, config_path, get_config_value, load_config, resolve_cache_dir,
        resolve_versions_dir, save_config, set_config_value,
    };

    #[test]
    fn default_config_uses_root_versions_and_cache_dirs() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        let config = AppConfig::default();

        assert_eq!(resolve_versions_dir(&root, &config), root.join("versions"));
        assert_eq!(resolve_cache_dir(&root, &config), root.join("cache"));
    }

    #[test]
    fn config_round_trips_install_and_storage_settings() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        fs::create_dir_all(&root).expect("root");

        let mut config = AppConfig::default();
        set_config_value(&mut config, "storage.versions_dir", "python-builds").expect("set");
        set_config_value(&mut config, "storage.cache_dir", "cache-downloads").expect("set");
        set_config_value(&mut config, "install.arch", "arm64").expect("set");
        set_config_value(
            &mut config,
            "install.python_build_path",
            "../pyenv/plugins/python-build/bin/python-build",
        )
        .expect("set");
        set_config_value(&mut config, "install.bootstrap_pip", "false").expect("set");
        save_config(&root, &config).expect("save");
        let loaded = load_config(&root).expect("load");

        assert_eq!(
            get_config_value(&loaded, "storage.versions_dir").expect("get"),
            "python-builds"
        );
        assert_eq!(
            get_config_value(&loaded, "storage.cache_dir").expect("get"),
            "cache-downloads"
        );
        assert_eq!(
            get_config_value(&loaded, "install.arch").expect("get"),
            "arm64"
        );
        assert_eq!(
            get_config_value(&loaded, "install.python_build_path").expect("get"),
            "../pyenv/plugins/python-build/bin/python-build"
        );
        assert_eq!(
            get_config_value(&loaded, "install.bootstrap_pip").expect("get"),
            "false"
        );
        assert_eq!(loaded.install.arch, RuntimeArch::Arm64);
        assert_eq!(
            loaded.install.python_build_path,
            Some(std::path::PathBuf::from(
                "../pyenv/plugins/python-build/bin/python-build"
            ))
        );
        assert_eq!(config_path(&root), root.join("config.toml"));
    }
}
