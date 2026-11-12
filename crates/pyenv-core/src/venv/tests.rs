// ./crates/pyenv-core/src/venv/tests.rs
//! Regression tests for managed venv creation, lookup, and selection.

use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;

use crate::config::AppConfig;
use crate::context::AppContext;

use super::commands::{abort_upgrade, lower_bound_requirement};
use super::{
    VenvUseScope, cmd_venv_create, cmd_venv_info, cmd_venv_list, cmd_venv_use,
    resolve_installed_runtime_version,
};

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

#[test]
fn venv_create_rejects_parent_directory_name() {
    let (_temp, ctx) = test_context();
    create_fake_runtime(&ctx, "3.12.6");

    let report = cmd_venv_create(&ctx, "3.12.6", "..", false, false);
    assert_eq!(report.exit_code, 1);
    assert!(report.stderr[0].contains("invalid managed venv name"));
}

#[test]
fn venv_upgrade_fails_on_missing_source() {
    let (_temp, ctx) = test_context();
    create_fake_runtime(&ctx, "3.13.1");

    let report = super::cmd_venv_upgrade(&ctx, "demo", "3.13.1", true, false);
    assert_eq!(report.exit_code, 1);
    assert!(report.stderr[0].contains("pyenv: no managed venv named `demo` was found"));
}

#[test]
fn venv_upgrade_fails_on_missing_target() {
    let (_temp, ctx) = test_context();
    create_fake_runtime(&ctx, "3.12.6");
    create_fake_managed_env(&ctx, "3.12.6", "demo");

    let report = super::cmd_venv_upgrade(&ctx, "demo", "3.13.1", true, false);
    assert_eq!(report.exit_code, 1);
    assert!(report.stderr[0].contains("pyenv: target new runtime `3.13.1` is not installed"));
}

#[test]
fn venv_upgrade_resolves_a_minor_target_to_the_latest_installed_runtime() {
    let (_temp, ctx) = test_context();
    create_fake_runtime(&ctx, "3.14.8");
    create_fake_runtime(&ctx, "3.15.0");
    create_fake_runtime(&ctx, "3.15.1");
    create_fake_managed_env(&ctx, "3.14.8", "api");

    let resolved =
        resolve_installed_runtime_version(&ctx, "3.15").expect("resolve installed 3.15 target");
    assert_eq!(resolved, "3.15.1");
}

#[test]
fn venv_upgrade_uses_exact_then_compatible_lower_bound_specs() {
    assert_eq!(lower_bound_requirement("tomli==2.0.1"), "tomli>=2.0.1");
    assert_eq!(
        lower_bound_requirement("example @ https://example.invalid/example.whl"),
        "example @ https://example.invalid/example.whl"
    );
    assert_eq!(
        lower_bound_requirement("-e git+https://example.invalid/example.git#egg=example"),
        "-e git+https://example.invalid/example.git#egg=example"
    );
}

#[test]
fn resolve_managed_venv_rejects_parent_escape_specs() {
    let (_temp, ctx) = test_context();
    create_fake_runtime(&ctx, "3.12.6");
    create_fake_managed_env(&ctx, "3.12.6", "demo");
    fs::create_dir_all(ctx.root.join("venvs").join("3.12.6")).expect("base");

    let report = super::cmd_venv_delete(&ctx, "3.12.6/envs/..", true);
    assert_eq!(report.exit_code, 1);
    assert!(
        report
            .stderr
            .iter()
            .any(|line| line.contains("not a safe path") || line.contains("no managed venv"))
    );
    assert!(
        ctx.root.join("venvs").join("3.12.6").join("demo").is_dir(),
        "delete of 3.12.6/envs/.. must not wipe the venv registry"
    );
}

#[test]
fn venv_upgrade_fails_closed_when_inventory_cannot_run() {
    let (_temp, ctx) = test_context();
    create_fake_runtime(&ctx, "3.12.6");
    create_fake_runtime(&ctx, "3.13.1");
    create_fake_managed_env(&ctx, "3.12.6", "demo");

    let report = super::cmd_venv_upgrade(&ctx, "demo", "3.13.1", true, false);
    assert_eq!(report.exit_code, 1);
    let joined = report.stderr.join("\n");
    assert!(
        joined.contains("cannot inventory")
            || joined.contains("pip list")
            || joined.contains("failed to run")
            || joined.contains("interpreter"),
        "unexpected upgrade error: {joined}"
    );
    assert!(
        ctx.root.join("venvs").join("3.12.6").join("demo").is_dir(),
        "failed upgrade must leave the source venv in place"
    );
    assert!(
        !ctx.root.join("venvs").join("3.13.1").join("demo").exists(),
        "failed upgrade must not create a destination venv"
    );
}

#[test]
fn failed_upgrade_discards_partial_target_and_restores_replaced_target() {
    let (_temp, ctx) = test_context();
    create_fake_managed_env(&ctx, "3.13.1", "demo");
    let base = ctx.root.join("venvs").join("3.13.1");
    let destination = base.join("demo");
    let backup = base.join("demo-previous");
    fs::write(destination.join("marker"), "previous").expect("marker");
    fs::rename(&destination, &backup).expect("move aside");

    // A partially built replacement now occupies the destination.
    fs::create_dir_all(&destination).expect("partial target");
    fs::write(destination.join("partial"), "").expect("partial file");

    let report = abort_upgrade(
        vec!["Progress:".to_string()],
        &destination,
        Some(&backup),
        Some("pyenv: boom".to_string()),
    );

    assert_eq!(report.exit_code, 1);
    assert!(report.stderr.iter().any(|line| line.contains("boom")));
    assert!(
        report
            .stderr
            .iter()
            .any(|line| line.contains("source venv was left unchanged"))
    );
    assert!(!destination.join("partial").exists());
    assert_eq!(
        fs::read_to_string(destination.join("marker")).expect("restored marker"),
        "previous"
    );
    assert!(!backup.exists());
}
