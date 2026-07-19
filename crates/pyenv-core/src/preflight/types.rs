// ./crates/pyenv-core/src/preflight/types.rs
//! Structured platform intelligence and install-preflight models shared by CLI, MCP, and GUI.

use serde::Serialize;

use crate::doctor::{DoctorCheck, DoctorFix, DoctorStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PreflightVerdict {
    Ready,
    NeedsAttention,
    Blocked,
}

impl PreflightVerdict {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "READY",
            Self::NeedsAttention => "NEEDS ATTENTION",
            Self::Blocked => "BLOCKED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlatformFact {
    pub key: String,
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlatformIntelligence {
    pub os: String,
    pub arch: String,
    pub os_pretty_name: String,
    pub shell: Option<String>,
    pub pyenv_root: String,
    pub install_strategy: String,
    pub source_build_required: bool,
    pub ready_to_install: bool,
    pub verdict: PreflightVerdict,
    pub summary: String,
    pub facts: Vec<PlatformFact>,
    pub checks: Vec<DoctorCheck>,
    pub blocking_issues: Vec<String>,
    pub warnings: Vec<String>,
    pub recommended_actions: Vec<DoctorFix>,
}

impl PlatformIntelligence {
    pub(crate) fn derive_verdict(
        checks: &[DoctorCheck],
        blocking_issues: &[String],
    ) -> PreflightVerdict {
        if !blocking_issues.is_empty()
            || checks
                .iter()
                .any(|check| check.status == DoctorStatus::Warn && is_blocking_check(&check.name))
        {
            return PreflightVerdict::Blocked;
        }
        if checks
            .iter()
            .any(|check| check.status == DoctorStatus::Warn)
        {
            return PreflightVerdict::NeedsAttention;
        }
        PreflightVerdict::Ready
    }
}

fn is_blocking_check(name: &str) -> bool {
    matches!(
        name,
        "source-build-shell"
            | "source-build-make"
            | "source-build-compiler"
            | "source-build-readiness"
            | "macos-xcode-clt"
            | "macos-openssl"
            | "termux-tool-clang"
            | "termux-tool-make"
            | "termux-tool-pkg-config"
            | "termux-lib-openssl"
            | "termux-lib-libffi"
            | "android-termux-prefix"
            | "android-source-build-readiness"
    )
}
