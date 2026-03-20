// ./crates/pyenv-core/src/self_update/types.rs
//! Shared self-update option and release-target models.

use std::cmp::Ordering;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfUpdateOptions {
    pub check: bool,
    pub yes: bool,
    pub force: bool,
    pub github_repo: Option<String>,
    pub tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct GitHubReleaseInfo {
    pub tag_name: String,
    pub html_url: Option<String>,
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
