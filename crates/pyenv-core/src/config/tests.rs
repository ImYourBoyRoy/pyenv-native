// ./crates/pyenv-core/src/config/tests.rs
//! Regression tests for config persistence, path resolution, and key-based mutation helpers.

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::super::storage::save_config;
    use super::super::values::{get_config_value, set_config_value};
    use super::super::{
        AppConfig, RuntimeArch, config_path, load_config, resolve_cache_dir, resolve_versions_dir,
    };

    #[test]
    fn default_config_uses_root_versions_and_cache_dirs() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        let config = AppConfig::default();

        assert_eq!(resolve_versions_dir(&root, &config), root.join("versions"));
        assert_eq!(resolve_cache_dir(&root, &config), root.join("cache"));
    }

    #[test]
    fn config_round_trips_install_and_storage_settings() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        fs::create_dir_all(&root).expect("root");

        let mut config = AppConfig::default();
        set_config_value(&mut config, "storage.versions_dir", "python-builds").expect("set");
        set_config_value(&mut config, "storage.cache_dir", "cache-downloads").expect("set");
        set_config_value(&mut config, "install.arch", "arm64").expect("set");
        set_config_value(
            &mut config,
            "install.python_build_path",
            "../pyenv/plugins/python-build/bin/python-build",
        )
        .expect("set");
        set_config_value(&mut config, "install.bootstrap_pip", "false").expect("set");
        save_config(&root, &config).expect("save");
        let loaded = load_config(&root).expect("load");

        assert_eq!(
            get_config_value(&loaded, "storage.versions_dir").expect("get"),
            "python-builds"
        );
        assert_eq!(
            get_config_value(&loaded, "storage.cache_dir").expect("get"),
            "cache-downloads"
        );
        assert_eq!(
            get_config_value(&loaded, "install.arch").expect("get"),
            "arm64"
        );
        assert_eq!(
            get_config_value(&loaded, "install.python_build_path").expect("get"),
            "../pyenv/plugins/python-build/bin/python-build"
        );
        assert_eq!(
            get_config_value(&loaded, "install.bootstrap_pip").expect("get"),
            "false"
        );
        assert_eq!(loaded.install.arch, RuntimeArch::Arm64);
        assert_eq!(
            loaded.install.python_build_path,
            Some(std::path::PathBuf::from(
                "../pyenv/plugins/python-build/bin/python-build"
            ))
        );
        assert_eq!(config_path(&root), root.join("config.toml"));
    }
}
