// ./crates/pyenv-core/src/install/tests/support.rs
//! Shared install-test fixtures and synthetic provider metadata helpers.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;

use crate::config::AppConfig;
use crate::config::RuntimeArch;
use crate::context::AppContext;

use super::super::fetch::{
    nuget_index_cache_path, pypy_index_cache_path, write_nuget_index_cache, write_pypy_index_cache,
};
use super::super::types::{PypyReleaseFile, PypyReleaseManifest};

pub(super) fn test_context() -> (TempDir, AppContext) {
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

pub(super) fn test_path_ext() -> Option<OsString> {
    if cfg!(windows) {
        Some(OsString::from(".exe;.cmd;.bat"))
    } else {
        None
    }
}

pub(super) fn seed_package_index(ctx: &AppContext, package_name: &str, versions: &[&str]) {
    let cache_path = nuget_index_cache_path(ctx, package_name);
    let owned = versions
        .iter()
        .map(|version| version.to_string())
        .collect::<Vec<_>>();
    write_nuget_index_cache(&cache_path, &owned).expect("write cache");
}

pub(super) fn seed_pypy_index(ctx: &AppContext, releases: &[PypyReleaseManifest]) {
    let cache_path = pypy_index_cache_path(ctx);
    write_pypy_index_cache(&cache_path, releases).expect("write pypy cache");
}

pub(super) fn write_fake_python_build(temp: &TempDir, definitions: &[&str]) -> PathBuf {
    let script_path = if cfg!(windows) {
        temp.path().join("python-build.cmd")
    } else {
        temp.path().join("python-build")
    };
    let mut contents = String::new();
    if cfg!(windows) {
        contents.push_str("@echo off\r\n");
        contents.push_str("if \"%~1\"==\"--definitions\" (\r\n");
        for definition in definitions {
            contents.push_str(&format!("  echo {definition}\r\n"));
        }
        contents.push_str("  exit /b 0\r\n)\r\n");
        contents.push_str("exit /b 1\r\n");
    } else {
        contents.push_str("#!/usr/bin/env sh\n");
        contents.push_str("if [ \"$1\" = \"--definitions\" ]; then\n");
        for definition in definitions {
            contents.push_str(&format!("  echo {definition}\n"));
        }
        contents.push_str("  exit 0\nfi\n");
        contents.push_str("exit 1\n");
    }
    fs::write(&script_path, contents).expect("write fake python-build");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&script_path).expect("metadata");
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).expect("chmod");
    }
    script_path
}

pub(super) fn pypy_release(
    pypy_version: &str,
    python_version: &str,
    platform: &str,
    arch: &str,
    filename: &str,
) -> PypyReleaseManifest {
    PypyReleaseManifest {
        pypy_version: pypy_version.to_string(),
        python_version: python_version.to_string(),
        stable: true,
        latest_pypy: true,
        files: vec![PypyReleaseFile {
            filename: filename.to_string(),
            arch: arch.to_string(),
            platform: platform.to_string(),
            download_url: format!("https://downloads.python.org/pypy/{filename}"),
        }],
    }
}

pub(super) fn arm64_ctx(ctx: AppContext) -> AppContext {
    let mut ctx = ctx;
    ctx.config.install.arch = RuntimeArch::Arm64;
    ctx
}

pub(super) fn path_env_for(directory: PathBuf) -> std::ffi::OsString {
    env::join_paths([directory]).expect("path env")
}
