// ./crates/pyenv-core/src/doctor/types.rs
//! Shared report and fix models for doctor diagnostics.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum DoctorStatus {
    Ok,
    Warn,
    Info,
}

impl DoctorStatus {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Warn => "WARN",
            Self::Info => "INFO",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct DoctorCheck {
    pub name: String,
    pub status: DoctorStatus,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct DoctorReport {
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
