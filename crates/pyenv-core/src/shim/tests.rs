// ./crates/pyenv-core/src/shim/tests.rs
//! Regression coverage for shim generation, locking, PATH shaping, and exec overrides.

#[cfg(test)]
mod tests {
    use std::env;
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::super::exec::cmd_exec;
    use super::super::paths::adjusted_path;
    use super::super::rehash::{cmd_rehash, rehash_shims};
    use super::super::render::make_executable;
    use super::super::types::SHIM_LOCK_FILE;

    fn test_path_ext() -> Option<OsString> {
        if cfg!(windows) {
            Some(OsString::from(".exe;.cmd;.bat"))
        } else {
            None
        }
    }

    fn executable_name(name: &str) -> String {
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
        let exe_path = temp.path().join("pyenv.exe");
        fs::create_dir_all(root.join("versions")).expect("versions");
        fs::create_dir_all(&dir).expect("work");
        fs::write(&exe_path, "shim source").expect("exe");

        let ctx = AppContext {
            root,
            dir,
            exe_path,
            env_version: None,
            env_shell: None,
            path_env: None,
            path_ext: test_path_ext(),
            config: AppConfig::default(),
        };

        (temp, ctx)
    }

    #[test]
    fn rehash_generates_cmd_shims_and_manifest() {
        let (_temp, ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.6");
        if cfg!(windows) {
            fs::create_dir_all(version_dir.join("Scripts")).expect("scripts");
            fs::write(version_dir.join("python.exe"), "").expect("python");
            fs::write(version_dir.join("Scripts").join("pip.cmd"), "").expect("pip");
        } else {
            fs::create_dir_all(version_dir.join("bin")).expect("bin");
            fs::write(version_dir.join("bin").join("python"), "").expect("python");
            fs::write(version_dir.join("bin").join("pip"), "").expect("pip");
        }

        let count = rehash_shims(&ctx).expect("rehash");
        assert_eq!(count, 2);
        if cfg!(windows) {
            assert!(ctx.shims_dir().join("python.exe").is_file());
            assert!(ctx.shims_dir().join("python.cmd").is_file());
            assert!(ctx.shims_dir().join("python.bat").is_file());
            assert!(ctx.shims_dir().join("pip.cmd").is_file());
        } else {
            assert!(ctx.shims_dir().join("python").is_file());
            assert!(ctx.shims_dir().join("pip").is_file());
        }
        assert!(ctx.shims_dir().join(".pyenv-shims.json").is_file());
        assert!(!ctx.shims_dir().join(SHIM_LOCK_FILE).exists());

        let report = cmd_rehash(&ctx);
        assert_eq!(report.exit_code, 0);
    }

    #[test]
    fn rehash_removes_stale_shims() {
        let (_temp, ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.6");
        if cfg!(windows) {
            fs::create_dir_all(&version_dir).expect("version");
            fs::write(version_dir.join("python.exe"), "").expect("python");
        } else {
            fs::create_dir_all(version_dir.join("bin")).expect("version");
            fs::write(version_dir.join("bin").join("python"), "").expect("python");
        }

        rehash_shims(&ctx).expect("rehash");
        assert!(ctx.shims_dir().join(executable_name("python")).is_file());
        if cfg!(windows) {
            assert!(ctx.shims_dir().join("python.cmd").is_file());
        }

        fs::remove_dir_all(&version_dir).expect("remove version");
        rehash_shims(&ctx).expect("rehash");
        assert!(!ctx.shims_dir().join(executable_name("python")).exists());
        if cfg!(windows) {
            assert!(!ctx.shims_dir().join("python.cmd").exists());
        }
    }

    #[test]
    fn rehash_hooks_can_register_additional_commands() {
        let (_temp, ctx) = test_context();
        let hook_dir = ctx.root.join("pyenv.d").join("rehash");
        fs::create_dir_all(&hook_dir).expect("hook dir");
        if cfg!(windows) {
            fs::write(hook_dir.join("register.cmd"), "@echo extra-tool").expect("hook");
        } else {
            fs::write(
                hook_dir.join("register.sh"),
                "#!/usr/bin/env sh\necho extra-tool\n",
            )
            .expect("hook");
        }

        rehash_shims(&ctx).expect("rehash");
        if cfg!(windows) {
            assert!(ctx.shims_dir().join("extra-tool.exe").is_file());
            assert!(ctx.shims_dir().join("extra-tool.cmd").is_file());
            assert!(ctx.shims_dir().join("extra-tool.bat").is_file());
            assert!(ctx.shims_dir().join("extra-tool.ps1").is_file());
        } else {
            assert!(ctx.shims_dir().join("extra-tool").is_file());
        }
    }

    #[test]
    fn exec_hooks_can_override_target_and_set_environment() {
        let (_temp, mut ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.6");
        let hook_dir = ctx.root.join("pyenv.d").join("exec");
        let target_path = if cfg!(windows) {
            ctx.root.join("override.cmd")
        } else {
            ctx.root.join("override.sh")
        };
        let output_path = ctx.root.join("exec-output.txt");
        fs::create_dir_all(&version_dir).expect("version");
        fs::create_dir_all(&hook_dir).expect("hook dir");
        if cfg!(windows) {
            fs::write(version_dir.join("python.cmd"), "@echo base").expect("python");
            fs::write(
                &target_path,
                format!("@echo %DEMO_ENV%>{}", output_path.display()),
            )
            .expect("override");
            fs::write(
                hook_dir.join("redirect.cmd"),
                format!(
                    "@echo ENV:DEMO_ENV=from-hook\r\n@echo PYENV_COMMAND_PATH={}",
                    target_path.display()
                ),
            )
            .expect("hook");
        } else {
            fs::write(version_dir.join("python"), "#!/usr/bin/env sh\nexit 0\n").expect("python");
            fs::write(
                &target_path,
                format!(
                    "#!/usr/bin/env sh\nprintf '%s' \"$DEMO_ENV\" > '{}'\n",
                    output_path.display()
                ),
            )
            .expect("override");
            make_executable(&target_path).expect("target executable");
            fs::write(
                hook_dir.join("redirect.sh"),
                format!(
                    "#!/usr/bin/env sh\necho ENV:DEMO_ENV=from-hook\necho PYENV_COMMAND_PATH={}\n",
                    target_path.display()
                ),
            )
            .expect("hook");
        }
        ctx.env_version = Some("3.12.6".to_string());

        let report = cmd_exec(&ctx, "python", &[]);
        assert_eq!(report.exit_code, 0);
        assert_eq!(
            fs::read_to_string(output_path).expect("output").trim(),
            "from-hook"
        );
    }

    #[test]
    fn rehash_fails_when_fresh_lock_exists() {
        let (_temp, ctx) = test_context();
        fs::create_dir_all(ctx.shims_dir()).expect("shims dir");
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        fs::write(
            ctx.shims_dir().join(SHIM_LOCK_FILE),
            format!("pid=9999\ncreated_at={created_at}\n"),
        )
        .expect("lock");

        let error = rehash_shims(&ctx).expect_err("rehash should fail");
        assert!(error.to_string().contains("cannot rehash"));
    }

    #[test]
    fn rehash_replaces_stale_lock_file() {
        let (_temp, ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.6");
        fs::create_dir_all(&version_dir).expect("version");
        fs::write(version_dir.join("python.exe"), "").expect("python");
        fs::create_dir_all(ctx.shims_dir()).expect("shims dir");
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(60 * 60);
        fs::write(
            ctx.shims_dir().join(SHIM_LOCK_FILE),
            format!("pid=9999\ncreated_at={created_at}\n"),
        )
        .expect("lock");

        let count = rehash_shims(&ctx).expect("rehash");
        assert_eq!(count, 1);
        assert!(!ctx.shims_dir().join(SHIM_LOCK_FILE).exists());
    }

    #[test]
    fn adjusted_path_deduplicates_prefix_and_existing_entries() {
        let (_temp, mut ctx) = test_context();
        let first = ctx.root.join("versions").join("3.12.6").join("Scripts");
        let first = if cfg!(windows) {
            first
        } else {
            ctx.root.join("versions").join("3.12.6").join("bin")
        };
        let second = if cfg!(windows) {
            PathBuf::from("C:\\Windows\\System32")
        } else {
            PathBuf::from("/usr/bin")
        };
        ctx.path_env = Some(
            env::join_paths([
                first.clone(),
                ctx.shims_dir(),
                second.clone(),
                first.clone(),
            ])
            .expect("path env"),
        );

        let joined = adjusted_path(&ctx, &[first.clone(), second.clone(), first.clone()])
            .expect("adjusted path");
        let entries = env::split_paths(&joined).collect::<Vec<_>>();
        assert_eq!(entries, vec![first, second]);
    }
}
