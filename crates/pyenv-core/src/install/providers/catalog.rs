// ./crates/pyenv-core/src/install/providers/catalog.rs
//! Provider-backed catalog discovery and rendering helpers for installable runtimes.

use std::collections::BTreeMap;

use crate::catalog::{VersionFamily, known_version_names};
use crate::command::CommandReport;
use crate::context::AppContext;
use crate::error::PyenvError;

use super::super::fetch::{
    is_stable_runtime_version, load_or_fetch_pypy_releases, pypy_provider_names,
};
use super::super::platform::{
    cpython_source_provider_name, family_filter_matches_provider, is_windows_platform,
    pypy_provider_name, python_build_provider_name,
};
use super::super::report::render_json_report;
use super::super::types::{InstallCommandOptions, ProviderCatalogEntry, ProviderCatalogGroup};
use super::python_build::load_python_build_definitions;
use super::versioning::{available_package_versions, nuget_package_name};

pub(crate) fn cmd_provider_install_list(
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
        let mut stderr =
            vec!["pyenv: no installable versions match the requested filters".to_string()];
        if let Some(pattern) = pattern.as_deref() {
            stderr.push(format!(
                "hint: try `pyenv install --list --known {pattern}` to inspect the broader embedded catalog"
            ));
        }
        return CommandReport::failure(stderr, 1);
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

pub(crate) fn provider_catalog_entries_for_platform(
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
                Err(error) if entries.is_empty() => return Err(error),
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

pub(crate) fn cpython_source_provider_versions() -> Vec<String> {
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

pub(crate) fn cpython_source_provider_entries(
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
