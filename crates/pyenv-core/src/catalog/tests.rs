// ./crates/pyenv-core/src/catalog/tests.rs
//! Regression coverage for version sorting, latest resolution, and grouped install listings.

#[cfg(test)]
mod cases {
    use std::fs;

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::super::commands::{cmd_install_list, cmd_latest};
    use super::super::entries::{installed_version_names, known_version_names};
    use super::super::latest::{compare_version_names, latest_version_from_names};
    use super::super::types::{CatalogSourceKind, InstallListOptions};

    fn test_context() -> (TempDir, AppContext) {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        let dir = temp.path().join("work");
        fs::create_dir_all(root.join("versions")).expect("versions dir");
        fs::create_dir_all(&dir).expect("work dir");

        let ctx = AppContext {
            root,
            dir,
            exe_path: std::path::PathBuf::from("pyenv"),
            env_version: None,
            env_shell: None,
            path_env: None,
            path_ext: None,
            config: AppConfig::default(),
        };

        (temp, ctx)
    }

    #[test]
    fn latest_prefers_highest_cpython_match() {
        let names = vec!["3.5.6", "3.10.8", "3.10.6"];
        assert_eq!(
            latest_version_from_names("3", &names),
            Some("3.10.8".to_string())
        );
    }

    #[test]
    fn latest_filters_prereleases_and_t_variants_without_t_prefix() {
        let names = vec![
            "3.8.5-dev",
            "3.8.5-src",
            "3.8.5-latest",
            "3.8.5a2",
            "3.8.5b3",
            "3.8.5rc2",
            "3.8.5t",
            "3.8.1",
            "3.8.1/envs/demo",
        ];
        assert_eq!(
            latest_version_from_names("3.8", &names),
            Some("3.8.1".to_string())
        );
    }

    #[test]
    fn latest_honors_t_suffix_requests() {
        let names = vec!["3.13.2t", "3.13.5", "3.13.5t", "3.14.6"];
        assert_eq!(
            latest_version_from_names("3t", &names),
            Some("3.13.5t".to_string())
        );
    }

    #[test]
    fn latest_keeps_prereleases_out_of_stable_prefix_resolution() {
        let names = vec![
            "3.15.0a8".to_string(),
            "3.15.0-b4".to_string(),
            "3.15.0-rc1".to_string(),
            "3.15.0".to_string(),
            "3.16.0-a1".to_string(),
        ];

        assert_eq!(
            latest_version_from_names("3.15", &names),
            Some("3.15.0".to_string())
        );
        assert_eq!(latest_version_from_names("3.16", &names), None);
        assert_eq!(
            latest_version_from_names("3.16.0a1", &names),
            Some("3.16.0-a1".to_string())
        );
    }

    #[test]
    fn version_order_places_stable_after_alpha_beta_and_rc() {
        let mut values = vec![
            "3.15.0".to_string(),
            "3.15.0-rc1".to_string(),
            "3.15.0-b4".to_string(),
            "3.15.0a8".to_string(),
        ];
        values.sort_by(|lhs, rhs| compare_version_names(lhs, rhs));
        assert_eq!(
            values,
            vec![
                "3.15.0a8".to_string(),
                "3.15.0-b4".to_string(),
                "3.15.0-rc1".to_string(),
                "3.15.0".to_string(),
            ]
        );
    }

    #[test]
    fn compare_version_names_orders_versions_naturally() {
        let mut values = vec![
            "3.10.8".to_string(),
            "3.5.6".to_string(),
            "3.10.6".to_string(),
        ];
        values.sort_by(|lhs, rhs| compare_version_names(lhs, rhs));
        assert_eq!(values, vec!["3.5.6", "3.10.6", "3.10.8"]);
    }

