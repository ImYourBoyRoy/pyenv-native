// ./crates/pyenv-core/src/doctor/tests.rs
//! Regression coverage for doctor reporting and non-Windows prerequisite diagnostics.

#[cfg(test)]
mod tests {
    use std::env;
    use std::ffi::OsString;
    use std::fs;

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::super::checks::collect_checks_for_platform;
    use super::super::report::cmd_doctor;
    use super::super::types::DoctorStatus;

    fn test_path_ext() -> Option<OsString> {
        if cfg!(windows) {
            Some(OsString::from(".exe;.cmd"))
        } else {
            None
        }
    }

    fn test_context() -> (TempDir, AppContext) {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        let dir = temp.path().join("work");
        fs::create_dir_all(root.join("versions")).expect("versions dir");
        fs::create_dir_all(root.join("shims")).expect("shims dir");
        fs::create_dir_all(root.join("bin")).expect("bin dir");
        fs::create_dir_all(&dir).expect("work dir");

        let ctx = AppContext {
            root: root.clone(),
            dir,
            exe_path: root
                .join("bin")
                .join(if cfg!(windows) { "pyenv.exe" } else { "pyenv" }),
            env_version: Some("3.12.10".to_string()),
            env_shell: None,
            path_env: Some(
                env::join_paths([root.join("bin"), root.join("shims")]).expect("path env"),
            ),
            path_ext: test_path_ext(),
            config: AppConfig::default(),
        };

        (temp, ctx)
    }

    #[test]
    fn doctor_reports_ok_for_bin_and_shims_on_path() {
        let (_temp, ctx) = test_context();
        let report = cmd_doctor(&ctx, false);
        assert_eq!(report.exit_code, 0);
        assert!(
            report
                .stdout
                .iter()
                .any(|line| line.contains("[OK] pyenv-bin-on-path"))
        );
        assert!(
            report
                .stdout
                .iter()
                .any(|line| line.contains("[OK] shims-on-path"))
        );
    }

    #[test]
    fn doctor_json_includes_checks() {
        let (_temp, mut ctx) = test_context();
        ctx.path_env = Some(OsString::from(String::new()));
        let report = cmd_doctor(&ctx, true);
        assert_eq!(report.exit_code, 0);
        let payload = report.stdout.join("\n");
        assert!(payload.contains("\"checks\""));
        assert!(payload.contains("\"pyenv-bin-on-path\""));
    }

    #[test]
    fn non_windows_doctor_reports_source_build_readiness() {
        let (_temp, ctx) = test_context();
        let checks = collect_checks_for_platform(&ctx, "linux");
        assert!(
            checks
                .iter()
                .any(|check| check.name == "source-build-shell")
        );
        assert!(checks.iter().any(|check| check.name == "source-build-make"));
        assert!(
            checks
                .iter()
                .any(|check| check.name == "source-build-compiler")
        );
        assert!(
            checks
                .iter()
                .any(|check| check.name == "source-build-readiness")
        );
    }

    #[test]
    fn non_windows_doctor_treats_missing_python_build_as_info() {
        let (_temp, mut ctx) = test_context();
        ctx.path_env = Some(OsString::from(String::new()));
        let checks = collect_checks_for_platform(&ctx, "macos");
        let python_build = checks
            .iter()
            .find(|check| check.name == "python-build-backend")
            .expect("python-build check");
        assert_eq!(python_build.status, DoctorStatus::Info);
    }
}
