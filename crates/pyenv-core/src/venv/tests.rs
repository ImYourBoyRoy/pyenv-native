// ./crates/pyenv-core/src/venv/tests.rs
//! Regression tests for managed venv creation, lookup, and selection.

use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;

use crate::config::AppConfig;
use crate::context::AppContext;

use super::{VenvUseScope, cmd_venv_create, cmd_venv_info, cmd_venv_list, cmd_venv_use};

fn python_file_name() -> &'static str {
    if cfg!(windows) {
        "python.exe"
    } else {
        "python"
    }
}

fn pip_file_name() -> &'static str {
    if cfg!(windows) { "pip.exe" } else { "pip" }
}

fn test_context() -> (TempDir, AppContext) {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path().join(".pyenv");
    let dir = temp.path().join("work");
    fs::create_dir_all(root.join("versions")).expect("versions");
    fs::create_dir_all(&dir).expect("work");

    let ctx = AppContext {
        root,
        dir,
        exe_path: PathBuf::from("pyenv"),
        env_version: None,
        env_shell: None,
        path_env: None,
        path_ext: None,
        config: AppConfig::default(),
    };

    (temp, ctx)
}

fn create_fake_runtime(ctx: &AppContext, version: &str) {
    let version_dir = ctx.versions_dir().join(version);
    if cfg!(windows) {
        fs::create_dir_all(version_dir.join("Scripts")).expect("scripts");
        fs::write(version_dir.join(python_file_name()), "").expect("python");
        fs::write(version_dir.join("Scripts").join(pip_file_name()), "").expect("pip");
    } else {
        fs::create_dir_all(version_dir.join("bin")).expect("bin");
        fs::write(version_dir.join("bin").join(python_file_name()), "").expect("python");
        fs::write(version_dir.join("bin").join(pip_file_name()), "").expect("pip");
    }
}

fn create_fake_managed_env(ctx: &AppContext, version: &str, name: &str) {
    let env_dir = ctx.root.join("venvs").join(version).join(name);
    if cfg!(windows) {
        fs::create_dir_all(env_dir.join("Scripts")).expect("scripts");
        fs::write(env_dir.join("Scripts").join("python.exe"), "").expect("python");
        fs::write(env_dir.join("Scripts").join("pip.exe"), "").expect("pip");
    } else {
        fs::create_dir_all(env_dir.join("bin")).expect("bin");
        fs::write(env_dir.join("bin").join("python"), "").expect("python");
        fs::write(env_dir.join("bin").join("pip"), "").expect("pip");
    }
}

#[test]
fn venv_list_reports_managed_env_specs() {
    let (_temp, ctx) = test_context();
    create_fake_runtime(&ctx, "3.12.6");
    create_fake_managed_env(&ctx, "3.12.6", "demo");

    let report = cmd_venv_list(&ctx, true, false);
    assert_eq!(report.exit_code, 0);
    assert_eq!(report.stdout, vec!["3.12.6/envs/demo".to_string()]);
}

#[test]
fn venv_info_accepts_short_name_when_unique() {
    let (_temp, ctx) = test_context();
    create_fake_runtime(&ctx, "3.12.6");
    create_fake_managed_env(&ctx, "3.12.6", "demo");

    let report = cmd_venv_info(&ctx, "demo", false);
    assert_eq!(report.exit_code, 0);
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line == "Spec: 3.12.6/envs/demo")
    );
}

#[test]
fn venv_use_local_writes_env_spec_to_python_version_file() {
    let (_temp, ctx) = test_context();
    create_fake_runtime(&ctx, "3.12.6");
    create_fake_managed_env(&ctx, "3.12.6", "demo");

    let report = cmd_venv_use(&ctx, "demo", VenvUseScope::Local);
    assert_eq!(report.exit_code, 0);
    let file = fs::read_to_string(ctx.dir.join(".python-version")).expect("version file");
    assert_eq!(file, "3.12.6/envs/demo\n");
}

#[test]
fn venv_info_reports_top_level_registry_location() {
    let (_temp, ctx) = test_context();
    create_fake_runtime(&ctx, "3.12.6");
    create_fake_managed_env(&ctx, "3.12.6", "demo");

    let report = cmd_venv_info(&ctx, "demo", false);
    assert_eq!(report.exit_code, 0);
    assert!(report.stdout.iter().any(|line| line
        == &format!(
            "Location: {}",
            ctx.root.join("venvs").join("3.12.6").join("demo").display()
        )));
}

#[test]
fn venv_create_rejects_duplicate_name_collisions() {
    let (_temp, ctx) = test_context();
    create_fake_runtime(&ctx, "3.12.6");
    create_fake_runtime(&ctx, "3.13.1");
    create_fake_managed_env(&ctx, "3.12.6", "demo");

    let report = cmd_venv_create(&ctx, "3.13", "demo", false, false);
    assert_eq!(report.exit_code, 1);
    assert!(report.stderr[0].contains("managed venv name `demo` already exists"));
}
