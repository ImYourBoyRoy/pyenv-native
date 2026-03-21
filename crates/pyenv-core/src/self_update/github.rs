// ./crates/pyenv-core/src/self_update/github.rs
//! GitHub release metadata retrieval and release-target resolution for self-update.

use std::env;

use super::types::{GitHubReleaseInfo, ReleaseTarget};
use super::versioning::{compare_release_versions, normalize_tag};
use crate::http::build_blocking_client;

pub(super) const DEFAULT_GITHUB_REPO: &str = "imyourboyroy/pyenv-native";
const DEFAULT_GITHUB_API_BASE: &str = "https://api.github.com";

pub(super) fn resolve_release_target(
    repo: &str,
    requested_tag: Option<&str>,
) -> Result<ReleaseTarget, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let current_tag = format!("v{current_version}");

    let (target_tag, release_url) = if let Some(tag) = requested_tag {
        (normalize_tag(tag), None)
    } else {
        let release = fetch_latest_release_info(repo, None)?;
        (normalize_tag(&release.tag_name), release.html_url)
    };

    let comparison = compare_release_versions(&target_tag, &current_tag);
    Ok(ReleaseTarget {
        current_version,
        current_tag,
        target_tag,
        release_url,
        comparison,
        repo: repo.to_string(),
    })
}

pub(super) fn fetch_latest_release_info(
    repo: &str,
    api_base_url: Option<&str>,
) -> Result<GitHubReleaseInfo, String> {
    let base_url = api_base_url
        .unwrap_or(DEFAULT_GITHUB_API_BASE)
        .trim_end_matches('/');
    let url = format!("{base_url}/repos/{repo}/releases/latest");
    let client = github_client()?;
    let response = client
        .get(&url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("pyenv: failed to query latest release for {repo}: {error}"))?;
    let body = response
        .text()
        .map_err(|error| format!("pyenv: failed to read latest release metadata: {error}"))?;
    serde_json::from_str::<GitHubReleaseInfo>(&body)
        .map_err(|error| format!("pyenv: failed to parse latest release metadata: {error}"))
}

fn github_client() -> Result<reqwest::blocking::Client, String> {
    build_blocking_client()
        .map_err(|error| format!("pyenv: failed to construct HTTP client: {error}"))
}
