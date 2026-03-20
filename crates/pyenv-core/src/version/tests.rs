// ./crates/pyenv-core/src/version/tests.rs
//! Regression coverage for version-file discovery, selection fallback, command output, and
//! hook-based overrides in the version subsystem.

use std::fs;

use tempfile::TempDir;

use crate::config::AppConfig;
use crate::context::AppContext;

use super::commands::{
    cmd_global, cmd_local, cmd_version, cmd_version_file_read, cmd_version_file_write,
    cmd_version_name, cmd_version_origin,
};
use super::files::{find_local_version_file, read_version_file, version_file_path};
use super::selection::installed_version_dir;
use super::types::LOCAL_VERSION_FILE;

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
fn finds_local_version_file_in_parent_chain() {
    let (_temp, ctx) = test_context();
    let project = ctx.dir.join("project").join("nested");
    fs::create_dir_all(&project).expect("project");
    let local_file = ctx.dir.join("project").join(LOCAL_VERSION_FILE);
    fs::write(&local_file, "3.12.1\n").expect("version file");

    assert_eq!(find_local_version_file(&project), Some(local_file));
}

#[test]
fn version_name_prefers_environment() {
    let (_temp, mut ctx) = test_context();
    fs::create_dir_all(installed_version_dir(&ctx, "3.12.1")).expect("installed version");
    ctx.env_version = Some("3.12.1".to_string());

    let report = cmd_version_name(&ctx, false);
    assert_eq!(report.exit_code, 0);
    assert_eq!(report.stdout, vec!["3.12.1"]);
}

#[test]
fn global_command_writes_version_file() {
    let (_temp, ctx) = test_context();
    fs::create_dir_all(installed_version_dir(&ctx, "3.11.9")).expect("installed version");

    let report = cmd_global(&ctx, &[String::from("3.11.9")], false);
    assert_eq!(report.exit_code, 0);
    assert_eq!(
        fs::read_to_string(
            version_file_path(&ctx, None)
                .parent()
                .expect("parent")
                .join("version")
        )
        .expect("global file"),
        "3.11.9\n"
    );
}

#[test]
fn local_command_can_force_uninstalled_version() {
    let (_temp, ctx) = test_context();

    let report = cmd_local(&ctx, &[String::from("3.99.0")], false, true);
    assert_eq!(report.exit_code, 0);
    assert_eq!(
        fs::read_to_string(ctx.dir.join(LOCAL_VERSION_FILE)).expect("local file"),
        "3.99.0\n"
    );
}

#[test]
fn version_defaults_to_system_when_unconfigured() {
    let (_temp, ctx) = test_context();
    let report = cmd_version_name(&ctx, false);
    assert_eq!(report.exit_code, 0);
    assert_eq!(report.stdout, vec!["system"]);
}

#[test]
fn version_name_falls_back_to_latest_prefix() {
    let (_temp, mut ctx) = test_context();
    fs::create_dir_all(installed_version_dir(&ctx, "3.12.6")).expect("installed version");
    ctx.env_version = Some("python-3.12".to_string());

    let report = cmd_version_name(&ctx, false);
    assert_eq!(report.exit_code, 0);
    assert_eq!(report.stdout, vec!["3.12.6"]);
}

#[test]
fn version_bare_emits_each_selected_version_on_its_own_line() {
    let (_temp, mut ctx) = test_context();
    for version in ["3.12.6", "3.11.9"] {
        fs::create_dir_all(installed_version_dir(&ctx, version)).expect("installed version");
    }
    ctx.env_version = Some("3.12:3.11".to_string());

    let report = cmd_version(&ctx, true);
    assert_eq!(report.exit_code, 0);
    assert_eq!(report.stdout, vec!["3.12.6", "3.11.9"]);
}

#[test]
fn version_file_read_joins_versions_and_reports_safe_warnings() {
    let (_temp, ctx) = test_context();
    let path = ctx.dir.join("my-version");
    fs::write(&path, "3.9.3\n../*\n3.8.9\n# ignored\n").expect("version file");

    let report = cmd_version_file_read(&path);
    assert_eq!(report.exit_code, 0);
    assert_eq!(report.stdout, vec!["3.9.3:3.8.9"]);
    assert_eq!(report.stderr.len(), 1);
    assert!(report.stderr[0].contains("invalid version"));
    assert!(report.stderr[0].contains("../*"));
}

#[test]
fn version_file_read_allows_internal_parent_components_within_version_tree() {
    let (_temp, ctx) = test_context();
    let path = ctx.dir.join("my-version");
    fs::write(&path, "3.10.3/envs/../test\n").expect("version file");

    let versions = read_version_file(&path).expect("versions");
    assert_eq!(versions, vec!["3.10.3/envs/../test"]);
}

#[test]
fn version_name_reports_missing_origin_for_environment_version() {
    let (_temp, mut ctx) = test_context();
    ctx.env_version = Some("1.2".to_string());

    let report = cmd_version_name(&ctx, false);
    assert_eq!(report.exit_code, 1);
    assert_eq!(report.stdout, vec![""]);
    assert!(report.stderr[0].contains("set by PYENV_VERSION environment variable"));
}

#[test]
fn version_file_write_persists_versions() {
    let (_temp, ctx) = test_context();
    fs::create_dir_all(installed_version_dir(&ctx, "3.12.6")).expect("installed version");
    let output = ctx.dir.join("custom-version");

    let report = cmd_version_file_write(&ctx, &output, &[String::from("3.12.6")], false);
    assert_eq!(report.exit_code, 0);
    assert_eq!(
        fs::read_to_string(output).expect("version file"),
        "3.12.6\n"
    );
}

#[test]
fn version_name_hook_can_override_selected_value() {
    let (_temp, mut ctx) = test_context();
    let hook_dir = ctx.root.join("pyenv.d").join("version-name");
    fs::create_dir_all(installed_version_dir(&ctx, "3.12.6")).expect("installed version");
    fs::create_dir_all(&hook_dir).expect("hook dir");
    if cfg!(windows) {
        fs::write(
            hook_dir.join("override.cmd"),
            "@echo ENV:PYENV_VERSION=3.12.6",
        )
        .expect("hook");
    } else {
        fs::write(
            hook_dir.join("override.sh"),
            "#!/usr/bin/env sh\necho ENV:PYENV_VERSION=3.12.6\n",
        )
        .expect("hook");
    }
    ctx.env_version = Some("3.12".to_string());

    let report = cmd_version_name(&ctx, false);
    assert_eq!(report.exit_code, 0);
    assert_eq!(report.stdout, vec!["3.12.6"]);
}

#[test]
fn version_origin_hook_can_override_origin_text() {
    let (_temp, ctx) = test_context();
    let hook_dir = ctx.root.join("pyenv.d").join("version-origin");
    fs::create_dir_all(&hook_dir).expect("hook dir");
    if cfg!(windows) {
        fs::write(
            hook_dir.join("override.cmd"),
            "@echo ENV:PYENV_VERSION_ORIGIN=hooked-origin",
        )
        .expect("hook");
    } else {
        fs::write(
            hook_dir.join("override.sh"),
            "#!/usr/bin/env sh\necho ENV:PYENV_VERSION_ORIGIN=hooked-origin\n",
        )
        .expect("hook");
    }

    let report = cmd_version_origin(&ctx);
    assert_eq!(report.exit_code, 0);
    assert_eq!(report.stdout, vec!["hooked-origin"]);
}