    #[test]
    fn install_list_groups_families() {
        let (_temp, ctx) = test_context();
        let report = cmd_install_list(
            &ctx,
            &InstallListOptions {
                family: Some("cpython".to_string()),
                json: false,
                pattern: Some("3.13".to_string()),
            },
        );

        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout[0], "Available versions:");
        assert!(report.stdout.iter().any(|line| line == "CPython"));
        assert!(
            report
                .stdout
                .iter()
                .any(|line| line.trim_start().starts_with("3.13."))
        );
        assert!(report.stdout.iter().all(|line| !line.contains("PyPy")));
    }

    #[test]
    fn install_list_json_is_grouped() {
        let (_temp, ctx) = test_context();
        let report = cmd_install_list(
            &ctx,
            &InstallListOptions {
                family: Some("pypy".to_string()),
                json: true,
                pattern: Some("pypy3.11".to_string()),
            },
        );

        assert_eq!(report.exit_code, 0);
        let payload = report.stdout.join("\n");
        assert!(payload.contains("\"family\": \"PyPy\""));
        assert!(payload.contains("\"source\": \"known\""));
    }

    #[test]
    fn installed_version_names_are_sorted() {
        let (_temp, ctx) = test_context();
        for version in ["3.10.8", "3.5.6", "3.10.6"] {
            fs::create_dir_all(ctx.versions_dir().join(version)).expect("version dir");
        }

        assert_eq!(
            installed_version_names(&ctx).expect("installed"),
            vec![
                "3.5.6".to_string(),
                "3.10.6".to_string(),
                "3.10.8".to_string()
            ]
        );
    }

    #[test]
    fn latest_command_supports_bypass() {
        let (_temp, ctx) = test_context();
        let report = cmd_latest(&ctx, "nonexistent", false, true, true);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec!["nonexistent"]);
    }

    #[test]
    fn catalog_source_kind_serializes_in_kebab_case() {
        let value = serde_json::to_string(&CatalogSourceKind::Installed).expect("serialize");
        assert_eq!(value, "\"installed\"");
    }

    #[test]
    fn known_versions_strip_utf8_bom_from_first_entry() {
        assert!(
            known_version_names()
                .first()
                .is_some_and(|value| !value.starts_with('\u{feff}'))
        );
    }

    #[test]
    fn latest_version_from_names_resolution() {
        let names = vec!["3.13.12".to_string()];

        assert_eq!(
            latest_version_from_names("3.13.12", &names),
            Some("3.13.12".to_string())
        );
        assert_eq!(
            latest_version_from_names("3.13", &names),
            Some("3.13.12".to_string())
        );
        assert_eq!(
            latest_version_from_names("3", &names),
            Some("3.13.12".to_string())
        );

        assert_eq!(latest_version_from_names("3.12", &names), None);
        assert_eq!(latest_version_from_names("4", &names), None);
    }

    #[test]
    fn python_org_index_keeps_release_directories_only() {
        let html = r#"
            <a href="../">../</a>
            <a href="src/">src/</a>
            <a href="3.14.8/">3.14.8/</a>
            <a href="3.15.0a1/">3.15.0a1/</a>
            <a href="3.15.0rc2/">3.15.0rc2/</a>
            <a href="README">README</a>
            <a href="https://example.com/3.12.0/">3.12.0/</a>
        "#;
        assert_eq!(
            super::super::remote::parse_python_org_ftp_index(html),
            vec![
                "3.12.0".to_string(),
                "3.14.8".to_string(),
                "3.15.0a1".to_string(),
                "3.15.0rc2".to_string(),
            ]
        );
    }

    #[test]
    fn python_org_release_index_reads_installable_source_archives() {
        let html = r#"
            <a href="../">../</a>
            <a href="Python-3.16.0a1.tgz">Python-3.16.0a1.tgz</a>
            <a href="Python-3.16.0a1.tar.xz">Python-3.16.0a1.tar.xz</a>
            <a href="Python-3.16.0a1.tgz.sig">Python-3.16.0a1.tgz.sig</a>
            <a href="Python-3.16.0b1.tgz">Python-3.16.0b1.tgz</a>
            <a href="Python-3.16.0rc1.tgz">Python-3.16.0rc1.tgz</a>
            <a href="Python-3.16.0.tgz">Python-3.16.0.tgz</a>
            <a href="python-3.16.0a1-amd64.zip">python-3.16.0a1-amd64.zip</a>
        "#;

        assert_eq!(
            super::super::remote::parse_python_org_release_index(html),
            vec![
                "3.16.0a1".to_string(),
                "3.16.0b1".to_string(),
                "3.16.0rc1".to_string(),
                "3.16.0".to_string(),
            ]
        );
    }

    #[test]
    fn legacy_publisher_cache_is_not_treated_as_authoritative() {
        let (_temp, ctx) = test_context();
        let cache_path = super::super::remote::cpython_index_cache_path(&ctx);
        fs::create_dir_all(cache_path.parent().expect("cache parent")).expect("cache dir");
        fs::write(&cache_path, r#"{"versions":["3.16.0"]}"#).expect("legacy cache");

        let report = cmd_latest(&ctx, "3.16", true, false, false);
        assert_eq!(report.exit_code, 1);
        assert!(report.stdout.is_empty());
        assert!(report.stderr[0].contains("no known versions"));
    }

    #[test]
    fn cpython_cache_rejects_archive_directory_mismatches() {
        let (_temp, ctx) = test_context();
        let cache_path = super::super::remote::cpython_index_cache_path(&ctx);
        fs::create_dir_all(cache_path.parent().expect("cache parent")).expect("cache dir");
        fs::write(
            &cache_path,
            r#"{
                "schema_version": 3,
                "releases": [{
                    "version": "9.99.0a1",
                    "directory": "9.99.0a1",
                    "filename": "Python-9.99.0a1.tgz"
                }]
            }"#,
        )
        .expect("mismatched cache");

        assert!(
            !super::super::remote::load_cpython_catalog_names(&ctx)
                .iter()
                .any(|version| version == "9.99.0a1")
        );
    }

    #[test]
    fn refreshed_publisher_cache_removes_phantom_stable_names_from_store() {
        let (_temp, ctx) = test_context();
        let cache_path = super::super::remote::cpython_index_cache_path(&ctx);
        super::super::remote::write_cpython_index_cache(&cache_path, &["3.16.0a1".to_string()])
            .expect("seed cpython prerelease cache");
        fs::write(
            super::super::remote::known_versions_store_path(&ctx),
            "3.16.0\n3.16.0t\n",
        )
        .expect("seed runtime known versions");

        let report = cmd_latest(&ctx, "3.16", true, false, false);
        assert_eq!(report.exit_code, 1);
        let store = fs::read_to_string(super::super::remote::known_versions_store_path(&ctx))
            .expect("runtime known versions");
        assert!(!store.lines().any(|line| line == "3.16.0"));
        assert!(!store.lines().any(|line| line == "3.16.0t"));
        assert!(store.lines().any(|line| line == "3.16.0a1"));
    }

    #[test]
    fn cached_publisher_index_adds_revisions_missing_from_the_seed() {
        let (_temp, ctx) = test_context();
        let cache_path = super::super::remote::cpython_index_cache_path(&ctx);
        super::super::remote::write_cpython_index_cache(&cache_path, &["9.9.1".to_string()])
            .expect("seed cache");

        let report = cmd_latest(&ctx, "9.9", true, false, false);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec!["9.9.1".to_string()]);
        assert!(
            !known_version_names()
                .iter()
                .any(|version| version == "9.9.1"),
            "9.9.1 must come from the publisher cache, not known_versions.txt"
        );

        let listed = cmd_install_list(
            &ctx,
            &InstallListOptions {
                family: Some("cpython".to_string()),
                json: false,
                pattern: Some("9.9.1".to_string()),
            },
        );
        assert_eq!(listed.exit_code, 0);
        assert!(listed.stdout.iter().any(|line| line.trim() == "9.9.1"));
        assert!(listed.stdout.iter().any(|line| line.trim() == "9.9.1t"));

        let store = fs::read_to_string(super::super::remote::known_versions_store_path(&ctx))
            .expect("runtime known versions");
        assert!(store.lines().any(|line| line == "9.9.1"));
        assert!(store.lines().any(|line| line == "2.7.18"));
        assert!(
            !known_version_names()
                .iter()
                .any(|version| version == "9.9.1")
        );

        fs::remove_file(cache_path).expect("drop publisher cache");
        let again = cmd_latest(&ctx, "9.9", true, false, false);
        assert_eq!(again.stdout, vec!["9.9.1".to_string()]);
    }

    #[test]
    fn version_patterns_do_not_match_inside_a_later_component() {
        use super::super::entries::matches_catalog_pattern;

        assert!(matches_catalog_pattern("3.15.0", "3.15"));
        assert!(matches_catalog_pattern("3.15.0t", "3.15"));
        assert!(matches_catalog_pattern("3.15.0rc1", "3.15.0rc"));
        assert!(!matches_catalog_pattern("3.13.15", "3.15"));
        assert!(!matches_catalog_pattern("3.13.14", "3.14"));
        assert!(matches_catalog_pattern("3.14.8", "3.14"));
        assert!(matches_catalog_pattern("pypy3.11-7.3.20", "pypy"));
        assert!(matches_catalog_pattern("CPython", "cpython"));
    }
}
