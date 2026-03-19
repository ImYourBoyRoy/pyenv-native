// ./crates/pyenv-core/src/install.rs
//! Install planning and Windows-native runtime installation backends.

use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::catalog::{
    InstallListOptions, VersionFamily, cmd_install_list, known_version_names,
    latest_version_from_names,
};
use crate::command::CommandReport;
use crate::config::RuntimeArch;
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::plugin::run_hook_scripts;
use crate::runtime::{BASE_VENV_DIR_NAME, search_path_entries};
use crate::shim::rehash_shims;
use crate::version::installed_version_dir;
use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use tar::Archive;
use zip::ZipArchive;

const DEFAULT_NUGET_BASE_URL: &str = "https://api.nuget.org/v3-flatcontainer";
const DEFAULT_CPYTHON_SOURCE_BASE_URL: &str = "https://www.python.org/ftp/python";
const INSTALL_RECEIPT_FILE: &str = ".pyenv-install.json";
const NUGET_INDEX_TTL_SECS: u64 = 60 * 60 * 24;
const PYPY_VERSIONS_URL: &str = "https://downloads.python.org/pypy/versions.json";
const PYPY_INDEX_TTL_SECS: u64 = 60 * 60 * 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallCommandOptions {
    pub list: bool,
    pub force: bool,
    pub dry_run: bool,
    pub json: bool,
    pub known: bool,
    pub family: Option<String>,
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallPlan {
    pub requested_version: String,
    pub resolved_version: String,
    pub family: String,
    pub provider: String,
    pub architecture: String,
    pub runtime_version: String,
    pub free_threaded: bool,
    pub package_name: String,
    pub package_version: String,
    pub download_url: String,
    pub cache_path: PathBuf,
    pub install_dir: PathBuf,
    pub python_executable: PathBuf,
    pub bootstrap_pip: bool,
    pub create_base_venv: bool,
    pub base_venv_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallOutcome {
    pub plan: InstallPlan,
    pub receipt_path: PathBuf,
    pub pip_bootstrapped: bool,
    pub base_venv_created: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct InstallReceipt {
    requested_version: String,
    resolved_version: String,
    provider: String,
    family: String,
    architecture: String,
    runtime_version: String,
    package_name: String,
    package_version: String,
    download_url: String,
    cache_path: PathBuf,
    python_executable: PathBuf,
    bootstrap_pip: bool,
    base_venv_path: Option<PathBuf>,
    installed_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct NugetPackageIndex {
    versions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PypyReleaseManifest {
    pypy_version: String,
    python_version: String,
    stable: bool,
    #[serde(default)]
    latest_pypy: bool,
    files: Vec<PypyReleaseFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PypyReleaseFile {
    filename: String,
    arch: String,
    platform: String,
    download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ProviderCatalogGroup {
    family: String,
    family_slug: String,
    provider: String,
    architecture: String,
    versions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProviderCatalogEntry {
    family: String,
    family_slug: String,
    provider: String,
    architecture: String,
    version: String,
}

fn current_platform() -> &'static str {
    env::consts::OS
}

fn is_windows_platform(platform: &str) -> bool {
    platform.eq_ignore_ascii_case("windows")
}

fn python_build_provider_name(platform: &str) -> String {
    format!("{platform}-python-build")
}

fn cpython_source_provider_name(platform: &str) -> Option<&'static str> {
    match platform {
        "linux" => Some("linux-cpython-source"),
        "macos" => Some("macos-cpython-source"),
        _ => None,
    }
}

fn pypy_provider_name(platform: &str) -> Option<&'static str> {
    match platform {
        "windows" => Some("windows-pypy-downloads"),
        "linux" => Some("linux-pypy-downloads"),
        "macos" => Some("macos-pypy-downloads"),
        _ => None,
    }
}

fn family_filter_matches_provider(
    filter: &str,
    family_slug: &str,
    family_label: &str,
    provider: Option<&str>,
) -> bool {
    filter == family_slug
        || filter == family_label.to_ascii_lowercase()
        || provider.is_some_and(|provider_name| provider_name.eq_ignore_ascii_case(filter))
}

fn pypy_manifest_platform(platform: &str) -> Option<&'static str> {
    match platform {
        "windows" => Some("win64"),
        "linux" => Some("linux"),
        "macos" => Some("darwin"),
        _ => None,
    }
}

fn pypy_manifest_arches(arch: RuntimeArch, platform: &str) -> &'static [&'static str] {
    match (platform, arch) {
        ("windows", RuntimeArch::X64 | RuntimeArch::Auto) => &["x64"],
        ("linux", RuntimeArch::X64 | RuntimeArch::Auto) => &["x64"],
        ("linux", RuntimeArch::X86) => &["i686", "x86"],
        ("linux", RuntimeArch::Arm64) => &["aarch64", "arm64"],
        ("macos", RuntimeArch::X64 | RuntimeArch::Auto) => &["x64"],
        ("macos", RuntimeArch::Arm64) => &["arm64", "aarch64"],
        _ => &[],
    }
}

fn pypy_python_executable_path(install_dir: &Path, platform: &str) -> PathBuf {
    if is_windows_platform(platform) {
        install_dir.join("python.exe")
    } else {
        install_dir.join("bin").join("pypy3")
    }
}

fn cpython_source_python_executable_path(install_dir: &Path) -> PathBuf {
    install_dir.join("bin").join("python")
}

pub fn cmd_install(ctx: &AppContext, options: &InstallCommandOptions) -> CommandReport {
    let platform = current_platform();
    if options.list {
        if !options.known {
            return cmd_provider_install_list(ctx, options, platform);
        }

        let list_options = InstallListOptions {
            family: options.family.clone(),
            json: options.json,
            pattern: options.versions.first().cloned(),
        };
        return cmd_install_list(ctx, &list_options);
    }

    if options.versions.is_empty() {
        return CommandReport::failure(vec![PyenvError::MissingInstallVersion.to_string()], 1);
    }

    let mut plans = Vec::new();
    let mut outcomes = Vec::new();
    let mut stderr = Vec::new();

    for requested in &options.versions {
        match resolve_install_plan_for_platform(ctx, requested, platform) {
            Ok(plan) => {
                if options.dry_run {
                    plans.push(plan);
                } else {
                    match install_runtime(ctx, &plan, options.force) {
                        Ok(outcome) => outcomes.push(outcome),
                        Err(error) => stderr.push(error.to_string()),
                    }
                }
            }
            Err(error) => stderr.push(error.to_string()),
        }
    }

    let stdout = if options.json {
        if options.dry_run {
            render_json_lines(&plans)
        } else {
            render_json_lines(&outcomes)
        }
    } else if options.dry_run {
        render_plan_lines(&plans)
    } else {
        render_outcome_lines(&outcomes)
    };

    let exit_code = if stderr.is_empty() { 0 } else { 1 };
    CommandReport {
        stdout,
        stderr,
        exit_code,
    }
}

pub fn resolve_install_plan(ctx: &AppContext, requested: &str) -> Result<InstallPlan, PyenvError> {
    resolve_install_plan_for_platform(ctx, requested, current_platform())
}

fn resolve_install_plan_for_platform(
    ctx: &AppContext,
    requested: &str,
    platform: &str,
) -> Result<InstallPlan, PyenvError> {
    let normalized_request = normalize_requested_version(requested);
    if is_pypy_request(&normalized_request) {
        match resolve_pypy_install_plan(ctx, requested, &normalized_request, platform) {
            Ok(plan) => return Ok(plan),
            Err(native_error) if !is_windows_platform(platform) => {
                if let Ok(plan) = resolve_python_build_install_plan(ctx, requested, platform) {
                    return Ok(plan);
                }
                return Err(native_error);
            }
            Err(error) => return Err(error),
        }
    }

    if !is_windows_platform(platform) {
        if normalized_request
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit())
        {
            match resolve_cpython_source_install_plan(ctx, requested, &normalized_request, platform)
            {
                Ok(plan) => return Ok(plan),
                Err(native_error) => {
                    if let Ok(plan) = resolve_python_build_install_plan(ctx, requested, platform) {
                        return Ok(plan);
                    }
                    return Err(native_error);
                }
            }
        }
        return resolve_python_build_install_plan(ctx, requested, platform);
    }

    ensure_supported_cpython_version(&normalized_request)?;

    let free_threaded = is_free_threaded(&normalized_request);
    let effective_arch = ctx.config.install.arch.effective();
    let package_name = nuget_package_name(effective_arch, free_threaded).to_string();
    let resolved_version =
        resolve_provider_version(ctx, &package_name, &normalized_request, free_threaded)?;
    let package_version = resolved_version.trim_end_matches('t').to_string();

    let base_url = ctx
        .config
        .install
        .source_base_url
        .as_deref()
        .unwrap_or(DEFAULT_NUGET_BASE_URL)
        .trim_end_matches('/')
        .to_string();
    let package_name_lower = package_name.to_ascii_lowercase();
    let package_version_lower = package_version.to_ascii_lowercase();
    let download_url = format!(
        "{base_url}/{package_name_lower}/{package_version_lower}/{package_name_lower}.{package_version_lower}.nupkg"
    );

    let install_dir = installed_version_dir(ctx, &resolved_version);
    let cache_path = ctx
        .cache_dir()
        .join("packages")
        .join(format!("{package_name}.{package_version}.nupkg"));
    let base_venv_path = ctx
        .config
        .venv
        .auto_create_base_venv
        .then(|| install_dir.join(BASE_VENV_DIR_NAME));

    Ok(InstallPlan {
        requested_version: requested.to_string(),
        resolved_version,
        family: "CPython".to_string(),
        provider: "windows-cpython-nuget".to_string(),
        architecture: effective_arch.as_str().to_string(),
        runtime_version: package_version.clone(),
        free_threaded,
        package_name,
        package_version,
        download_url,
        cache_path,
        python_executable: install_dir.join("python.exe"),
        install_dir,
        bootstrap_pip: ctx.config.install.bootstrap_pip,
        create_base_venv: ctx.config.venv.auto_create_base_venv,
        base_venv_path,
    })
}

fn resolve_cpython_source_install_plan(
    ctx: &AppContext,
    requested: &str,
    normalized_request: &str,
    platform: &str,
) -> Result<InstallPlan, PyenvError> {
    let provider = cpython_source_provider_name(platform)
        .ok_or_else(|| PyenvError::UnsupportedInstallTarget(requested.to_string()))?;
    ensure_supported_cpython_version(normalized_request)?;

    let provider_versions = cpython_source_provider_versions();
    let resolved_version = if provider_versions
        .iter()
        .any(|candidate| candidate == normalized_request)
    {
        normalized_request.to_string()
    } else {
        latest_version_from_names(normalized_request, &provider_versions)
            .ok_or_else(|| PyenvError::UnknownVersion(requested.to_string()))?
    };

    let effective_arch = ctx.config.install.arch.effective();
    let package_version = resolved_version.trim_end_matches('t').to_string();
    let package_name = format!("Python-{package_version}.tgz");
    let download_url = format!(
        "{}/{}/{}",
        DEFAULT_CPYTHON_SOURCE_BASE_URL.trim_end_matches('/'),
        package_version,
        package_name
    );
    let install_dir = installed_version_dir(ctx, &resolved_version);
    let base_venv_path = ctx
        .config
        .venv
        .auto_create_base_venv
        .then(|| install_dir.join(BASE_VENV_DIR_NAME));

    Ok(InstallPlan {
        requested_version: requested.to_string(),
        resolved_version,
        family: "CPython".to_string(),
        provider: provider.to_string(),
        architecture: effective_arch.as_str().to_string(),
        runtime_version: package_version.clone(),
        free_threaded: is_free_threaded(normalized_request),
        package_name: package_name.clone(),
        package_version: package_version.clone(),
        download_url,
        cache_path: ctx.cache_dir().join("packages").join(package_name),
        python_executable: cpython_source_python_executable_path(&install_dir),
        install_dir,
        bootstrap_pip: ctx.config.install.bootstrap_pip,
        create_base_venv: ctx.config.venv.auto_create_base_venv,
        base_venv_path,
    })
}

fn resolve_python_build_install_plan(
    ctx: &AppContext,
    requested: &str,
    platform: &str,
) -> Result<InstallPlan, PyenvError> {
    let normalized_request = normalize_requested_version(requested);
    let definitions = load_python_build_definitions(ctx)?;
    let resolved_version = if definitions
        .iter()
        .any(|candidate| candidate == &normalized_request)
    {
        normalized_request.clone()
    } else {
        latest_version_from_names(&normalized_request, &definitions)
            .ok_or_else(|| PyenvError::UnknownVersion(requested.to_string()))?
    };

    let family = VersionFamily::classify(&resolved_version);
    let effective_arch = ctx.config.install.arch.effective();
    let install_dir = installed_version_dir(ctx, &resolved_version);
    let base_venv_path = ctx
        .config
        .venv
        .auto_create_base_venv
        .then(|| install_dir.join(BASE_VENV_DIR_NAME));

    Ok(InstallPlan {
        requested_version: requested.to_string(),
        resolved_version: resolved_version.clone(),
        family: family.label(),
        provider: python_build_provider_name(platform),
        architecture: effective_arch.as_str().to_string(),
        runtime_version: resolved_version.trim_end_matches('t').to_string(),
        free_threaded: is_free_threaded(&resolved_version),
        package_name: "python-build".to_string(),
        package_version: resolved_version.clone(),
        download_url: format!("python-build://{resolved_version}"),
        cache_path: ctx
            .cache_dir()
            .join("python-build")
            .join(format!("{}.cache", sanitize_for_fs(&resolved_version))),
        python_executable: install_dir.join("bin").join("python"),
        install_dir,
        bootstrap_pip: ctx.config.install.bootstrap_pip,
        create_base_venv: ctx.config.venv.auto_create_base_venv,
        base_venv_path,
    })
}

fn resolve_pypy_install_plan(
    ctx: &AppContext,
    requested: &str,
    normalized_request: &str,
    platform: &str,
) -> Result<InstallPlan, PyenvError> {
    let effective_arch = ctx.config.install.arch.effective();
    if pypy_manifest_arches(effective_arch, platform).is_empty() {
        return Err(PyenvError::UnsupportedInstallTarget(requested.to_string()));
    }
    let provider = pypy_provider_name(platform)
        .ok_or_else(|| PyenvError::UnsupportedInstallTarget(requested.to_string()))?;

    let releases = load_or_fetch_pypy_releases(ctx)?;
    let provider_names = pypy_provider_names(&releases, effective_arch, platform);
    let resolved_version = if provider_names
        .iter()
        .any(|candidate| candidate == normalized_request)
    {
        normalized_request.to_string()
    } else {
        latest_version_from_names(normalized_request, &provider_names)
            .ok_or_else(|| PyenvError::UnknownVersion(requested.to_string()))?
    };

    let (release, archive) =
        find_pypy_release_by_provider_name(&releases, &resolved_version, effective_arch, platform)
            .ok_or_else(|| PyenvError::UnsupportedInstallTarget(requested.to_string()))?;

    let install_dir = installed_version_dir(ctx, &resolved_version);
    let cache_path = ctx.cache_dir().join("packages").join(&archive.filename);
    let base_venv_path = ctx
        .config
        .venv
        .auto_create_base_venv
        .then(|| install_dir.join(BASE_VENV_DIR_NAME));

    Ok(InstallPlan {
        requested_version: requested.to_string(),
        resolved_version,
        family: "PyPy".to_string(),
        provider: provider.to_string(),
        architecture: effective_arch.as_str().to_string(),
        runtime_version: release.python_version.clone(),
        free_threaded: false,
        package_name: archive.filename.clone(),
        package_version: release.pypy_version.clone(),
        download_url: archive.download_url.clone(),
        cache_path,
        python_executable: pypy_python_executable_path(&install_dir, platform),
        install_dir,
        bootstrap_pip: ctx.config.install.bootstrap_pip,
        create_base_venv: ctx.config.venv.auto_create_base_venv,
        base_venv_path,
    })
}

pub fn install_runtime_plan(
    ctx: &AppContext,
    plan: &InstallPlan,
    force: bool,
) -> Result<InstallOutcome, PyenvError> {
    install_runtime(ctx, plan, force)
}

fn install_runtime(
    ctx: &AppContext,
    plan: &InstallPlan,
    force: bool,
) -> Result<InstallOutcome, PyenvError> {
    if is_cpython_source_provider(&plan.provider) {
        return install_runtime_via_cpython_source(ctx, plan, force);
    }

    if is_python_build_provider(&plan.provider) {
        return install_runtime_via_python_build(ctx, plan, force);
    }

    if plan.install_dir.exists() {
        if force {
            fs::remove_dir_all(&plan.install_dir).map_err(io_error)?;
        } else {
            return Err(PyenvError::VersionAlreadyInstalled(
                plan.resolved_version.clone(),
            ));
        }
    }

    run_hook_scripts(
        ctx,
        "install",
        &[
            ("PYENV_VERSION_NAME", plan.resolved_version.clone()),
            ("PYENV_VERSION", plan.resolved_version.clone()),
            ("PYENV_PREFIX", plan.install_dir.display().to_string()),
            ("PYENV_HOOK_STAGE", "before".to_string()),
            ("PYENV_INSTALL_PROVIDER", plan.provider.clone()),
        ],
    )?;

    download_package(plan)?;

    let versions_dir = plan
        .install_dir
        .parent()
        .ok_or_else(|| PyenvError::Io("pyenv: invalid install directory".to_string()))?;
    fs::create_dir_all(versions_dir).map_err(io_error)?;

    let staging_dir = versions_dir.join(format!(
        ".installing-{}-{}",
        sanitize_for_fs(&plan.resolved_version),
        unique_suffix()
    ));

    let outcome = (|| {
        extract_archive(plan, &staging_dir)?;
        move_directory(&staging_dir, &plan.install_dir)?;
        validate_python(&plan.python_executable)?;

        let pip_bootstrapped = if plan.bootstrap_pip {
            let pip_available = ensure_pip_available(&plan.python_executable)?;
            if plan.provider.starts_with("windows-") {
                ensure_pip_wrappers(plan)?;
            }
            pip_available
        } else {
            false
        };

        let mut base_venv_created = false;
        if plan.create_base_venv
            && let Some(base_venv_path) = &plan.base_venv_path
        {
            let base_venv_arg = base_venv_path.display().to_string();
            run_python(
                &plan.python_executable,
                &["-m", "venv", base_venv_arg.as_str()],
            )?;
            base_venv_created = true;
        }

        let receipt_path = write_install_receipt(plan)?;
        rehash_shims(ctx)?;
        run_hook_scripts(
            ctx,
            "install",
            &[
                ("PYENV_VERSION_NAME", plan.resolved_version.clone()),
                ("PYENV_VERSION", plan.resolved_version.clone()),
                ("PYENV_PREFIX", plan.install_dir.display().to_string()),
                ("PYENV_HOOK_STAGE", "after".to_string()),
                ("PYENV_INSTALL_PROVIDER", plan.provider.clone()),
            ],
        )?;
        Ok(InstallOutcome {
            plan: plan.clone(),
            receipt_path,
            pip_bootstrapped,
            base_venv_created,
        })
    })();

    if outcome.is_err() {
        let _ = fs::remove_dir_all(&staging_dir);
        let _ = fs::remove_dir_all(&plan.install_dir);
    }

    outcome
}

fn is_python_build_provider(provider: &str) -> bool {
    provider.ends_with("-python-build")
}

fn is_cpython_source_provider(provider: &str) -> bool {
    provider.ends_with("-cpython-source")
}

fn install_runtime_via_cpython_source(
    ctx: &AppContext,
    plan: &InstallPlan,
    force: bool,
) -> Result<InstallOutcome, PyenvError> {
    if plan.install_dir.exists() {
        if force {
            fs::remove_dir_all(&plan.install_dir).map_err(io_error)?;
        } else {
            return Err(PyenvError::VersionAlreadyInstalled(
                plan.resolved_version.clone(),
            ));
        }
    }

    run_hook_scripts(
        ctx,
        "install",
        &[
            ("PYENV_VERSION_NAME", plan.resolved_version.clone()),
            ("PYENV_VERSION", plan.resolved_version.clone()),
            ("PYENV_PREFIX", plan.install_dir.display().to_string()),
            ("PYENV_HOOK_STAGE", "before".to_string()),
            ("PYENV_INSTALL_PROVIDER", plan.provider.clone()),
        ],
    )?;

    download_package(plan)?;

    let versions_dir = plan
        .install_dir
        .parent()
        .ok_or_else(|| PyenvError::Io("pyenv: invalid install directory".to_string()))?;
    fs::create_dir_all(versions_dir).map_err(io_error)?;

    let source_dir = versions_dir.join(format!(
        ".building-source-{}-{}",
        sanitize_for_fs(&plan.resolved_version),
        unique_suffix()
    ));
    let build_dir = versions_dir.join(format!(
        ".building-work-{}-{}",
        sanitize_for_fs(&plan.resolved_version),
        unique_suffix()
    ));

    let outcome = (|| {
        extract_archive(plan, &source_dir)?;
        fs::create_dir_all(&build_dir).map_err(io_error)?;
        build_cpython_source_install(plan, &source_dir, &build_dir)?;
        ensure_unix_runtime_aliases(&plan.install_dir, &plan.runtime_version)?;
        validate_python(&plan.python_executable)?;

        let pip_bootstrapped = if plan.bootstrap_pip {
            ensure_pip_available(&plan.python_executable)?
        } else {
            false
        };

        let mut base_venv_created = false;
        if plan.create_base_venv
            && let Some(base_venv_path) = &plan.base_venv_path
        {
            let base_venv_arg = base_venv_path.display().to_string();
            run_python(
                &plan.python_executable,
                &["-m", "venv", base_venv_arg.as_str()],
            )?;
            base_venv_created = true;
        }

        let receipt_path = write_install_receipt(plan)?;
        rehash_shims(ctx)?;
        run_hook_scripts(
            ctx,
            "install",
            &[
                ("PYENV_VERSION_NAME", plan.resolved_version.clone()),
                ("PYENV_VERSION", plan.resolved_version.clone()),
                ("PYENV_PREFIX", plan.install_dir.display().to_string()),
                ("PYENV_HOOK_STAGE", "after".to_string()),
                ("PYENV_INSTALL_PROVIDER", plan.provider.clone()),
            ],
        )?;

        Ok(InstallOutcome {
            plan: plan.clone(),
            receipt_path,
            pip_bootstrapped,
            base_venv_created,
        })
    })();

    let _ = fs::remove_dir_all(&source_dir);
    let _ = fs::remove_dir_all(&build_dir);

    if outcome.is_err() {
        let _ = fs::remove_dir_all(&plan.install_dir);
    }

    outcome
}

fn install_runtime_via_python_build(
    ctx: &AppContext,
    plan: &InstallPlan,
    force: bool,
) -> Result<InstallOutcome, PyenvError> {
    if plan.install_dir.exists() {
        if force {
            fs::remove_dir_all(&plan.install_dir).map_err(io_error)?;
        } else {
            return Err(PyenvError::VersionAlreadyInstalled(
                plan.resolved_version.clone(),
            ));
        }
    }

    run_hook_scripts(
        ctx,
        "install",
        &[
            ("PYENV_VERSION_NAME", plan.resolved_version.clone()),
            ("PYENV_VERSION", plan.resolved_version.clone()),
            ("PYENV_PREFIX", plan.install_dir.display().to_string()),
            ("PYENV_HOOK_STAGE", "before".to_string()),
            ("PYENV_INSTALL_PROVIDER", plan.provider.clone()),
        ],
    )?;

    let outcome = (|| {
        let python_build = resolve_python_build_path(ctx)?;
        if let Some(parent) = plan.install_dir.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }

        run_python_build_install(
            ctx,
            &python_build,
            &plan.resolved_version,
            &plan.install_dir,
        )?;
        validate_python(&plan.python_executable)?;

        let pip_bootstrapped = if plan.bootstrap_pip {
            ensure_pip_available(&plan.python_executable)?
        } else {
            false
        };

        let mut base_venv_created = false;
        if plan.create_base_venv
            && let Some(base_venv_path) = &plan.base_venv_path
        {
            let base_venv_arg = base_venv_path.display().to_string();
            run_python(
                &plan.python_executable,
                &["-m", "venv", base_venv_arg.as_str()],
            )?;
            base_venv_created = true;
        }

        let receipt_path = write_install_receipt(plan)?;
        rehash_shims(ctx)?;
        run_hook_scripts(
            ctx,
            "install",
            &[
                ("PYENV_VERSION_NAME", plan.resolved_version.clone()),
                ("PYENV_VERSION", plan.resolved_version.clone()),
                ("PYENV_PREFIX", plan.install_dir.display().to_string()),
                ("PYENV_HOOK_STAGE", "after".to_string()),
                ("PYENV_INSTALL_PROVIDER", plan.provider.clone()),
            ],
        )?;

        Ok(InstallOutcome {
            plan: plan.clone(),
            receipt_path,
            pip_bootstrapped,
            base_venv_created,
        })
    })();

    if outcome.is_err() {
        let _ = fs::remove_dir_all(&plan.install_dir);
    }

    outcome
}

fn run_python_build_install(
    ctx: &AppContext,
    python_build: &Path,
    version: &str,
    prefix: &Path,
) -> Result<(), PyenvError> {
    let cache_dir = ctx.cache_dir().join("python-build");
    fs::create_dir_all(&cache_dir).map_err(io_error)?;

    let output = Command::new(python_build)
        .arg(version)
        .arg(prefix)
        .current_dir(&ctx.dir)
        .env("PYENV_ROOT", &ctx.root)
        .env("PYTHON_BUILD_CACHE_PATH", cache_dir)
        .output()
        .map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to execute {}: {error}",
                python_build.display()
            ))
        })?;

    if output.status.success() {
        return Ok(());
    }

    Err(PyenvError::Io(format!(
        "pyenv: python-build failed for `{version}` with exit code {}{}",
        output.status.code().unwrap_or(1),
        format_command_output_suffix(&output.stdout, &output.stderr)
    )))
}

