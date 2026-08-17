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
    use super::super::types::{DoctorOptions, DoctorStatus};

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
        let checks = collect_checks_for_platform(&ctx, "linux", Default::default());
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
        let checks = collect_checks_for_platform(&ctx, "macos", Default::default());
        let python_build = checks
            .iter()
            .find(|check| check.name == "python-build-backend")
            .expect("python-build check");
        assert_eq!(python_build.status, DoctorStatus::Info);
    }

    #[test]
    fn doctor_warns_when_python_shim_matches_gui_launcher() {
        let (_temp, mut ctx) = test_context();
        let bin = ctx.root.join("bin");
        let cli = bin.join(if cfg!(windows) { "pyenv.exe" } else { "pyenv" });
        let gui = bin.join(if cfg!(windows) {
            "pyenv-gui.exe"
        } else {
            "pyenv-gui"
        });
        fs::write(&cli, "cli launcher").expect("cli");
        fs::write(&gui, "gui launcher").expect("gui");
        ctx.exe_path = gui.clone();

        if cfg!(windows) {
            fs::copy(&gui, ctx.shims_dir().join("python.exe")).expect("bad shim");
        } else {
            fs::write(
                ctx.shims_dir().join("python"),
                format!(
                    "#!/usr/bin/env sh\nexec '{}' exec \"$(basename \"$0\")\" \"$@\"\n",
                    gui.display()
                ),
            )
            .expect("bad shim");
        }

        let checks = collect_checks_for_platform(&ctx, env::consts::OS, Default::default());
        let integrity = checks
            .iter()
            .find(|check| check.name == "shim-launcher-integrity")
            .expect("integrity check");
        assert_eq!(integrity.status, DoctorStatus::Warn);
        assert!(integrity.detail.contains("pyenv-gui"));
    }

    #[test]
    fn doctor_reports_plugin_path_search() {
        let (_temp, mut ctx) = test_context();
        let checks = collect_checks_for_platform(&ctx, env::consts::OS, Default::default());
        let enabled = checks
            .iter()
            .find(|check| check.name == "plugin-path-search")
            .expect("plugin path check");
        assert_eq!(enabled.status, DoctorStatus::Info);
        ctx.config.plugins.search_path = false;
        let checks = collect_checks_for_platform(&ctx, env::consts::OS, Default::default());
        let disabled = checks
            .iter()
            .find(|check| check.name == "plugin-path-search")
            .expect("plugin path check");
        assert_eq!(disabled.status, DoctorStatus::Ok);
    }

    #[test]
    fn desktop_session_downgrades_missing_process_path_when_profiles_exist() {
        crate::i18n::set_lang_tag("en-US");
        let (_temp, mut ctx) = test_context();
        ctx.path_env = Some(OsString::from(String::new()));
        let options = DoctorOptions {
            desktop_session: true,
            profiles_configured: Some(true),
        };
        let checks = collect_checks_for_platform(&ctx, env::consts::OS, options);
        let bin = checks
            .iter()
            .find(|check| check.name == "pyenv-bin-on-path")
            .expect("bin check");
        let shims = checks
            .iter()
            .find(|check| check.name == "shims-on-path")
            .expect("shims check");
        assert_eq!(bin.status, DoctorStatus::Info);
        assert_eq!(shims.status, DoctorStatus::Info);
        assert!(bin.detail.contains("desktop"));

        let cli_checks =
            collect_checks_for_platform(&ctx, env::consts::OS, DoctorOptions::default());
        let cli_bin = cli_checks
            .iter()
            .find(|check| check.name == "pyenv-bin-on-path")
            .expect("cli bin");
        assert_eq!(cli_bin.status, DoctorStatus::Warn);
    }

    fn windowsapps_python_stub(root: &std::path::Path) -> std::path::PathBuf {
        let alias_dir = root.join("Microsoft").join("WindowsApps");
        fs::create_dir_all(&alias_dir).expect("windowsapps dir");
        let stub = if cfg!(windows) {
            alias_dir.join("python.exe")
        } else {
            alias_dir.join("python")
        };
        fs::write(&stub, "").expect("store alias stub");
        alias_dir
    }

    #[test]
    fn windows_store_alias_is_info_when_shims_precede_it() {
        crate::i18n::set_lang_tag("en-US");
        let (_temp, mut ctx) = test_context();
        let alias_dir = windowsapps_python_stub(_temp.path());
        ctx.path_env =
            Some(env::join_paths([ctx.shims_dir(), alias_dir.clone()]).expect("path env"));
        let checks = collect_checks_for_platform(&ctx, "windows", DoctorOptions::default());
        let system_python = checks
            .iter()
            .find(|check| check.name == "system-python")
            .expect("system-python check");
        assert_eq!(system_python.status, DoctorStatus::Info);
        assert!(
            system_python.detail.contains("optional")
                || system_python.detail.to_ascii_lowercase().contains("shims"),
            "got {}",
            system_python.detail
        );
        assert!(!super::super::helpers::windows_store_alias_needs_manual_fix(&ctx));
    }

    #[test]
    fn windows_store_alias_warns_when_it_precedes_shims() {
        crate::i18n::set_lang_tag("en-US");
        let (_temp, mut ctx) = test_context();
        let alias_dir = windowsapps_python_stub(_temp.path());
        ctx.path_env =
            Some(env::join_paths([alias_dir.clone(), ctx.shims_dir()]).expect("path env"));
        let checks = collect_checks_for_platform(&ctx, "windows", DoctorOptions::default());
        let system_python = checks
            .iter()
            .find(|check| check.name == "system-python")
            .expect("system-python check");
        assert_eq!(system_python.status, DoctorStatus::Warn);
        assert!(
            system_python.detail.contains("WindowsApps"),
            "got {}",
            system_python.detail
        );
        assert!(super::super::helpers::windows_store_alias_needs_manual_fix(
            &ctx
        ));
    }
}
