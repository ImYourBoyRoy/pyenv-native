// ./crates/pyenv-core/src/install/tests/catalog_tests.rs
//! Provider-catalog and source-provider regression tests for install listing behavior.

use crate::config::RuntimeArch;

use super::super::InstallCommandOptions;
use super::super::providers::{
    cmd_provider_install_list, cpython_source_provider_entries,
    provider_catalog_entries_for_platform,
};
use super::support::{
    arm64_ctx, pypy_release, seed_package_index, seed_pypy_index, test_context,
    write_fake_python_build,
};

#[test]
fn install_list_defaults_to_provider_backed_catalog() {
    let (_temp, mut ctx) = test_context();
    ctx.config.install.arch = RuntimeArch::X64;
    seed_package_index(&ctx, "python", &["3.12.9", "3.12.10"]);
    seed_package_index(&ctx, "python-freethreaded", &["3.13.1"]);
    seed_pypy_index(
        &ctx,
        &[pypy_release(
            "7.3.20",
            "3.11.13",
            "win64",
            "x64",
            "pypy3.11-v7.3.20-win64.zip",
        )],
    );

    let report = cmd_provider_install_list(
        &ctx,
        &InstallCommandOptions {
            list: true,
            force: false,
            dry_run: false,
            json: false,
            known: false,
            family: None,
            versions: vec![],
        },
        "windows",
    );

    assert_eq!(report.exit_code, 0);
    assert_eq!(report.stdout[0], "Available installable versions:");
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("windows-cpython-nuget"))
    );
    assert!(report.stdout.iter().any(|line| line.trim() == "3.12.10"));
    assert!(report.stdout.iter().any(|line| line.trim() == "3.13.1t"));
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.trim() == "pypy3.11-7.3.20")
    );
}

#[test]
fn provider_catalog_entries_prefer_native_pypy_on_macos() {
    let (_temp, ctx) = test_context();
    seed_pypy_index(
        &ctx,
        &[pypy_release(
            "7.3.21",
            "3.11.13",
            "darwin",
            "arm64",
            "pypy3.11-v7.3.21-macos_arm64.tar.bz2",
        )],
    );

    let arm_ctx = arm64_ctx(ctx);
    let entries =
        provider_catalog_entries_for_platform(&arm_ctx, Some("pypy"), Some("3.11"), "macos")
            .expect("entries");

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].provider, "macos-pypy-downloads");
    assert_eq!(entries[0].family_slug, "pypy");
    assert_eq!(entries[0].version, "pypy3.11-7.3.21");
}

#[test]
fn provider_catalog_entries_include_native_cpython_on_macos_without_python_build() {
    let (_temp, ctx) = test_context();

    let entries =
        provider_catalog_entries_for_platform(&ctx, Some("cpython"), Some("3.12"), "macos")
            .expect("entries");

    assert!(!entries.is_empty());
    assert!(
        entries
            .iter()
            .all(|entry| entry.provider == "macos-cpython-source")
    );
    assert!(entries.iter().all(|entry| entry.family_slug == "cpython"));
}

#[test]
fn provider_catalog_entries_include_native_cpython_on_android_without_python_build() {
    let (_temp, ctx) = test_context();

    let entries =
        provider_catalog_entries_for_platform(&ctx, Some("cpython"), Some("3.12"), "android")
            .expect("entries");

    assert!(!entries.is_empty());
    assert!(
        entries
            .iter()
            .all(|entry| entry.provider == "android-cpython-source")
    );
    assert!(entries.iter().all(|entry| entry.family_slug == "cpython"));
}

#[test]
fn provider_catalog_entries_still_include_python_build_non_pypy_on_macos() {
    let (temp, mut ctx) = test_context();
    let script = write_fake_python_build(&temp, &["stackless-3.7.5", "pypy3.10-7.3.15"]);
    ctx.config.install.python_build_path = Some(script);

    let entries =
        provider_catalog_entries_for_platform(&ctx, Some("stackless"), Some("stackless"), "macos")
            .expect("entries");

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].provider, "macos-python-build");
    assert_eq!(entries[0].family_slug, "stackless");
    assert_eq!(entries[0].version, "stackless-3.7.5");
}

#[test]
fn cpython_source_entries_include_free_threaded_variants() {
    let (_temp, ctx) = test_context();
    let entries = cpython_source_provider_entries(&ctx, "linux").expect("entries");
    assert!(entries.iter().any(|entry| entry.version.ends_with('t')));
    assert!(
        entries
            .iter()
            .all(|entry| entry.provider == "linux-cpython-source")
    );
}