fn build_cpython_source_install(
    plan: &InstallPlan,
    source_dir: &Path,
    build_dir: &Path,
) -> Result<(), PyenvError> {
    let configure_script = source_dir.join("configure");
    if !configure_script.is_file() {
        return Err(PyenvError::Io(format!(
            "pyenv: extracted source tree is missing {}",
            configure_script.display()
        )));
    }

    let prefix_arg = format!("--prefix={}", plan.install_dir.display());
    let mut configure = Command::new("sh");
    configure
        .current_dir(build_dir)
        .arg(configure_script)
        .arg(prefix_arg)
        .arg("--with-ensurepip=install");
    if plan.free_threaded {
        configure.arg("--disable-gil");
    }
    run_checked_process(
        configure,
        format!("configure source build for `{}`", plan.resolved_version),
    )?;

    let jobs = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1);
    let mut make = Command::new("make");
    make.current_dir(build_dir).arg(format!("-j{jobs}"));
    run_checked_process(make, format!("build `{}`", plan.resolved_version))?;

    let mut install = Command::new("make");
    install.current_dir(build_dir).arg("install");
    run_checked_process(
        install,
        format!(
            "install `{}` into {}",
            plan.resolved_version,
            plan.install_dir.display()
        ),
    )
}

fn run_checked_process(mut command: Command, description: String) -> Result<(), PyenvError> {
    let output = command
        .output()
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to {description}: {error}")))?;

    if output.status.success() {
        return Ok(());
    }

    Err(PyenvError::Io(format!(
        "pyenv: failed to {description} with exit code {}{}",
        output.status.code().unwrap_or(1),
        format_command_output_suffix(&output.stdout, &output.stderr)
    )))
}

