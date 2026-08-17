// ./crates/pyenv-core/src/error.rs
//! Error types for the native pyenv core.

use std::collections::HashMap;
use std::fmt;

use crate::i18n;

#[derive(Debug)]
pub enum PyenvError {
    MissingHome,
    InvalidDirectory(String),
    InvalidVersion(String, String),
    NoLocalVersion,
    VersionNotInstalled(String, String),
    UnknownConfigKey(String),
    InvalidConfigValue {
        key: String,
        value: String,
    },
    VersionAlreadyInstalled(String),
    UnknownVersion(String),
    UnsupportedInstallTarget(String),
    MissingInstallVersion,
    MissingPythonBuildBackend,
    ChecksumMismatch {
        url: String,
        algorithm: String,
        expected: String,
        actual: String,
    },
    MissingChecksum(String),
    Io(String),
}

impl PyenvError {
    pub fn message_id(&self) -> &'static str {
        match self {
            Self::MissingHome => "error-missing-home",
            Self::InvalidDirectory(_) => "error-invalid-directory",
            Self::InvalidVersion(_, _) => "error-invalid-version",
            Self::NoLocalVersion => "error-no-local-version",
            Self::VersionNotInstalled(_, _) => "error-version-not-installed",
            Self::UnknownConfigKey(_) => "error-unknown-config-key",
            Self::InvalidConfigValue { .. } => "error-invalid-config-value",
            Self::VersionAlreadyInstalled(_) => "error-version-already-installed",
            Self::UnknownVersion(_) => "error-unknown-version",
            Self::UnsupportedInstallTarget(_) => "error-unsupported-install-target",
            Self::MissingInstallVersion => "error-missing-install-version",
            Self::MissingPythonBuildBackend => "error-missing-python-build",
            Self::ChecksumMismatch { .. } => "error-checksum-mismatch",
            Self::MissingChecksum(_) => "error-missing-checksum",
            Self::Io(_) => "error-io",
        }
    }

    pub fn message_args(&self) -> HashMap<String, String> {
        match self {
            Self::InvalidDirectory(path) => map_args(&[("path", path.as_str())]),
            Self::InvalidVersion(version, path) => {
                map_args(&[("version", version.as_str()), ("path", path.as_str())])
            }
            Self::VersionNotInstalled(version, origin) => {
                map_args(&[("version", version.as_str()), ("origin", origin.as_str())])
            }
            Self::UnknownConfigKey(key) => map_args(&[("key", key.as_str())]),
            Self::InvalidConfigValue { key, value } => {
                map_args(&[("key", key.as_str()), ("value", value.as_str())])
            }
            Self::VersionAlreadyInstalled(version) | Self::UnknownVersion(version) => {
                map_args(&[("version", version.as_str())])
            }
            Self::UnsupportedInstallTarget(version) => map_args(&[("version", version.as_str())]),
            Self::ChecksumMismatch {
                url,
                algorithm,
                expected,
                actual,
            } => map_args(&[
                ("url", url.as_str()),
                ("algorithm", algorithm.as_str()),
                ("expected", expected.as_str()),
                ("actual", actual.as_str()),
            ]),
            Self::MissingChecksum(source) => map_args(&[("source", source.as_str())]),
            Self::Io(message) => map_args(&[("message", message.as_str())]),
            _ => HashMap::new(),
        }
    }

    pub fn localized_message(&self) -> String {
        i18n::format_message(self.message_id(), &self.message_args())
    }
}

impl fmt::Display for PyenvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.localized_message())
    }
}

impl std::error::Error for PyenvError {}

fn map_args(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}
