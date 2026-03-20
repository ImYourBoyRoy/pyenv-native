// ./crates/pyenv-core/src/manage/tests.rs
//! Regression tests for installed-version listing, prefix resolution, and uninstall behavior.

#[cfg(test)]
mod tests {
    use std::env;
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::super::commands::{cmd_prefix, cmd_uninstall, cmd_versions};
    use super::super::types::VersionsCommandOptions;

    fn test_path_ext() -> Option<OsString> {
        if cfg!(windows) {
            Some(OsString::from(".exe;.cmd;.bat"))
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
        let system_bin = temp.path().join("system");
        fs::create_dir_all(root.join("versions")).expect("versions");
        fs::create_dir_all(&dir).expect("dir");
        fs::create_dir_all(&system_bin).expect("system");

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
    fn prefix_resolves_selected_version_directory() {
        let (_temp, mut ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.6");
        fs::create_dir_all(&version_dir).expect("version");
        ctx.env_version = Some("3.12".to_string());

        let report = cmd_prefix(&ctx, &[]);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec![version_dir.display().to_string()]);
    }

    #[test]
    fn versions_marks_current_and_lists_envs() {
        let (_temp, mut ctx) = test_context();
        fs::write(
            PathBuf::from(ctx.path_env.clone().expect("path env")).join(command_file("python")),
            "",
        )
        .expect("system python");
        fs::create_dir_all(ctx.versions_dir().join("3.12.6")).expect("version");
        fs::create_dir_all(ctx.root.join("venvs").join("3.12.6").join("demo")).expect("env");
        fs::create_dir_all(ctx.versions_dir().join("3.13.2")).expect("version");
        ctx.env_version = Some("3.12.6".to_string());

        let report = cmd_versions(&ctx, &VersionsCommandOptions::default());
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout[0], "  system");
        assert!(
            report
                .stdout
                .iter()
                .any(|line| line.starts_with("* 3.12.6 "))
        );
        assert!(
            report
                .stdout
                .iter()
                .any(|line| line.trim() == "3.12.6/envs/demo")
        );
    }

    #[test]
    fn versions_executables_are_deduplicated() {
        let (_temp, ctx) = test_context();
        if cfg!(windows) {
            fs::create_dir_all(ctx.versions_dir().join("3.12.6").join("Scripts")).expect("scripts");
            fs::create_dir_all(ctx.versions_dir().join("3.13.2").join("Scripts")).expect("scripts");
            fs::write(ctx.versions_dir().join("3.12.6").join("python.exe"), "").expect("python");
            fs::write(
                ctx.versions_dir()
                    .join("3.12.6")
                    .join("Scripts")
                    .join("pip.cmd"),
                "",
            )
            .expect("pip");
            fs::write(ctx.versions_dir().join("3.13.2").join("python.exe"), "").expect("python");
        } else {
            fs::create_dir_all(ctx.versions_dir().join("3.12.6").join("bin")).expect("bin");
            fs::create_dir_all(ctx.versions_dir().join("3.13.2").join("bin")).expect("bin");
            fs::write(
                ctx.versions_dir().join("3.12.6").join("bin").join("python"),
                "",
            )
            .expect("python");
            fs::write(
                ctx.versions_dir().join("3.12.6").join("bin").join("pip"),
                "",
            )
            .expect("pip");
            fs::write(
                ctx.versions_dir().join("3.13.2").join("bin").join("python"),
                "",
            )
            .expect("python");
        }

        let report = cmd_versions(
            &ctx,
            &VersionsCommandOptions {
                executables: true,
                ..VersionsCommandOptions::default()
            },
        );

        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec!["pip".to_string(), "python".to_string()]);
    }

    #[test]
    fn uninstall_force_removes_version_directory() {
        let (_temp, ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.6");
        if cfg!(windows) {
            fs::create_dir_all(version_dir.join("Scripts")).expect("version");
            fs::write(version_dir.join("python.exe"), "").expect("python");
        } else {
            fs::create_dir_all(version_dir.join("bin")).expect("version");
            fs::write(version_dir.join("bin").join("python"), "").expect("python");
        }

        let report = cmd_uninstall(&ctx, &[String::from("3.12.6")], true);
        assert_eq!(report.exit_code, 0);
        assert!(
            report
                .stdout
                .iter()
                .any(|line| line.contains("3.12.6 uninstalled"))
        );
        assert!(!version_dir.exists());
    }

    #[test]
    fn uninstall_blocks_runtime_when_managed_venvs_depend_on_it() {
        let (_temp, ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.6");
        fs::create_dir_all(&version_dir).expect("version");
        let managed_env = ctx.root.join("venvs").join("3.12.6").join("demo");
        fs::create_dir_all(&managed_env).expect("managed env");

        let report = cmd_uninstall(&ctx, &[String::from("3.12.6")], false);
        assert_eq!(report.exit_code, 1);
        assert!(
            report
                .stderr
                .iter()
                .any(|line| line.contains("has managed venvs that depend on it"))
        );
        assert!(version_dir.exists());
        assert!(managed_env.exists());
    }

    #[test]
    fn uninstall_force_removes_dependent_managed_venvs() {
        let (_temp, ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.6");
        fs::create_dir_all(&version_dir).expect("version");
        let managed_env = ctx.root.join("venvs").join("3.12.6").join("demo");
        fs::create_dir_all(&managed_env).expect("managed env");

        let report = cmd_uninstall(&ctx, &[String::from("3.12.6")], true);
        assert_eq!(report.exit_code, 0);
        assert!(
            report
                .stdout
                .iter()
                .any(|line| line.contains("removed dependent managed venv 3.12.6/envs/demo"))
        );
        assert!(!version_dir.exists());
        assert!(!managed_env.exists());
    }
}