fn ensure_unix_runtime_aliases(prefix: &Path, runtime_version: &str) -> Result<(), PyenvError> {
    let bin_dir = prefix.join("bin");
    if !bin_dir.is_dir() {
        return Ok(());
    }

    let parts = runtime_version.split('.').collect::<Vec<_>>();
    let major = parts.first().copied().unwrap_or("3");
    let major_minor = parts.iter().take(2).copied().collect::<Vec<_>>().join(".");

    let python_candidates = [
        bin_dir.join("python"),
        bin_dir.join("python3"),
        bin_dir.join(format!("python{major}")),
        bin_dir.join(format!("python{major_minor}")),
    ];
    if let Some(source) = first_existing_file(&python_candidates) {
        ensure_path_alias(&source, &bin_dir.join("python3"))?;
        ensure_path_alias(&source, &bin_dir.join("python"))?;
    }

    let pip_candidates = [
        bin_dir.join("pip"),
        bin_dir.join("pip3"),
        bin_dir.join(format!("pip{major}")),
        bin_dir.join(format!("pip{major_minor}")),
    ];
    if let Some(source) = first_existing_file(&pip_candidates) {
        ensure_path_alias(&source, &bin_dir.join("pip3"))?;
        ensure_path_alias(&source, &bin_dir.join("pip"))?;
    }

    Ok(())
}

