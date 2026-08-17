// ./crates/pyenv-core/src/doctor/types.rs
//! Shared report and fix models for doctor diagnostics.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DoctorStatus {
    Ok,
    Warn,
    Info,
}

impl DoctorStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Warn => "WARN",
            Self::Info => "INFO",
        }
    }
}

/// How doctor should interpret process PATH (CLI vs desktop GUI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DoctorOptions {
    /// True when diagnostics run inside pyenv-gui. Desktop launches rarely inherit shell PATH.
    pub desktop_session: bool,
    /// Override profile detection in tests. `None` inspects the user home directory.
    pub profiles_configured: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub status: DoctorStatus,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorReport {
    pub root: String,
    pub platform: String,
    pub installed_versions: usize,
    pub checks: Vec<DoctorCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorFix {
    pub key: String,
    pub automated: bool,
    pub description: String,
    pub command_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorFixOutcome {
    pub applied: Vec<String>,
    pub manual: Vec<DoctorFix>,
}
