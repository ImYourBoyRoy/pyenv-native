// ./crates/pyenv-core/src/config/types.rs
//! Shared config models and enum parsing helpers for persisted pyenv-native settings.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "disabled" | "none" | "off" | "false" => Some(Self::Disabled),
            "pep514" | "pep-514" => Some(Self::Pep514),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
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