fn first_existing_file(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|path| path.is_file()).cloned()
}

fn ensure_path_alias(source: &Path, destination: &Path) -> Result<(), PyenvError> {
    if source == destination || destination.exists() {
        return Ok(());
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let link_target = if source.parent() == destination.parent() {
            PathBuf::from(
                source
                    .file_name()
                    .ok_or_else(|| PyenvError::Io("pyenv: invalid alias source".to_string()))?,
            )
        } else {
            source.to_path_buf()
        };

        match symlink(&link_target, destination) {
            Ok(_) => return Ok(()),
            Err(error) => {
                fs::copy(source, destination).map_err(|copy_error| {
                    PyenvError::Io(format!(
                        "pyenv: failed to create alias {} -> {}: {error}; copy fallback also failed: {copy_error}",
                        destination.display(),
                        source.display()
                    ))
                })?;
                return Ok(());
            }
        }
    }

    #[cfg(not(unix))]
    {
        fs::copy(source, destination).map_err(io_error)?;
        Ok(())
    }
}

fn ensure_pip_available(python_executable: &Path) -> Result<bool, PyenvError> {
    if run_python(python_executable, &["-m", "pip", "--version"]).is_ok() {
        return Ok(true);
    }

    run_python(python_executable, &["-m", "ensurepip", "--default-pip"])?;
    run_python(python_executable, &["-m", "pip", "--version"])?;
    Ok(true)
}

fn resolve_provider_version(
    ctx: &AppContext,
    package_name: &str,
    requested: &str,
    free_threaded: bool,
) -> Result<String, PyenvError> {
    let versions = available_package_versions(ctx, package_name, free_threaded)?;
    if versions.iter().any(|version| version == requested) {
        return Ok(requested.to_string());
    }

    latest_version_from_names(requested, &versions)
        .ok_or_else(|| PyenvError::UnknownVersion(requested.to_string()))
}

fn ensure_supported_cpython_version(version: &str) -> Result<(), PyenvError> {
    if is_supported_cpython_version(version) {
        Ok(())
    } else {
        Err(PyenvError::UnsupportedInstallTarget(version.to_string()))
    }
}

fn is_supported_cpython_version(version: &str) -> bool {
    let probe = version.trim_end_matches('t');
    !probe.is_empty() && probe.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

fn is_free_threaded(version: &str) -> bool {
    version.len() > 1
        && version.ends_with('t')
        && version
            .chars()
            .nth_back(1)
            .is_some_and(|ch| ch.is_ascii_digit())
}

fn is_pypy_request(version: &str) -> bool {
    version.to_ascii_lowercase().starts_with("pypy")
}

fn normalize_requested_version(version: &str) -> String {
    let trimmed = version.trim();
    let stripped = trimmed
        .strip_prefix("python-")
        .or_else(|| trimmed.strip_prefix("cpython-"))
        .unwrap_or(trimmed);

    if stripped.to_ascii_lowercase().starts_with("pypy") {
        stripped.replace("-v", "-")
    } else {
        stripped.to_string()
    }
}

fn nuget_package_name(arch: RuntimeArch, free_threaded: bool) -> &'static str {
    match (arch, free_threaded) {
        (RuntimeArch::X64 | RuntimeArch::Auto, false) => "python",
        (RuntimeArch::X64 | RuntimeArch::Auto, true) => "python-freethreaded",
        (RuntimeArch::X86, false) => "pythonx86",
        (RuntimeArch::X86, true) => "pythonx86-freethreaded",
        (RuntimeArch::Arm64, false) => "pythonarm64",
        (RuntimeArch::Arm64, true) => "pythonarm64-freethreaded",
    }
}

fn available_package_versions(
    ctx: &AppContext,
    package_name: &str,
    free_threaded: bool,
) -> Result<Vec<String>, PyenvError> {
    let mut versions = load_or_fetch_nuget_package_versions(ctx, package_name)?;
    if free_threaded {
        versions = versions
            .into_iter()
            .map(|version| format!("{version}t"))
            .collect();
    }
    Ok(versions)
}

fn cmd_provider_install_list(
    ctx: &AppContext,
    options: &InstallCommandOptions,
    platform: &str,
) -> CommandReport {
    let pattern = options.versions.first().cloned();
    let entries = match provider_catalog_entries_for_platform(
        ctx,
        options.family.as_deref(),
        pattern.as_deref(),
        platform,
    ) {
        Ok(entries) => entries,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };

    if entries.is_empty() {
        return CommandReport::failure(
            vec!["pyenv: no installable versions match the requested filters".to_string()],
            1,
        );
    }

    let groups = group_provider_entries(entries);
    if options.json {
        return render_json_report(&groups);
    }

    let mut stdout = vec!["Available installable versions:".to_string()];
    for group in groups {
        stdout.push(String::new());
        stdout.push(format!(
            "{} [{} / {}]",
            group.family, group.provider, group.architecture
        ));
        stdout.extend(
            group
                .versions
                .into_iter()
                .map(|version| format!("  {version}")),
        );
    }
    CommandReport::success(stdout)
}

fn provider_catalog_entries_for_platform(
    ctx: &AppContext,
    family_filter: Option<&str>,
    pattern_filter: Option<&str>,
    platform: &str,
) -> Result<Vec<ProviderCatalogEntry>, PyenvError> {
    let family_filter = family_filter
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase());
    let pattern_filter = pattern_filter
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase());
    let cpython_provider = if is_windows_platform(platform) {
        Some("windows-cpython-nuget")
    } else {
        cpython_source_provider_name(platform)
    };
    let include_cpython = family_filter.as_ref().is_none_or(|filter| {
        family_filter_matches_provider(filter, "cpython", "CPython", cpython_provider)
    });
    let include_pypy = family_filter.as_ref().is_none_or(|filter| {
        family_filter_matches_provider(filter, "pypy", "PyPy", pypy_provider_name(platform))
    });
    let python_build_provider = python_build_provider_name(platform);
    let include_python_build = family_filter.as_ref().is_none_or(|filter| {
        filter == &python_build_provider
            || (!family_filter_matches_provider(filter, "cpython", "CPython", cpython_provider)
                && !family_filter_matches_provider(
                    filter,
                    "pypy",
                    "PyPy",
                    pypy_provider_name(platform),
                ))
    });

    let entries = if is_windows_platform(platform) {
        let mut entries = Vec::new();
        if include_cpython {
            entries.extend(cpython_provider_entries(ctx)?);
        }
        if include_pypy {
            entries.extend(pypy_provider_entries(ctx, platform)?);
        }
        entries
    } else {
        let mut entries = Vec::new();
        if include_cpython {
            entries.extend(cpython_source_provider_entries(ctx, platform)?);
        }
        if include_pypy {
            entries.extend(pypy_provider_entries(ctx, platform)?);
        }
        if include_python_build {
            match python_build_provider_entries(ctx, platform) {
                Ok(mut python_build_entries) => {
                    python_build_entries
                        .retain(|entry| !matches!(entry.family_slug.as_str(), "cpython" | "pypy"));
                    entries.extend(python_build_entries);
                }
                Err(error) if entries.is_empty() => {
                    return Err(error);
                }
                Err(_) => {}
            }
        }
        entries
    };

    Ok(entries
        .into_iter()
        .filter(|entry| {
            family_filter.as_ref().is_none_or(|filter| {
                entry.family_slug == *filter
                    || entry.family.to_ascii_lowercase() == *filter
                    || entry.provider.to_ascii_lowercase() == *filter
            })
        })
        .filter(|entry| {
            pattern_filter.as_ref().is_none_or(|filter| {
                entry.version.to_ascii_lowercase().contains(filter)
                    || entry.family.to_ascii_lowercase().contains(filter)
                    || entry.provider.to_ascii_lowercase().contains(filter)
            })
        })
        .collect())
}

