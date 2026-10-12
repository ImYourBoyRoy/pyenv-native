// ./crates/pyenv-core/src/catalog/entries.rs
//! Known/installed version loading plus catalog entry grouping and filtering.

use std::collections::BTreeMap;
use std::fs;
use std::sync::OnceLock;

use crate::context::AppContext;
use crate::error::PyenvError;

use super::families::VersionFamily;
use super::latest::compare_version_names;
use super::types::{CatalogEntry, CatalogGroup, CatalogSourceKind, InstallListOptions};

static KNOWN_VERSION_NAMES: OnceLock<Vec<String>> = OnceLock::new();

pub fn known_version_names() -> &'static [String] {
    KNOWN_VERSION_NAMES
        .get_or_init(|| parse_known_version_lines(include_str!("../../data/known_versions.txt")))
        .as_slice()
}

pub(crate) fn parse_known_version_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|line: &str| line.trim_start_matches('\u{feff}').trim())
        .filter(|line: &&str| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect()
}

pub fn installed_version_names(ctx: &AppContext) -> Result<Vec<String>, PyenvError> {
    let versions_dir = ctx.versions_dir();
    if !versions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut versions = fs::read_dir(&versions_dir)
        .map_err(|error| PyenvError::Io(format!("pyenv: {error}")))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            entry
                .path()
                .is_dir()
                .then(|| entry.file_name().to_string_lossy().to_string())
        })
        .collect::<Vec<_>>();

    versions.sort_by(|lhs, rhs| compare_version_names(lhs, rhs));
    Ok(versions)
}

pub(crate) fn known_catalog_entries(ctx: &AppContext) -> Vec<CatalogEntry> {
    super::remote::merged_known_version_names(ctx)
        .into_iter()
        .map(|name| catalog_entry(name, CatalogSourceKind::Known))
        .collect()
}

pub(crate) fn catalog_entry(name: String, source: CatalogSourceKind) -> CatalogEntry {
    let family = VersionFamily::classify(&name);
    CatalogEntry {
        name,
        family: family.label(),
        family_slug: family.slug(),
        source,
    }
}

pub(crate) fn group_entries(entries: Vec<CatalogEntry>) -> Vec<CatalogGroup> {
    let mut groups = BTreeMap::<(usize, String, String, CatalogSourceKind), Vec<String>>::new();

    for entry in entries {
        let family = VersionFamily::classify(&entry.name);
        let key = (
            family.rank(),
            entry.family.clone(),
            entry.family_slug.clone(),
            entry.source,
        );
        groups.entry(key).or_default().push(entry.name);
    }

    groups
        .into_iter()
        .map(|((_, family, family_slug, source), mut versions)| {
            versions.sort_by(|lhs, rhs| compare_version_names(lhs, rhs).reverse());
            CatalogGroup {
                family,
                family_slug,
                source,
                versions,
            }
        })
        .collect()
}

pub(crate) fn filter_catalog_entries(
    entries: Vec<CatalogEntry>,
    options: &InstallListOptions,
) -> Vec<CatalogEntry> {
    let family_filter = options
        .family
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase());
    let pattern_filter = options
        .pattern
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase());

    entries
        .into_iter()
        .filter(|entry| {
            family_filter.as_ref().is_none_or(|filter| {
                entry.family_slug == *filter || entry.family.to_ascii_lowercase() == *filter
            })
        })
        .filter(|entry| {
            pattern_filter.as_ref().is_none_or(|filter| {
                matches_catalog_pattern(&entry.name, filter)
                    || matches_catalog_pattern(&entry.family, filter)
                    || matches_catalog_pattern(&entry.family_slug, filter)
            })
        })
        .collect()
}

pub(crate) fn matches_catalog_pattern(haystack: &str, filter: &str) -> bool {
    let haystack = haystack.to_ascii_lowercase();
    let filter = filter.to_ascii_lowercase();
    if filter.is_empty() {
        return true;
    }
    if !filter.starts_with(|ch: char| ch.is_ascii_digit()) {
        return haystack.contains(&filter);
    }

    let ends_with_digit = filter.ends_with(|ch: char| ch.is_ascii_digit());
    let mut start = 0;
    while let Some(relative) = haystack[start..].find(&filter) {
        let index = start + relative;
        let after = &haystack[index + filter.len()..];
        let next_ok = after
            .chars()
            .next()
            .is_none_or(|ch| !(ends_with_digit && ch.is_ascii_digit()));
        let previous_ok = index == 0 || !haystack.as_bytes()[index - 1].is_ascii_digit();
        if next_ok && previous_ok {
            return true;
        }
        start = index + 1;
    }
    false
}
