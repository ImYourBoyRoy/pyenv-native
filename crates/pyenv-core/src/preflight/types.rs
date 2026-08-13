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
        // Host Environment is install-readiness, not live shell/shim health.
        // PATH / shim / selected-version warnings belong in doctor + shell cards —
        // they must not mark a healthy Windows NuGet host as "Needs attention"
        // merely because a desktop GUI process did not inherit User PATH.
        if checks.iter().any(|check| {
            check.status == DoctorStatus::Warn && affects_install_readiness(&check.name)
        }) {
            return PreflightVerdict::NeedsAttention;
        }
        PreflightVerdict::Ready
    }
}

/// Checks that hard-block runtime installs when Warn.
pub(crate) fn is_blocking_check(name: &str) -> bool {
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

/// Soft install/toolchain prerequisites and install conflicts.
/// Excludes shell PATH, shim function, and selection health (shown elsewhere).
pub(crate) fn affects_install_readiness(name: &str) -> bool {
    is_blocking_check(name)
        || matches!(
            name,
            "root-directory" | "pyenv-win-root" | "pyenv-win-path-conflict"
        )
}