fn python_build_provider_entries(
    ctx: &AppContext,
    platform: &str,
) -> Result<Vec<ProviderCatalogEntry>, PyenvError> {
    let definitions = load_python_build_definitions(ctx)?;
    let mut entries = definitions
        .into_iter()
        .map(|version| {
            let family = VersionFamily::classify(&version);
            ProviderCatalogEntry {
                family: family.label(),
                family_slug: family.slug(),
                provider: python_build_provider_name(platform),
                architecture: ctx.config.install.arch.effective().as_str().to_string(),
                version,
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|lhs, rhs| crate::catalog::compare_version_names(&lhs.version, &rhs.version));
    entries.dedup_by(|lhs, rhs| lhs.version == rhs.version && lhs.provider == rhs.provider);
    Ok(entries)
}

fn cpython_provider_entries(ctx: &AppContext) -> Result<Vec<ProviderCatalogEntry>, PyenvError> {
    let arch = ctx.config.install.arch.effective();
    let mut versions = available_package_versions(ctx, nuget_package_name(arch, false), false)?
        .into_iter()
        .filter(|version| is_stable_runtime_version(version))
        .collect::<Vec<_>>();
    versions.extend(
        available_package_versions(ctx, nuget_package_name(arch, true), true)?
            .into_iter()
            .filter(|version| is_stable_runtime_version(version)),
    );
    versions.sort_by(|lhs, rhs| crate::catalog::compare_version_names(lhs, rhs).reverse());
    versions.dedup();

    Ok(versions
        .into_iter()
        .map(|version| ProviderCatalogEntry {
            family: "CPython".to_string(),
            family_slug: "cpython".to_string(),
            provider: "windows-cpython-nuget".to_string(),
            architecture: arch.as_str().to_string(),
            version,
        })
        .collect())
}

fn cpython_source_provider_versions() -> Vec<String> {
    let mut versions = known_version_names()
        .iter()
        .filter(|version| {
            matches!(VersionFamily::classify(version), VersionFamily::CPython)
                && is_stable_runtime_version(version)
        })
        .cloned()
        .collect::<Vec<_>>();
    versions.sort_by(|lhs, rhs| crate::catalog::compare_version_names(lhs, rhs).reverse());
    versions.dedup();
    versions
}

fn cpython_source_provider_entries(
    ctx: &AppContext,
    platform: &str,
) -> Result<Vec<ProviderCatalogEntry>, PyenvError> {
    let Some(provider) = cpython_source_provider_name(platform) else {
        return Ok(Vec::new());
    };
    let arch = ctx.config.install.arch.effective();
    Ok(cpython_source_provider_versions()
        .into_iter()
        .map(|version| ProviderCatalogEntry {
            family: "CPython".to_string(),
            family_slug: "cpython".to_string(),
            provider: provider.to_string(),
            architecture: arch.as_str().to_string(),
            version,
        })
        .collect())
}

fn pypy_provider_entries(
    ctx: &AppContext,
    platform: &str,
) -> Result<Vec<ProviderCatalogEntry>, PyenvError> {
    let arch = ctx.config.install.arch.effective();
    let provider = match pypy_provider_name(platform) {
        Some(provider) => provider,
        None => return Ok(Vec::new()),
    };
    let versions = pypy_provider_names(&load_or_fetch_pypy_releases(ctx)?, arch, platform);

    Ok(versions
        .into_iter()
        .map(|version| ProviderCatalogEntry {
            family: "PyPy".to_string(),
            family_slug: "pypy".to_string(),
            provider: provider.to_string(),
            architecture: arch.as_str().to_string(),
            version,
        })
        .collect())
}

fn group_provider_entries(entries: Vec<ProviderCatalogEntry>) -> Vec<ProviderCatalogGroup> {
    let mut groups = BTreeMap::<(String, String, String, String), Vec<String>>::new();

    for entry in entries {
        groups
            .entry((
                entry.family.clone(),
                entry.family_slug.clone(),
                entry.provider.clone(),
                entry.architecture.clone(),
            ))
            .or_default()
            .push(entry.version);
    }

    groups
        .into_iter()
        .map(
            |((family, family_slug, provider, architecture), mut versions)| {
                versions
                    .sort_by(|lhs, rhs| crate::catalog::compare_version_names(lhs, rhs).reverse());
                versions.dedup();
                ProviderCatalogGroup {
                    family,
                    family_slug,
                    provider,
                    architecture,
                    versions,
                }
            },
        )
        .collect()
}

fn load_python_build_definitions(ctx: &AppContext) -> Result<Vec<String>, PyenvError> {
    let python_build = resolve_python_build_path(ctx)?;
    let output = Command::new(&python_build)
        .arg("--definitions")
        .current_dir(&ctx.dir)
        .output()
        .map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to execute {} --definitions: {error}",
                python_build.display()
            ))
        })?;

    if !output.status.success() {
        return Err(PyenvError::Io(format!(
            "pyenv: python-build --definitions failed with exit code {}{}",
            output.status.code().unwrap_or(1),
            format_command_output_suffix(&output.stdout, &output.stderr)
        )));
    }

    let mut versions = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    versions.sort_by(|lhs, rhs| crate::catalog::compare_version_names(lhs, rhs).reverse());
    versions.dedup();
    Ok(versions)
}

pub(crate) fn resolve_python_build_path(ctx: &AppContext) -> Result<PathBuf, PyenvError> {
    if let Some(configured) = ctx.config.install.python_build_path.as_ref() {
        let path = if configured.is_absolute() {
            configured.clone()
        } else {
            ctx.root.join(configured)
        };
        if path.is_file() {
            return Ok(path);
        }
    }

    if let Some(path) = find_command_on_path(ctx, "python-build") {
        return Ok(path);
    }

    for candidate in repo_relative_python_build_candidates(ctx) {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(PyenvError::MissingPythonBuildBackend)
}

fn repo_relative_python_build_candidates(ctx: &AppContext) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let mut roots = Vec::new();
    if let Some(parent) = ctx.dir.parent() {
        roots.push(parent.to_path_buf());
    }
    if let Some(parent) = ctx.root.parent() {
        roots.push(parent.to_path_buf());
    }
    if let Some(parent) = ctx.exe_path.parent().and_then(|path| path.parent()) {
        roots.push(parent.to_path_buf());
    }

    for root in roots {
        candidates.push(
            root.join("pyenv")
                .join("plugins")
                .join("python-build")
                .join("bin")
                .join("python-build"),
        );
        candidates.push(
            root.join("vendor")
                .join("pyenv")
                .join("plugins")
                .join("python-build")
                .join("bin")
                .join("python-build"),
        );
        candidates.push(
            root.join("pyenv")
                .join("plugins")
                .join("python-build")
                .join("bin")
                .join("python-build.cmd"),
        );
        candidates.push(
            root.join("vendor")
                .join("pyenv")
                .join("plugins")
                .join("python-build")
                .join("bin")
                .join("python-build.cmd"),
        );
    }

    candidates
}

fn find_command_on_path(ctx: &AppContext, command: &str) -> Option<PathBuf> {
    let directories = ctx
        .path_env
        .as_ref()
        .map(env::split_paths)
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    search_path_entries(&directories, command, ctx.path_ext.as_deref())
}

fn format_command_output_suffix(stdout: &[u8], stderr: &[u8]) -> String {
    let mut details = Vec::new();

    let stdout_text = String::from_utf8_lossy(stdout).trim().to_string();
    if !stdout_text.is_empty() {
        details.push(format!(
            "; stdout: {}",
            summarize_command_text(&stdout_text)
        ));
    }

    let stderr_text = String::from_utf8_lossy(stderr).trim().to_string();
    if !stderr_text.is_empty() {
        details.push(format!(
            "; stderr: {}",
            summarize_command_text(&stderr_text)
        ));
    }

    details.concat()
}

fn summarize_command_text(text: &str) -> String {
    let compact = text.lines().map(str::trim).collect::<Vec<_>>().join(" ");
    if compact.len() <= 220 {
        compact
    } else {
        format!("{}...", &compact[..220])
    }
}

fn is_stable_runtime_version(version: &str) -> bool {
    let probe = version.trim_end_matches('t');
    !probe.is_empty() && probe.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
}

