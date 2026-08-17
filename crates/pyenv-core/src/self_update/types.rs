// ./crates/pyenv-core/src/self_update/types.rs
//! Shared self-update option and release-target models.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfUpdateOptions {
    pub check: bool,
    pub yes: bool,
    pub force: bool,
    pub github_repo: Option<String>,
    pub tag: Option<String>,
    /// When true, POSIX self-update spawns a background updater and relaunches the GUI.
    pub restart_gui: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct GitHubReleaseInfo {
    pub tag_name: String,
    pub html_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SelfUpdateCheckStatus {
    Current,
    Available,
    Ahead,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SelfUpdateCheck {
    pub status: SelfUpdateCheckStatus,
    pub current_tag: String,
    pub target_tag: String,
    pub release_url: Option<String>,
}

impl SelfUpdateCheck {
    pub(super) fn from_target(target: &ReleaseTarget) -> Self {
        let status = match target.comparison {
            Ordering::Equal => SelfUpdateCheckStatus::Current,
            Ordering::Greater => SelfUpdateCheckStatus::Available,
            Ordering::Less => SelfUpdateCheckStatus::Ahead,
        };
        Self {
            status,
            current_tag: target.current_tag.clone(),
            target_tag: target.target_tag.clone(),
            release_url: target.release_url.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReleaseTarget {
    pub current_version: String,
    pub current_tag: String,
    pub target_tag: String,
    pub release_url: Option<String>,
    pub comparison: Ordering,
    pub repo: String,
}
