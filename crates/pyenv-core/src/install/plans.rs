// ./crates/pyenv-core/src/install/plans.rs
//! Install-plan resolution across native providers and platform-specific backends.

use crate::catalog::{
    InstallListOptions, VersionFamily, cmd_install_list, latest_version_from_names,
};
use crate::command::CommandReport;
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::meta::cmd_help;
use crate::runtime::BASE_VENV_DIR_NAME;
use crate::version::installed_version_dir;

use super::fetch::{
    find_pypy_release_by_provider_name, load_or_fetch_pypy_releases, pypy_provider_names,
};
use super::platform::{
    cpython_source_provider_name, cpython_source_python_executable_path, current_platform,
    is_windows_platform, pypy_manifest_arches, pypy_provider_name, pypy_python_executable_path,
    python_build_provider_name,
};
use super::providers::{
    cmd_provider_install_list, cpython_source_provider_versions, ensure_supported_cpython_version,
    is_free_threaded, is_pypy_request, load_python_build_definitions, normalize_requested_version,
    nuget_package_name, resolve_provider_version,
};
use super::report::{
    render_install_error_lines, render_json_lines, render_outcome_lines, render_plan_lines,
    sanitize_for_fs,
};
use super::runtime::install_runtime;
use super::types::{
    DEFAULT_CPYTHON_SOURCE_BASE_URL, DEFAULT_NUGET_BASE_URL, InstallCommandOptions, InstallPlan,
};

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

    if is_split_help_request(&options.versions) {
        return cmd_help(ctx, Some("install"), false);
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
                        Err(error) => stderr.extend(render_install_error_lines(&error, requested)),
                    }
                }
            }
            Err(error) => stderr.extend(render_install_error_lines(&error, requested)),
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

fn is_split_help_request(versions: &[String]) -> bool {
    if versions.is_empty() {
        return false;
    }

    let joined = versions.concat().to_ascii_lowercase();
    matches!(joined.as_str(), "-help" | "--help" | "/?")
}

pub fn cmd_available(
    ctx: &AppContext,
    family: Option<String>,
    pattern: Option<String>,
    known: bool,
    json: bool,
) -> CommandReport {
    cmd_install(
        ctx,
        &InstallCommandOptions {
            list: true,
            force: false,
            dry_run: false,
            json,
            known,
            family,
            versions: pattern.into_iter().collect(),
        },
    )
}

pub fn resolve_install_plan(ctx: &AppContext, requested: &str) -> Result<InstallPlan, PyenvError> {
    resolve_install_plan_for_platform(ctx, requested, current_platform())
}

pub(super) fn resolve_install_plan_for_platform(
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