fn load_or_fetch_nuget_package_versions(
    ctx: &AppContext,
    package_name: &str,
) -> Result<Vec<String>, PyenvError> {
    let cache_path = nuget_index_cache_path(ctx, package_name);
    if cache_path.is_file() && cache_is_fresh_with_ttl(&cache_path, NUGET_INDEX_TTL_SECS) {
        return read_nuget_index_cache(&cache_path);
    }

    match fetch_nuget_package_versions(ctx, package_name) {
        Ok(versions) => {
            write_nuget_index_cache(&cache_path, &versions)?;
            Ok(versions)
        }
        Err(error) => {
            if cache_path.is_file() {
                read_nuget_index_cache(&cache_path).or(Err(error))
            } else {
                Err(error)
            }
        }
    }
}

fn fetch_nuget_package_versions(
    ctx: &AppContext,
    package_name: &str,
) -> Result<Vec<String>, PyenvError> {
    let base_url = ctx
        .config
        .install
        .source_base_url
        .as_deref()
        .unwrap_or(DEFAULT_NUGET_BASE_URL)
        .trim_end_matches('/')
        .to_string();
    let url = format!(
        "{base_url}/{}/index.json",
        package_name.to_ascii_lowercase()
    );
    let client = Client::builder()
        .user_agent(format!("pyenv-native/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to build HTTP client: {error}")))?;

    let response_body = client
        .get(&url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to query {url}: {error}")))?
        .text()
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to read {url}: {error}")))?;
    let index = serde_json::from_str::<NugetPackageIndex>(&response_body)
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to parse {url}: {error}")))?;

    Ok(index.versions)
}

fn nuget_index_cache_path(ctx: &AppContext, package_name: &str) -> PathBuf {
    ctx.cache_dir()
        .join("metadata")
        .join("nuget")
        .join(format!("{}.index.json", package_name.to_ascii_lowercase()))
}

fn cache_is_fresh_with_ttl(path: &Path, ttl_secs: u64) -> bool {
    path.metadata()
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .is_some_and(|age| age.as_secs() <= ttl_secs)
}

fn read_nuget_index_cache(path: &Path) -> Result<Vec<String>, PyenvError> {
    let contents = fs::read_to_string(path).map_err(io_error)?;
    let index = serde_json::from_str::<NugetPackageIndex>(&contents).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to parse cached package index {}: {error}",
            path.display()
        ))
    })?;
    Ok(index.versions)
}

fn write_nuget_index_cache(path: &Path, versions: &[String]) -> Result<(), PyenvError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    let payload = serde_json::to_string_pretty(&NugetPackageIndex {
        versions: versions.to_vec(),
    })
    .map_err(|error| {
        PyenvError::Io(format!("pyenv: failed to serialize package index: {error}"))
    })?;
    fs::write(path, payload).map_err(io_error)
}

fn pypy_provider_names(
    releases: &[PypyReleaseManifest],
    arch: RuntimeArch,
    platform: &str,
) -> Vec<String> {
    if pypy_manifest_arches(arch, platform).is_empty() {
        return Vec::new();
    }

    let mut versions = releases
        .iter()
        .filter(|release| release.stable)
        .filter_map(|release| {
            release
                .files
                .iter()
                .find(|file| pypy_file_matches_target(file, platform, arch))
                .map(|_| {
                    normalize_pypy_provider_name(&release.python_version, &release.pypy_version)
                })
        })
        .collect::<Vec<_>>();
    versions.sort_by(|lhs, rhs| crate::catalog::compare_version_names(lhs, rhs).reverse());
    versions.dedup();
    versions
}

fn find_pypy_release_by_provider_name<'a>(
    releases: &'a [PypyReleaseManifest],
    provider_name: &str,
    arch: RuntimeArch,
    platform: &str,
) -> Option<(&'a PypyReleaseManifest, &'a PypyReleaseFile)> {
    if pypy_manifest_arches(arch, platform).is_empty() {
        return None;
    }

    releases.iter().find_map(|release| {
        if !release.stable
            || normalize_pypy_provider_name(&release.python_version, &release.pypy_version)
                != provider_name
        {
            return None;
        }

        release
            .files
            .iter()
            .find(|file| pypy_file_matches_target(file, platform, arch))
            .map(|file| (release, file))
    })
}

fn pypy_file_matches_target(file: &PypyReleaseFile, platform: &str, arch: RuntimeArch) -> bool {
    let Some(expected_platform) = pypy_manifest_platform(platform) else {
        return false;
    };
    file.platform.eq_ignore_ascii_case(expected_platform)
        && pypy_manifest_arches(arch, platform)
            .iter()
            .any(|candidate| file.arch.eq_ignore_ascii_case(candidate))
}

fn normalize_pypy_provider_name(python_version: &str, pypy_version: &str) -> String {
    let major_minor = python_version
        .split('.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".");
    format!("pypy{major_minor}-{pypy_version}")
}

fn load_or_fetch_pypy_releases(ctx: &AppContext) -> Result<Vec<PypyReleaseManifest>, PyenvError> {
    let cache_path = pypy_index_cache_path(ctx);
    if cache_path.is_file() && cache_is_fresh_with_ttl(&cache_path, PYPY_INDEX_TTL_SECS) {
        return read_pypy_index_cache(&cache_path);
    }

    match fetch_pypy_releases() {
        Ok(releases) => {
            write_pypy_index_cache(&cache_path, &releases)?;
            Ok(releases)
        }
        Err(error) => {
            if cache_path.is_file() {
                read_pypy_index_cache(&cache_path).or(Err(error))
            } else {
                Err(error)
            }
        }
    }
}

fn fetch_pypy_releases() -> Result<Vec<PypyReleaseManifest>, PyenvError> {
    let client = Client::builder()
        .user_agent(format!("pyenv-native/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to build HTTP client: {error}")))?;

    let response = client.get(PYPY_VERSIONS_URL).send().map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to query {PYPY_VERSIONS_URL}: {error}"
        ))
    })?;

    let response = response.error_for_status().map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to query {PYPY_VERSIONS_URL}: {error}"
        ))
    })?;

    let response_body = response.text().map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to read {PYPY_VERSIONS_URL}: {error}"
        ))
    })?;

    serde_json::from_str::<Vec<PypyReleaseManifest>>(&response_body).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to parse {PYPY_VERSIONS_URL}: {error}"
        ))
    })
}

fn pypy_index_cache_path(ctx: &AppContext) -> PathBuf {
    ctx.cache_dir()
        .join("metadata")
        .join("pypy")
        .join("versions.json")
}

fn read_pypy_index_cache(path: &Path) -> Result<Vec<PypyReleaseManifest>, PyenvError> {
    let contents = fs::read_to_string(path).map_err(io_error)?;
    serde_json::from_str::<Vec<PypyReleaseManifest>>(&contents).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to parse cached PyPy index {}: {error}",
            path.display()
        ))
    })
}

fn write_pypy_index_cache(path: &Path, releases: &[PypyReleaseManifest]) -> Result<(), PyenvError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    let payload = serde_json::to_string_pretty(releases).map_err(|error| {
        PyenvError::Io(format!("pyenv: failed to serialize PyPy index: {error}"))
    })?;
    fs::write(path, payload).map_err(io_error)
}

fn download_package(plan: &InstallPlan) -> Result<(), PyenvError> {
    if plan.cache_path.is_file() {
        return Ok(());
    }

    let parent = plan
        .cache_path
        .parent()
        .ok_or_else(|| PyenvError::Io("pyenv: invalid cache path".to_string()))?;
    fs::create_dir_all(parent).map_err(io_error)?;

    let extension = plan
        .cache_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("download");
    let partial_path = parent.join(format!(
        ".partial-{}.{}",
        sanitize_for_fs(&plan.package_name),
        extension
    ));
    if partial_path.exists() {
        let _ = fs::remove_file(&partial_path);
    }

    let client = Client::builder()
        .user_agent(format!("pyenv-native/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to build HTTP client: {error}")))?;

    let response = client.get(&plan.download_url).send().map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to download {}: {error}",
            plan.download_url
        ))
    })?;

    let mut response = response.error_for_status().map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to download {}: {error}",
            plan.download_url
        ))
    })?;

    let mut file = fs::File::create(&partial_path).map_err(io_error)?;
    response.copy_to(&mut file).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to write {}: {error}",
            partial_path.display()
        ))
    })?;
    file.flush().map_err(io_error)?;
    fs::rename(&partial_path, &plan.cache_path).map_err(io_error)
}

fn extract_archive(plan: &InstallPlan, destination: &Path) -> Result<(), PyenvError> {
    if is_zip_extension(plan.cache_path.as_path()) && plan.provider != "windows-cpython-nuget" {
        extract_root_archive(&plan.cache_path, destination)
    } else if is_tar_archive_extension(plan.cache_path.as_path()) {
        extract_tar_root_archive(&plan.cache_path, destination)
    } else {
        extract_tools_archive(&plan.cache_path, destination)
    }
}

fn extract_tools_archive(archive_path: &Path, destination: &Path) -> Result<(), PyenvError> {
    prepare_clean_directory(destination)?;

    let file = fs::File::open(archive_path).map_err(io_error)?;
    let mut archive = ZipArchive::new(file).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to open {}: {error}",
            archive_path.display()
        ))
    })?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| {
            PyenvError::Io(format!("pyenv: failed to read archive entry: {error}"))
        })?;
        let Some(path) = entry.enclosed_name().map(|value| value.to_path_buf()) else {
            continue;
        };
        let Ok(relative) = path.strip_prefix("tools") else {
            continue;
        };
        if relative.as_os_str().is_empty() {
            continue;
        }

        let output_path = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&output_path).map_err(io_error)?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }

        let mut output = fs::File::create(&output_path).map_err(io_error)?;
        std::io::copy(&mut entry, &mut output).map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to extract {}: {error}",
                output_path.display()
            ))
        })?;
        output.flush().map_err(io_error)?;
    }

    Ok(())
}

