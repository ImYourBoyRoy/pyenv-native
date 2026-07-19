// ./crates/pyenv-core/src/preflight/tests.rs
//! Coverage for platform intelligence and preflight gate helpers.

#[cfg(test)]
mod tests {
    use std::env;
    use std::ffi::OsString;
    use std::fs;

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;
    use crate::doctor::DoctorStatus;
    use crate::preflight::{build_platform_intelligence, cmd_preflight};

    fn test_context() -> (TempDir, AppContext) {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        let dir = temp.path().join("work");
        fs::create_dir_all(root.join("versions")).expect("versions");
        fs::create_dir_all(root.join("shims")).expect("shims");
        fs::create_dir_all(root.join("bin")).expect("bin");
        fs::create_dir_all(&dir).expect("work");

        let ctx = AppContext {
            root: root.clone(),
            dir,
            exe_path: root.join("bin").join("pyenv"),
            env_version: None,
            env_shell: Some("bash".to_string()),
            path_env: Some(env::join_paths([root.join("bin"), root.join("shims")]).expect("path")),
            path_ext: if cfg!(windows) {
                Some(OsString::from(".exe;.cmd"))
            } else {
                None
            },
            config: AppConfig::default(),
        };
        (temp, ctx)
    }

    #[test]
    fn preflight_json_includes_host_facts() {
        let (_temp, ctx) = test_context();
        let report = cmd_preflight(&ctx, true);
        assert_eq!(report.exit_code, 0);
        let payload = report.stdout.join("\n");
        assert!(payload.contains("\"facts\""));
        assert!(
            payload.contains("\"install_strategy\"") || payload.contains("\"install-strategy\"")
        );
        assert!(payload.contains("\"verdict\""));
    }

    #[test]
    fn platform_intelligence_includes_os_and_strategy() {
        let (_temp, ctx) = test_context();
        let intel = build_platform_intelligence(&ctx);
        assert!(!intel.os.is_empty());
        assert!(!intel.install_strategy.is_empty());
        assert!(!intel.facts.is_empty());
        assert!(
            intel
                .checks
                .iter()
                .any(|check| check.status == DoctorStatus::Ok
                    || check.status == DoctorStatus::Warn
                    || check.status == DoctorStatus::Info)
        );
    }
}
