// ./crates/pyenv-core/src/executable/tests.rs
//! Regression coverage for managed-runtime, system, and hook-based executable lookup.

#[cfg(test)]
mod tests {
    use std::env;
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::super::commands::{cmd_whence, cmd_which};
    use super::super::lookup::find_command_in_version;

    fn test_path_ext() -> Option<OsString> {
        if cfg!(windows) {
            Some(OsString::from(".exe;.bat;.cmd"))
        } else {
            None
        }
    }

    fn command_file(name: &str) -> String {
        if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_string()
        }
    }

    fn test_context() -> (TempDir, AppContext) {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        let dir = temp.path().join("work");
        let system_bin = temp.path().join("system-bin");
        fs::create_dir_all(root.join("versions")).expect("versions dir");
        fs::create_dir_all(&dir).expect("work dir");
        fs::create_dir_all(&system_bin).expect("system bin");

        let ctx = AppContext {
            root,
            dir,
            exe_path: PathBuf::from("pyenv"),
            env_version: None,
            env_shell: None,
            path_env: Some(env::join_paths([system_bin.clone()]).expect("path env")),
            path_ext: test_path_ext(),
            config: AppConfig::default(),
        };

        (temp, ctx)
    }

    #[test]
    fn which_finds_version_root_and_scripts_commands() {
        let (_temp, mut ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.1");
        let python_path = if cfg!(windows) {
            fs::create_dir_all(version_dir.join("Scripts")).expect("scripts");
            let path = version_dir.join("python.exe");
            fs::write(&path, "").expect("python");
            fs::write(version_dir.join("Scripts").join("pip.exe"), "").expect("pip");
            path
        } else {
            fs::create_dir_all(version_dir.join("bin")).expect("bin");
            let path = version_dir.join("bin").join("python");
            fs::write(&path, "").expect("python");
            fs::write(version_dir.join("bin").join("pip"), "").expect("pip");
            path
        };
        let pip_path = if cfg!(windows) {
            version_dir.join("Scripts").join("pip.exe")
        } else {
            version_dir.join("bin").join("pip")
        };
        ctx.env_version = Some("3.12.1".to_string());

        let python_report = cmd_which(&ctx, "python", false, false);
        assert_eq!(python_report.exit_code, 0);
        assert_eq!(PathBuf::from(&python_report.stdout[0]), python_path);

        let pip_report = cmd_which(&ctx, "pip", false, false);
        assert_eq!(pip_report.exit_code, 0);
        assert_eq!(PathBuf::from(&pip_report.stdout[0]), pip_path);

        let python = find_command_in_version(&ctx, "3.12.1", "python").expect("python");
        assert_eq!(python, python_path);
    }

    #[test]
    fn which_falls_back_to_system_path_without_shims() {
        let (_temp, ctx) = test_context();
        let system_path = PathBuf::from(ctx.path_env.clone().expect("path env"));
        let ruff_path = system_path.join(command_file("ruff"));
        fs::write(&ruff_path, "").expect("ruff");

        let report = cmd_which(&ctx, "ruff", false, false);
        assert_eq!(report.exit_code, 0);
        assert_eq!(PathBuf::from(&report.stdout[0]), ruff_path);
    }

    #[test]
    fn which_can_skip_system_lookup() {
        let (_temp, ctx) = test_context();
        let system_path = PathBuf::from(ctx.path_env.clone().expect("path env"));
        fs::write(system_path.join(command_file("ruff")), "").expect("ruff");

        let report = cmd_which(&ctx, "ruff", true, false);
        assert_eq!(report.exit_code, 127);
        assert!(
            report
                .stderr
                .iter()
                .any(|line| line.contains("command not found"))
        );
    }

    #[test]
    fn whence_lists_versions_in_ascending_order() {
        let (_temp, ctx) = test_context();
        for version in ["2.7", "3.4"] {
            let version_dir = if cfg!(windows) {
                ctx.versions_dir().join(version)
            } else {
                ctx.versions_dir().join(version).join("bin")
            };
            fs::create_dir_all(&version_dir).expect("bin");
            fs::write(version_dir.join(command_file("python")), "").expect("python");
        }

        let report = cmd_whence(&ctx, "python", false);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec!["2.7".to_string(), "3.4".to_string()]);
    }

    #[test]
    fn which_reports_advice_from_other_versions() {
        let (_temp, mut ctx) = test_context();
        let version_dir = if cfg!(windows) {
            ctx.versions_dir().join("3.4")
        } else {
            ctx.versions_dir().join("3.4").join("bin")
        };
        fs::create_dir_all(&version_dir).expect("bin");
        fs::write(version_dir.join("py.test"), "").expect("py.test");
        ctx.env_version = Some("2.7".to_string());

        let report = cmd_which(&ctx, "py.test", false, false);
        assert_eq!(report.exit_code, 127);
        assert!(
            report
                .stderr
                .iter()
                .any(|line| line.contains("version `2.7' is not installed"))
        );
        assert!(report
            .stderr
            .iter()
            .any(|line| line.contains("The `py.test' command exists in these Python versions:")));
        assert!(report.stderr.iter().any(|line| line.trim() == "3.4"));
    }

    #[test]
    fn which_skip_advice_suppresses_other_version_hints() {
        let (_temp, mut ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.4").join("bin");
        fs::create_dir_all(&version_dir).expect("bin");
        fs::write(version_dir.join("py.test"), "").expect("py.test");
        ctx.env_version = Some("2.7".to_string());

        let report = cmd_which(&ctx, "py.test", false, true);
        assert_eq!(report.exit_code, 127);
        assert_eq!(
            report.stderr,
            vec![
                "pyenv: version `2.7' is not installed (set by PYENV_VERSION environment variable)"
                    .to_string(),
                "pyenv: py.test: command not found".to_string(),
            ]
        );
        assert!(
            report
                .stderr
                .iter()
                .all(|line| !line.contains("exists in these Python versions"))
        );
    }

    #[test]
    fn which_hook_can_override_command_path() {
        let (_temp, mut ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.1");
        let hook_dir = ctx.root.join("pyenv.d").join("which");
        let override_path = if cfg!(windows) {
            ctx.root.join("override.exe")
        } else {
            ctx.root.join("override")
        };
        fs::create_dir_all(&version_dir).expect("version");
        fs::create_dir_all(&hook_dir).expect("hook dir");
        if cfg!(windows) {
            fs::write(version_dir.join("python.exe"), "").expect("python");
            fs::write(
                hook_dir.join("override.cmd"),
                format!("@echo PYENV_COMMAND_PATH={}", override_path.display()),
            )
            .expect("hook");
        } else {
            fs::write(version_dir.join("python"), "").expect("python");
            fs::write(
                hook_dir.join("override.sh"),
                format!(
                    "#!/usr/bin/env sh\necho PYENV_COMMAND_PATH={}\n",
                    override_path.display()
                ),
            )
            .expect("hook");
        }
        fs::write(&override_path, "").expect("override");
        ctx.env_version = Some("3.12.1".to_string());

        let report = cmd_which(&ctx, "python", false, false);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec![override_path.display().to_string()]);
    }
}