fn extract_root_archive(archive_path: &Path, destination: &Path) -> Result<(), PyenvError> {
    prepare_clean_directory(destination)?;

    let file = fs::File::open(archive_path).map_err(io_error)?;
    let mut archive = ZipArchive::new(file).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to open {}: {error}",
            archive_path.display()
        ))
    })?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| {
            PyenvError::Io(format!("pyenv: failed to read archive entry: {error}"))
        })?;
        let Some(path) = entry.enclosed_name().map(|value| value.to_path_buf()) else {
            continue;
        };

        let mut components = path.components();
        let _ = components.next();
        let relative = components.collect::<PathBuf>();
        if relative.as_os_str().is_empty() {
            continue;
        }

        let output_path = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&output_path).map_err(io_error)?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }

        let mut output = fs::File::create(&output_path).map_err(io_error)?;
        std::io::copy(&mut entry, &mut output).map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to extract {}: {error}",
                output_path.display()
            ))
        })?;
        output.flush().map_err(io_error)?;
    }

    Ok(())
}

fn extract_tar_root_archive(archive_path: &Path, destination: &Path) -> Result<(), PyenvError> {
    prepare_clean_directory(destination)?;

    let file = fs::File::open(archive_path).map_err(io_error)?;
    let file_name = archive_path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();

    if file_name.ends_with(".tar.bz2")
        || file_name.ends_with(".tbz2")
        || file_name.ends_with(".tbz")
    {
        let decoder = BzDecoder::new(BufReader::new(file));
        let mut archive = Archive::new(decoder);
        archive.unpack(destination).map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to extract {}: {error}",
                archive_path.display()
            ))
        })?;
    } else if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
        let decoder = GzDecoder::new(BufReader::new(file));
        let mut archive = Archive::new(decoder);
        archive.unpack(destination).map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to extract {}: {error}",
                archive_path.display()
            ))
        })?;
    } else if file_name.ends_with(".tar") {
        let mut archive = Archive::new(BufReader::new(file));
        archive.unpack(destination).map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to extract {}: {error}",
                archive_path.display()
            ))
        })?;
    } else {
        return Err(PyenvError::Io(format!(
            "pyenv: unsupported archive format: {}",
            archive_path.display()
        )));
    }

    flatten_single_top_level_directory(destination)
}

fn prepare_clean_directory(destination: &Path) -> Result<(), PyenvError> {
    if destination.exists() {
        fs::remove_dir_all(destination).map_err(io_error)?;
    }
    fs::create_dir_all(destination).map_err(io_error)
}

fn flatten_single_top_level_directory(destination: &Path) -> Result<(), PyenvError> {
    let mut entries = fs::read_dir(destination)
        .map_err(io_error)?
        .filter_map(|entry| entry.ok())
        .collect::<Vec<_>>();
    if entries.len() != 1 {
        return Ok(());
    }

    let root = entries.pop().expect("single entry").path();
    if !root.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(&root).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        move_path(&source_path, &destination_path)?;
    }

    fs::remove_dir_all(&root).map_err(io_error)
}

fn move_path(source: &Path, destination: &Path) -> Result<(), PyenvError> {
    match fs::rename(source, destination) {
        Ok(_) => Ok(()),
        Err(_) => {
            if source.is_dir() {
                copy_dir_recursive(source, destination)?;
                fs::remove_dir_all(source).map_err(io_error)
            } else {
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent).map_err(io_error)?;
                }
                fs::copy(source, destination).map_err(io_error)?;
                fs::remove_file(source).map_err(io_error)
            }
        }
    }
}

fn is_zip_extension(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|value| {
            value.eq_ignore_ascii_case("zip") || value.eq_ignore_ascii_case("nupkg")
        })
}

fn is_tar_archive_extension(path: &Path) -> bool {
    let lower = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    lower.ends_with(".tar")
        || lower.ends_with(".tar.gz")
        || lower.ends_with(".tgz")
        || lower.ends_with(".tar.bz2")
        || lower.ends_with(".tbz")
        || lower.ends_with(".tbz2")
}

fn move_directory(source: &Path, destination: &Path) -> Result<(), PyenvError> {
    match fs::rename(source, destination) {
        Ok(_) => Ok(()),
        Err(_) => {
            copy_dir_recursive(source, destination)?;
            fs::remove_dir_all(source).map_err(io_error)
        }
    }
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<(), PyenvError> {
    fs::create_dir_all(destination).map_err(io_error)?;
    for entry in fs::read_dir(source).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type().map_err(io_error)?.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent).map_err(io_error)?;
            }
            fs::copy(&source_path, &destination_path).map_err(io_error)?;
        }
    }
    Ok(())
}

fn validate_python(python_executable: &Path) -> Result<(), PyenvError> {
    run_python(python_executable, &["-V"])
}

fn run_python(python_executable: &Path, args: &[&str]) -> Result<(), PyenvError> {
    let output = Command::new(python_executable)
        .args(args)
        .output()
        .map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to run {}: {error}",
                python_executable.display()
            ))
        })?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("exit status {}", output.status)
        };
        Err(PyenvError::Io(format!(
            "pyenv: command `{}` failed: {detail}",
            render_command(python_executable, args)
        )))
    }
}

fn render_command(executable: &Path, args: &[&str]) -> String {
    let mut parts = vec![executable.display().to_string()];
    parts.extend(args.iter().map(|arg| arg.to_string()));
    parts.join(" ")
}

