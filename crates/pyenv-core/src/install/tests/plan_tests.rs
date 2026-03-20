// ./crates/pyenv-core/src/install/tests/plan_tests.rs
//! Install-plan resolution and command-behavior regression tests.

use std::fs;

use crate::config::RuntimeArch;

use super::super::plans::resolve_install_plan_for_platform;
use super::super::{InstallCommandOptions, cmd_install};
use super::support::{
    path_env_for, pypy_release, seed_package_index, seed_pypy_index, test_context,
    write_fake_python_build,
};

#[test]
fn resolve_install_plan_uses_nuget_package_for_cpython() {
    let (_temp, mut ctx) = test_context();
    ctx.config.install.arch = RuntimeArch::X64;
    seed_package_index(&ctx, "python", &["3.13.10", "3.13.11", "3.13.12"]);
    let plan = resolve_install_plan_for_platform(&ctx, "3.13", "windows").expect("plan");
    assert_eq!(plan.resolved_version, "3.13.12");
    assert_eq!(plan.package_name, "python");
    assert_eq!(plan.package_version, "3.13.12");
    assert!(
        plan.download_url
            .contains("/python/3.13.12/python.3.13.12.nupkg")
    );
    assert!(plan.bootstrap_pip);
}

#[test]
fn resolve_install_plan_uses_provider_versions_not_upstream_catalog_only() {
    let (_temp, mut ctx) = test_context();
    ctx.config.install.arch = RuntimeArch::X64;
    seed_package_index(&ctx, "python", &["3.12.8", "3.12.9", "3.12.10"]);
    let plan = resolve_install_plan_for_platform(&ctx, "3.12", "windows").expect("plan");
    assert_eq!(plan.resolved_version, "3.12.10");
}

#[test]
fn resolve_install_plan_uses_freethreaded_package_names() {
    let (_temp, mut ctx) = test_context();
    ctx.config.install.arch = RuntimeArch::Arm64;
    seed_package_index(&ctx, "pythonarm64-freethreaded", &["3.13.10", "3.13.12"]);
    let plan = resolve_install_plan_for_platform(&ctx, "3.13t", "windows").expect("plan");
    assert_eq!(plan.package_name, "pythonarm64-freethreaded");
    assert_eq!(plan.package_version, "3.13.12");
    assert_eq!(plan.resolved_version, "3.13.12t");
    assert!(plan.free_threaded);
}

#[test]
fn resolve_install_plan_supports_pypy_provider_versions() {
    let (_temp, mut ctx) = test_context();
    ctx.config.install.arch = RuntimeArch::X64;
    seed_pypy_index(
        &ctx,
        &[
            pypy_release(
                "7.3.19",
                "3.11.11",
                "win64",
                "x64",
                "pypy3.11-v7.3.19-win64.zip",
            ),
            pypy_release(
                "7.3.20",
                "3.11.13",
                "win64",
                "x64",
                "pypy3.11-v7.3.20-win64.zip",
            ),
        ],
    );

    let plan = resolve_install_plan_for_platform(&ctx, "pypy3.11", "windows").expect("plan");
    assert_eq!(plan.provider, "windows-pypy-downloads");
    assert_eq!(plan.family, "PyPy");
    assert_eq!(plan.resolved_version, "pypy3.11-7.3.20");
    assert_eq!(plan.runtime_version, "3.11.13");
    assert!(plan.download_url.ends_with("pypy3.11-v7.3.20-win64.zip"));
}

