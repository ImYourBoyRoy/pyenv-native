// ./crates/pyenv-core/src/catalog/tests.rs
//! Regression coverage for version sorting, latest resolution, and grouped install listings.

#[cfg(test)]
mod tests {
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
}