fn write_install_receipt(plan: &InstallPlan) -> Result<PathBuf, PyenvError> {
    let receipt = InstallReceipt {
        requested_version: plan.requested_version.clone(),
        resolved_version: plan.resolved_version.clone(),
        provider: plan.provider.clone(),
        family: plan.family.clone(),
        architecture: plan.architecture.clone(),
        runtime_version: plan.runtime_version.clone(),
        package_name: plan.package_name.clone(),
        package_version: plan.package_version.clone(),
        download_url: plan.download_url.clone(),
        cache_path: plan.cache_path.clone(),
        python_executable: plan.python_executable.clone(),
        bootstrap_pip: plan.bootstrap_pip,
        base_venv_path: plan.base_venv_path.clone(),
        installed_at_epoch_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let receipt_path = plan.install_dir.join(INSTALL_RECEIPT_FILE);
    let contents = serde_json::to_string_pretty(&receipt)
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to serialize receipt: {error}")))?;
    fs::write(&receipt_path, contents).map_err(io_error)?;
    Ok(receipt_path)
}

fn ensure_pip_wrappers(plan: &InstallPlan) -> Result<(), PyenvError> {
    let scripts_dir = plan.install_dir.join("Scripts");
    fs::create_dir_all(&scripts_dir).map_err(io_error)?;

    let wrappers = pip_wrapper_names(&plan.runtime_version);
    let wrapper_body = "@echo off\r\n\"%~dp0..\\python.exe\" -m pip %*\r\n";

    for wrapper_name in wrappers {
        let wrapper_path = scripts_dir.join(format!("{wrapper_name}.cmd"));
        if !wrapper_path.exists() {
            fs::write(wrapper_path, wrapper_body).map_err(io_error)?;
        }
    }

    Ok(())
}

fn pip_wrapper_names(package_version: &str) -> Vec<String> {
    let mut names = vec!["pip".to_string()];
    let parts = package_version
        .split('.')
        .take(2)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if let Some(major) = parts.first() {
        names.push(format!("pip{major}"));
    }
    if parts.len() == 2 {
        names.push(format!("pip{}.{}", parts[0], parts[1]));
    }
    names
}

fn render_plan_lines(plans: &[InstallPlan]) -> Vec<String> {
    let mut lines = Vec::new();
    for (index, plan) in plans.iter().enumerate() {
        if index > 0 {
            lines.push(String::new());
        }
        lines.push(format!("Requested: {}", plan.requested_version));
        lines.push(format!("Resolved: {}", plan.resolved_version));
        lines.push(format!("Provider: {}", plan.provider));
        lines.push(format!("Runtime: {}", plan.runtime_version));
        lines.push(format!(
            "Package: {} {}",
            plan.package_name, plan.package_version
        ));
        lines.push(format!("Architecture: {}", plan.architecture));
        lines.push(format!("Download: {}", plan.download_url));
        lines.push(format!("Cache: {}", plan.cache_path.display()));
        lines.push(format!("Install dir: {}", plan.install_dir.display()));
        lines.push(format!("Bootstrap pip: {}", plan.bootstrap_pip));
        lines.push(format!("Create base venv: {}", plan.create_base_venv));
    }
    lines
}

fn render_outcome_lines(outcomes: &[InstallOutcome]) -> Vec<String> {
    let mut lines = Vec::new();
    for (index, outcome) in outcomes.iter().enumerate() {
        if index > 0 {
            lines.push(String::new());
        }
        lines.push(format!(
            "Installed {} -> {}",
            outcome.plan.requested_version, outcome.plan.resolved_version
        ));
        lines.push(format!("Location: {}", outcome.plan.install_dir.display()));
        lines.push(format!(
            "Python: {}",
            outcome.plan.python_executable.display()
        ));
        lines.push(format!("Runtime: {}", outcome.plan.runtime_version));
        lines.push(format!("Pip bootstrapped: {}", outcome.pip_bootstrapped));
        lines.push(format!("Base venv created: {}", outcome.base_venv_created));
        lines.push(format!("Receipt: {}", outcome.receipt_path.display()));
    }
    lines
}

fn render_json_lines<T: Serialize>(value: &T) -> Vec<String> {
    serde_json::to_string_pretty(value)
        .map(|json| json.lines().map(ToOwned::to_owned).collect())
        .unwrap_or_else(|error| vec![format!("pyenv: failed to serialize JSON output: {error}")])
}

fn render_json_report<T: Serialize>(value: &T) -> CommandReport {
    match serde_json::to_string_pretty(value) {
        Ok(json) => CommandReport::success(json.lines().map(ToOwned::to_owned).collect()),
        Err(error) => CommandReport::failure(
            vec![format!("pyenv: failed to serialize JSON output: {error}")],
            1,
        ),
    }
}

fn sanitize_for_fs(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn unique_suffix() -> String {
    format!(
        "{}-{}",
        process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    )
}

fn io_error(error: std::io::Error) -> PyenvError {
    PyenvError::Io(format!("pyenv: {error}"))
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::ffi::OsString;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    use bzip2::Compression;
    use bzip2::write::BzEncoder;
    use tar::Builder;
    use tempfile::TempDir;
    use zip::write::FileOptions;

    use crate::config::{AppConfig, RuntimeArch};
    use crate::context::AppContext;

    use super::{
        InstallCommandOptions, PypyReleaseFile, PypyReleaseManifest, cmd_install,
        cmd_provider_install_list, cpython_source_provider_entries, ensure_unix_runtime_aliases,
        extract_root_archive, extract_tar_root_archive, extract_tools_archive,
        nuget_index_cache_path, pip_wrapper_names, provider_catalog_entries_for_platform,
        pypy_index_cache_path, resolve_install_plan_for_platform, resolve_python_build_path,
        write_nuget_index_cache, write_pypy_index_cache,
    };

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

    fn test_path_ext() -> Option<OsString> {
        if cfg!(windows) {
            Some(OsString::from(".exe;.cmd;.bat"))
        } else {
            None
        }
    }

    fn seed_package_index(ctx: &AppContext, package_name: &str, versions: &[&str]) {
        let cache_path = nuget_index_cache_path(ctx, package_name);
        let owned = versions
            .iter()
            .map(|version| version.to_string())
            .collect::<Vec<_>>();
        write_nuget_index_cache(&cache_path, &owned).expect("write cache");
    }

    fn seed_pypy_index(ctx: &AppContext, releases: &[PypyReleaseManifest]) {
        let cache_path = pypy_index_cache_path(ctx);
        write_pypy_index_cache(&cache_path, releases).expect("write pypy cache");
    }

    fn write_fake_python_build(temp: &TempDir, definitions: &[&str]) -> PathBuf {
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

    fn pypy_release(
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
    fn extract_tools_archive_strips_prefix() {
        let temp = TempDir::new().expect("tempdir");
        let archive_path = temp.path().join("test.nupkg");
        let output_dir = temp.path().join("out");
        let file = fs::File::create(&archive_path).expect("archive");
        let mut writer = zip::ZipWriter::new(file);
        let options = FileOptions::<()>::default();
        writer.add_directory("tools/Lib/", options).expect("dir");
        writer
            .start_file("tools/python.exe", options)
            .expect("file");
        writer.write_all(b"python").expect("write");
        writer
            .start_file("tools/Lib/test.py", options)
            .expect("file");
        writer.write_all(b"pass").expect("write");
        writer.finish().expect("finish");

        extract_tools_archive(&archive_path, &output_dir).expect("extract");

        assert!(output_dir.join("python.exe").is_file());
        assert!(output_dir.join("Lib").join("test.py").is_file());
        assert!(!output_dir.join("tools").exists());
    }

    #[test]
    fn extract_root_archive_strips_top_level_directory() {
        let temp = TempDir::new().expect("tempdir");
        let archive_path = temp.path().join("test.zip");
        let output_dir = temp.path().join("out");
        let file = fs::File::create(&archive_path).expect("archive");
        let mut writer = zip::ZipWriter::new(file);
        let options = FileOptions::<()>::default();
        writer.add_directory("runtime/Lib/", options).expect("dir");
        writer
            .start_file("runtime/python.exe", options)
            .expect("file");
        writer.write_all(b"python").expect("write");
        writer
            .start_file("runtime/Lib/test.py", options)
            .expect("file");
        writer.write_all(b"pass").expect("write");
        writer.finish().expect("finish");

        extract_root_archive(&archive_path, &output_dir).expect("extract");

        assert!(output_dir.join("python.exe").is_file());
        assert!(output_dir.join("Lib").join("test.py").is_file());
        assert!(!output_dir.join("runtime").exists());
    }

    #[test]
    fn extract_tar_root_archive_strips_top_level_directory() {
        let temp = TempDir::new().expect("tempdir");
        let archive_path = temp.path().join("test.tar.bz2");
        let output_dir = temp.path().join("out");
        let file = fs::File::create(&archive_path).expect("archive");
        let encoder = BzEncoder::new(file, Compression::best());
        let mut builder = Builder::new(encoder);

        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Directory);
        header.set_mode(0o755);
        header.set_size(0);
        header.set_cksum();
        builder
            .append_data(&mut header, "runtime/Lib", std::io::empty())
            .expect("dir");

        let mut file_header = tar::Header::new_gnu();
        file_header.set_mode(0o755);
        file_header.set_size(6);
        file_header.set_cksum();
        builder
            .append_data(&mut file_header, "runtime/bin/pypy3", &b"python"[..])
            .expect("binary");

        let mut lib_header = tar::Header::new_gnu();
        lib_header.set_mode(0o644);
        lib_header.set_size(4);
        lib_header.set_cksum();
        builder
            .append_data(&mut lib_header, "runtime/Lib/test.py", &b"pass"[..])
            .expect("lib");
        let encoder = builder.into_inner().expect("encoder");
        encoder.finish().expect("finish");

        extract_tar_root_archive(&archive_path, &output_dir).expect("extract");

        assert!(output_dir.join("bin").join("pypy3").is_file());
        assert!(output_dir.join("Lib").join("test.py").is_file());
        assert!(!output_dir.join("runtime").exists());
    }

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
    fn pip_wrapper_names_include_versioned_commands() {
        assert_eq!(
            pip_wrapper_names("3.13.12"),
            vec!["pip".to_string(), "pip3".to_string(), "pip3.13".to_string()]
        );
    }

    #[test]
    fn resolve_install_plan_delegates_to_python_build_on_linux() {
        let (temp, mut ctx) = test_context();
        let script = write_fake_python_build(&temp, &["stackless-3.7.5", "pypy3.10-7.3.15"]);
        ctx.config.install.python_build_path = Some(script);

        let plan =
            resolve_install_plan_for_platform(&ctx, "stackless-3.7.5", "linux").expect("plan");

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

        let mut arm_ctx = ctx;
        arm_ctx.config.install.arch = RuntimeArch::Arm64;
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
    fn provider_catalog_entries_still_include_python_build_non_pypy_on_macos() {
        let (temp, mut ctx) = test_context();
        let script = write_fake_python_build(&temp, &["stackless-3.7.5", "pypy3.10-7.3.15"]);
        ctx.config.install.python_build_path = Some(script);

        let entries = provider_catalog_entries_for_platform(
            &ctx,
            Some("stackless"),
            Some("stackless"),
            "macos",
        )
        .expect("entries");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].provider, "macos-python-build");
        assert_eq!(entries[0].family_slug, "stackless");
        assert_eq!(entries[0].version, "stackless-3.7.5");
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
        ctx.path_env = Some(env::join_paths([tool_dir]).expect("path env"));
        ctx.path_ext = test_path_ext();

        let resolved = resolve_python_build_path(&ctx).expect("python-build path");
        assert_eq!(
            resolved.to_string_lossy().to_ascii_lowercase(),
            final_script.to_string_lossy().to_ascii_lowercase()
        );
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

    #[test]
    fn unix_runtime_aliases_create_python_and_pip_links() {
        let temp = TempDir::new().expect("tempdir");
        let prefix = temp.path().join("runtime");
        let bin = prefix.join("bin");
        fs::create_dir_all(&bin).expect("bin");
        fs::write(bin.join("python3"), "").expect("python3");
        fs::write(bin.join("pip3"), "").expect("pip3");

        ensure_unix_runtime_aliases(&prefix, "3.12.10").expect("aliases");

        assert!(bin.join("python").exists());
        assert!(bin.join("pip").exists());
    }
}