#[test]
fn resolve_install_plan_supports_linux_pypy_provider_versions() {
    let (_temp, mut ctx) = test_context();
    ctx.config.install.arch = RuntimeArch::X64;
    seed_pypy_index(
        &ctx,
        &[pypy_release(
            "7.3.21",
            "3.11.13",
            "linux",
            "x64",
            "pypy3.11-v7.3.21-linux64.tar.bz2",
        )],
    );

    let plan = resolve_install_plan_for_platform(&ctx, "pypy3.11", "linux").expect("plan");
    assert_eq!(plan.provider, "linux-pypy-downloads");
    assert_eq!(plan.resolved_version, "pypy3.11-7.3.21");
    assert!(
        plan.python_executable
            .display()
            .to_string()
            .replace('\\', "/")
            .ends_with("/bin/pypy3")
    );
    assert!(
        plan.download_url
            .ends_with("pypy3.11-v7.3.21-linux64.tar.bz2")
    );
}

#[test]
fn install_command_requires_versions_without_list_mode() {
    let (_temp, ctx) = test_context();
    let report = cmd_install(
        &ctx,
        &InstallCommandOptions {
            list: false,
            force: false,
            dry_run: false,
            json: false,
            known: false,
            family: None,
            versions: Vec::new(),
        },
    );

    assert_eq!(report.exit_code, 1);
    assert!(report.stderr[0].contains("requires at least one version"));
}

#[test]
fn resolve_install_plan_delegates_to_python_build_on_linux() {
    let (temp, mut ctx) = test_context();
    let script = write_fake_python_build(&temp, &["stackless-3.7.5", "pypy3.10-7.3.15"]);
    ctx.config.install.python_build_path = Some(script);

    let plan = resolve_install_plan_for_platform(&ctx, "stackless-3.7.5", "linux").expect("plan");

    assert_eq!(plan.provider, "linux-python-build");
    assert_eq!(plan.family, "Stackless");
    assert_eq!(plan.resolved_version, "stackless-3.7.5");
    assert!(plan.download_url.starts_with("python-build://"));
    assert!(
        plan.python_executable
            .display()
            .to_string()
            .replace('\\', "/")
            .ends_with("/bin/python")
    );
}

#[test]
fn resolve_install_plan_prefers_native_cpython_source_on_linux() {
    let (_temp, ctx) = test_context();
    let plan = resolve_install_plan_for_platform(&ctx, "3.12", "linux").expect("plan");

    assert_eq!(plan.provider, "linux-cpython-source");
    assert!(plan.family == "CPython");
    assert!(plan.download_url.contains("python.org/ftp/python/"));
    assert!(plan.download_url.ends_with(".tgz"));
    assert!(
        plan.python_executable
            .display()
            .to_string()
            .replace('\\', "/")
            .ends_with("/bin/python")
    );
}

#[test]
fn resolve_install_plan_prefers_native_cpython_source_on_android() {
    let (_temp, ctx) = test_context();
    let plan = resolve_install_plan_for_platform(&ctx, "3.12", "android").expect("plan");

    assert_eq!(plan.provider, "android-cpython-source");
    assert!(plan.family == "CPython");
    assert!(plan.download_url.contains("python.org/ftp/python/"));
    assert!(plan.download_url.ends_with(".tgz"));
    assert!(
        plan.python_executable
            .display()
            .to_string()
            .replace('\\', "/")
            .ends_with("/bin/python")
    );
}

#[test]
fn resolve_python_build_path_can_search_path_environment() {
    let (temp, mut ctx) = test_context();
    let tool_dir = temp.path().join("tools");
    fs::create_dir_all(&tool_dir).expect("tool dir");
    let script_path = write_fake_python_build(&temp, &["3.12.10"]);
    fs::copy(
        &script_path,
        tool_dir.join(script_path.file_name().expect("filename")),
    )
    .expect("copy script");
    let final_script = tool_dir.join(script_path.file_name().expect("filename"));
    ctx.path_env = Some(path_env_for(tool_dir));
    ctx.path_ext = super::support::test_path_ext();

    let resolved =
        super::super::providers::resolve_python_build_path(&ctx).expect("python-build path");
    assert_eq!(
        resolved.to_string_lossy().to_ascii_lowercase(),
        final_script.to_string_lossy().to_ascii_lowercase()
    );
}
