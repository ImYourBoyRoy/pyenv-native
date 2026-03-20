// ./crates/pyenv-core/src/self_update/tests.rs
//! Regression coverage for self-update tag normalization, version comparison, and release parsing.

#[cfg(test)]
mod tests {
    use mockito::Server;

    use super::super::github::fetch_latest_release_info;
    use super::super::versioning::{compare_release_versions, normalize_tag, parse_semverish};

    #[test]
    fn normalize_tag_adds_v_prefix_once() {
        assert_eq!(normalize_tag("0.1.8"), "v0.1.8");
        assert_eq!(normalize_tag("v0.1.8"), "v0.1.8");
        assert_eq!(normalize_tag("V0.1.8"), "v0.1.8");
    }

    #[test]
    fn compare_release_versions_honors_semver_ordering() {
        assert!(compare_release_versions("v0.1.9", "v0.1.8").is_gt());
        assert!(compare_release_versions("v0.1.8", "v0.1.8").is_eq());
        assert!(compare_release_versions("v0.1.8", "v0.1.9").is_lt());
        assert!(compare_release_versions("v0.2.0", "v0.1.99").is_gt());
        assert!(compare_release_versions("v0.2.0", "v0.2.0-rc.1").is_gt());
    }

    #[test]
    fn parse_semverish_treats_missing_segments_as_zero() {
        let parsed = parse_semverish("v1.2");
        assert_eq!(parsed.numeric, vec![1, 2]);
        assert_eq!(parsed.pre_release, None);
    }

    #[test]
    fn fetch_latest_release_info_reads_github_api_payload() {
        let mut server = Server::new();
        let _mock = server
            .mock("GET", "/repos/imyourboyroy/pyenv-native/releases/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"tag_name":"v0.1.9","html_url":"https://example.test/release"}"#)
            .create();

        let release = fetch_latest_release_info("imyourboyroy/pyenv-native", Some(&server.url()))
            .expect("latest release");

        assert_eq!(release.tag_name, "v0.1.9");
        assert_eq!(
            release.html_url.as_deref(),
            Some("https://example.test/release")
        );
    }
}
