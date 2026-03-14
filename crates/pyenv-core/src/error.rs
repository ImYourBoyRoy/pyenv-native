// ./crates/pyenv-core/src/error.rs
//! Error types for the native pyenv core.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PyenvError {
    #[error("pyenv: cannot determine home directory for PYENV_ROOT")]
    MissingHome,
    #[error("pyenv: cannot change working directory to `{0}`")]
    InvalidDirectory(String),
    #[error("pyenv: invalid version `{0}` ignored in `{1}`")]
    InvalidVersion(String, String),
    #[error("pyenv: no local version configured for this directory")]
    NoLocalVersion,
    #[error("pyenv: version `{0}` is not installed (set by {1})")]
    VersionNotInstalled(String, String),
    #[error("pyenv: unknown config key `{0}`")]
    UnknownConfigKey(String),
    #[error("pyenv: invalid value `{value}` for config key `{key}`")]
    InvalidConfigValue { key: String, value: String },
    #[error("pyenv: version `{0}` is already installed")]
    VersionAlreadyInstalled(String),
    #[error("pyenv: no known versions match `{0}`")]
    UnknownVersion(String),
    #[error("pyenv: install backend does not support `{0}` on this platform")]
    UnsupportedInstallTarget(String),
    #[error("pyenv: install operation requires at least one version argument")]
    MissingInstallVersion,
    #[error(
        "pyenv: unable to locate python-build backend; set install.python_build_path or add python-build to PATH"
    )]
    MissingPythonBuildBackend,
    #[error("{0}")]
    Io(String),
}
