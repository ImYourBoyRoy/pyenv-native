// ./crates/pyenv-core/src/runtime/tests.rs
//! Regression tests for runtime search roots, prefix lookup, shim inventory, and trap skipping.

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::super::{
        BASE_VENV_DIR_NAME, collect_shim_names_from_prefix, find_command_in_prefix,
        inventory_roots_for_version, managed_search_roots_for_version, prefix_bin_dirs,
        search_path_entries,
    };

    fn test_path_ext() -> Option<OsString> {
        if cfg!(windows) {
            Some(OsString::from(".exe;.bat;.cmd"))
        } else {
            None
        }
    }

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
            path_ext: test_path_ext(),
            config: AppConfig::default(),
        };

        (temp, ctx)
    }

    #[test]
    fn search_roots_prefer_base_venv_when_enabled() {
        let (_temp, mut ctx) = test_context();
        ctx.config.venv.auto_use_base_venv = true;
        let version_dir = ctx.versions_dir().join("3.12.1");
        fs::create_dir_all(version_dir.join(BASE_VENV_DIR_NAME)).expect("base venv");

        let roots = managed_search_roots_for_version(&ctx, "3.12.1");
        assert_eq!(
            roots,
            vec![version_dir.join(BASE_VENV_DIR_NAME), version_dir,]
        );
    }

    #[test]
    fn inventory_roots_include_top_level_managed_venvs_for_runtime() {
        let (_temp, ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.1");
        let managed_env = ctx.root.join("venvs").join("3.12.1").join("demo");
        fs::create_dir_all(&version_dir).expect("version");
        fs::create_dir_all(&managed_env).expect("managed env");

        let roots = inventory_roots_for_version(&ctx, "3.12.1");
        assert!(roots.contains(&version_dir));
        assert!(roots.contains(&managed_env));
    }

    #[test]
    fn find_command_checks_prefix_root_and_scripts() {
        let temp = TempDir::new().expect("tempdir");
        let prefix = temp.path().join("runtime");
        let python_path = if cfg!(windows) {
            fs::create_dir_all(prefix.join("Scripts")).expect("scripts");
            let path = prefix.join("python.exe");
            fs::write(&path, "").expect("python");
            fs::write(prefix.join("Scripts").join("pip.cmd"), "").expect("pip");
            path
        } else {
            fs::create_dir_all(prefix.join("bin")).expect("bin");
            let path = prefix.join("python");
            fs::write(&path, "").expect("python");
            fs::write(prefix.join("bin").join("pip"), "").expect("pip");
            path
        };
        let pip_path = if cfg!(windows) {
            prefix.join("Scripts").join("pip.cmd")
        } else {
            prefix.join("bin").join("pip")
        };

        let python =
            find_command_in_prefix(&prefix, "python", test_path_ext().as_deref()).expect("python");
        let pip = find_command_in_prefix(&prefix, "pip", test_path_ext().as_deref()).expect("pip");

        assert_eq!(python, python_path);
        assert_eq!(pip, pip_path);
        assert_eq!(prefix_bin_dirs(&prefix).len(), 3);
    }

    #[test]
    fn shim_inventory_normalizes_extensions() {
        let temp = TempDir::new().expect("tempdir");
        let prefix = temp.path().join("runtime");
        if cfg!(windows) {
            fs::create_dir_all(prefix.join("Scripts")).expect("scripts");
            fs::write(prefix.join("python.exe"), "").expect("python");
            fs::write(prefix.join("Scripts").join("pip3.13.cmd"), "").expect("pip");
            fs::write(prefix.join("Scripts").join("activate.bat"), "").expect("activate");
            fs::write(prefix.join("Scripts").join("pythonw.dll"), "").expect("dll");

            let names = collect_shim_names_from_prefix(&prefix, test_path_ext().as_deref())
                .expect("inventory");
            assert_eq!(names, vec!["activate", "pip3.13", "python"]);
        } else {
            fs::create_dir_all(prefix.join("bin")).expect("bin");
            let python = prefix.join("bin").join("python");
            let pip = prefix.join("bin").join("pip3.13");
            let activate = prefix.join("bin").join("activate");
            fs::write(&python, "").expect("python");
            fs::write(&pip, "").expect("pip");
            fs::write(&activate, "").expect("activate");
            fs::write(prefix.join("bin").join("pythonw.dll"), "").expect("dll");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                for path in [&python, &pip, &activate] {
                    let metadata = fs::metadata(path).expect("metadata");
                    let mut permissions = metadata.permissions();
                    permissions.set_mode(0o755);
                    fs::set_permissions(path, permissions).expect("chmod");
                }
            }

            let names = collect_shim_names_from_prefix(&prefix, None).expect("inventory");
            assert_eq!(names, vec!["activate", "pip3.13", "python"]);
        }
    }

    #[test]
    #[cfg(windows)]
    fn search_path_entries_skips_windows_apps_trap() {
        let temp = TempDir::new().expect("tempdir");
        let trap_dir = temp
            .path()
            .join("Local")
            .join("Microsoft")
            .join("WindowsApps")
            .join("PythonSoftwareFoundation.Python.3.12_qbz5n2kfra8p0");
        let valid_dir = temp.path().join("Python312");

        fs::create_dir_all(&trap_dir).expect("trap dir");
        fs::create_dir_all(&valid_dir).expect("valid dir");

        let python_exe = "python.exe";
        fs::write(trap_dir.join(python_exe), "").expect("trap python");
        fs::write(valid_dir.join(python_exe), "").expect("valid python");

        let path_ext = Some(std::ffi::OsStr::new(".exe"));

        let directories = vec![trap_dir.clone(), valid_dir.clone()];
        let found = search_path_entries(&directories, "python", path_ext);
        assert_eq!(found, Some(valid_dir.join(python_exe)));

        let found_trap_only = search_path_entries(&[trap_dir], "python", path_ext);
        assert_eq!(found_trap_only, None);
    }
}
