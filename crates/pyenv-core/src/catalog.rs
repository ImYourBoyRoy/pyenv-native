// ./crates/pyenv-core/src/catalog.rs
//! Catalog models for installable runtimes, grouped install listings, and prefix resolution.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::sync::OnceLock;

use serde::Serialize;

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::error::PyenvError;

static KNOWN_VERSION_NAMES: OnceLock<Vec<String>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CatalogSourceKind {
    Installed,
    Known,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CatalogEntry {
    pub name: String,
    pub family: String,
    pub family_slug: String,
    pub source: CatalogSourceKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CatalogGroup {
    pub family: String,
    pub family_slug: String,
    pub source: CatalogSourceKind,
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InstallListOptions {
    pub family: Option<String>,
    pub json: bool,
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VersionFamily {
    CPython,
    PyPy,
    Miniforge,
    Mambaforge,
    Miniconda,
    Anaconda,
    GraalPy,
    Stackless,
    Pyston,
    MicroPython,
    Jython,
    IronPython,
    ActivePython,
    Cinder,
    Nogil,
    Other(String),
}

impl VersionFamily {
    pub(crate) fn classify(name: &str) -> Self {
        let lowered = name.to_ascii_lowercase();
        if lowered.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
            Self::CPython
        } else if lowered.starts_with("pypy") {
            Self::PyPy
        } else if lowered.starts_with("miniforge") {
            Self::Miniforge
        } else if lowered.starts_with("mambaforge") {
            Self::Mambaforge
        } else if lowered.starts_with("miniconda") {
            Self::Miniconda
        } else if lowered.starts_with("anaconda") {
            Self::Anaconda
        } else if lowered.starts_with("graalpy") || lowered.starts_with("graalpython") {
            Self::GraalPy
        } else if lowered.starts_with("stackless") {
            Self::Stackless
        } else if lowered.starts_with("pyston") {
            Self::Pyston
        } else if lowered.starts_with("micropython") {
            Self::MicroPython
        } else if lowered.starts_with("jython") {
            Self::Jython
        } else if lowered.starts_with("ironpython") {
            Self::IronPython
        } else if lowered.starts_with("activepython") {
            Self::ActivePython
        } else if lowered.starts_with("cinder") {
            Self::Cinder
        } else if lowered.starts_with("nogil") {
            Self::Nogil
        } else {
            let family = lowered
                .split(['-', '.'])
                .next()
                .filter(|segment| !segment.is_empty())
                .unwrap_or("other");
            Self::Other(title_case(family))
        }
    }

    pub(crate) fn label(&self) -> String {
        match self {
            Self::CPython => "CPython".to_string(),
            Self::PyPy => "PyPy".to_string(),
            Self::Miniforge => "Miniforge".to_string(),
            Self::Mambaforge => "Mambaforge".to_string(),
            Self::Miniconda => "Miniconda".to_string(),
            Self::Anaconda => "Anaconda".to_string(),
            Self::GraalPy => "GraalPy".to_string(),
            Self::Stackless => "Stackless".to_string(),
            Self::Pyston => "Pyston".to_string(),
            Self::MicroPython => "MicroPython".to_string(),
            Self::Jython => "Jython".to_string(),
            Self::IronPython => "IronPython".to_string(),
            Self::ActivePython => "ActivePython".to_string(),
            Self::Cinder => "Cinder".to_string(),
            Self::Nogil => "Nogil".to_string(),
            Self::Other(label) => label.clone(),
        }
    }

    pub(crate) fn slug(&self) -> String {
        match self {
            Self::CPython => "cpython".to_string(),
            Self::PyPy => "pypy".to_string(),
            Self::Miniforge => "miniforge".to_string(),
            Self::Mambaforge => "mambaforge".to_string(),
            Self::Miniconda => "miniconda".to_string(),
            Self::Anaconda => "anaconda".to_string(),
            Self::GraalPy => "graalpy".to_string(),
            Self::Stackless => "stackless".to_string(),
            Self::Pyston => "pyston".to_string(),
            Self::MicroPython => "micropython".to_string(),
            Self::Jython => "jython".to_string(),
            Self::IronPython => "ironpython".to_string(),
            Self::ActivePython => "activepython".to_string(),
            Self::Cinder => "cinder".to_string(),
            Self::Nogil => "nogil".to_string(),
            Self::Other(label) => label.to_ascii_lowercase().replace(' ', "-"),
        }
    }

    fn rank(&self) -> usize {
        match self {
            Self::CPython => 0,
            Self::PyPy => 1,
            Self::Miniforge => 2,
            Self::Mambaforge => 3,
            Self::Miniconda => 4,
            Self::Anaconda => 5,
            Self::GraalPy => 6,
            Self::Stackless => 7,
            Self::Pyston => 8,
            Self::MicroPython => 9,
            Self::Jython => 10,
            Self::IronPython => 11,
            Self::ActivePython => 12,
            Self::Cinder => 13,
            Self::Nogil => 14,
            Self::Other(_) => 15,
        }
    }
}

pub fn known_version_names() -> &'static [String] {
    KNOWN_VERSION_NAMES
        .get_or_init(|| {
            include_str!("../data/known_versions.txt")
                .lines()
                .map(|line| line.trim_start_matches('\u{feff}').trim())
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(ToOwned::to_owned)
                .collect()
        })
        .as_slice()
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

pub fn latest_installed_version(ctx: &AppContext, prefix: &str) -> Option<String> {
    installed_version_names(ctx)
        .ok()
        .and_then(|versions| latest_version_from_names(prefix, &versions))
}

pub fn latest_known_version(prefix: &str) -> Option<String> {
    latest_version_from_names(prefix, known_version_names())
}

pub fn compare_version_names(lhs: &str, rhs: &str) -> Ordering {
    let (lhs_family, lhs_tail) = split_family_and_tail(lhs);
    let (rhs_family, rhs_tail) = split_family_and_tail(rhs);
    lhs_family
        .cmp(&rhs_family)
        .then_with(|| {
            compare_numeric_parts(
                &extract_numeric_parts(&lhs_tail),
                &extract_numeric_parts(&rhs_tail),
            )
        })
        .then_with(|| is_t_variant(lhs).cmp(&is_t_variant(rhs)))
        .then_with(|| lhs.to_ascii_lowercase().cmp(&rhs.to_ascii_lowercase()))
}

pub fn cmd_install_list(_ctx: &AppContext, options: &InstallListOptions) -> CommandReport {
    let entries = filter_catalog_entries(known_catalog_entries(), options);
    if entries.is_empty() {
        return CommandReport::failure(
            vec!["pyenv: no known versions match the requested filters".to_string()],
            1,
        );
    }

    let groups = group_entries(entries);
    if options.json {
        match serde_json::to_string_pretty(&groups) {
            Ok(json) => CommandReport::success(json.lines().map(ToOwned::to_owned).collect()),
            Err(error) => CommandReport::failure(
                vec![format!(
                    "pyenv: failed to serialize install catalog: {error}"
                )],
                1,
            ),
        }
    } else {
        let mut stdout = vec!["Available versions:".to_string()];
        for group in &groups {
            stdout.push(String::new());
            stdout.push(group.family.clone());
            stdout.extend(group.versions.iter().map(|version| format!("  {version}")));
        }
        CommandReport::success(stdout)
    }
}

pub fn cmd_latest(
    ctx: &AppContext,
    prefix: &str,
    known: bool,
    bypass: bool,
    force: bool,
) -> CommandReport {
    let resolved = if known {
        latest_known_version(prefix)
    } else {
        latest_installed_version(ctx, prefix)
    };

    if let Some(version) = resolved {
        return CommandReport::success_one(version);
    }

    if bypass {
        return CommandReport {
            stdout: vec![prefix.to_string()],
            stderr: Vec::new(),
            exit_code: if force { 0 } else { 1 },
        };
    }

    let scope = if known { "known" } else { "installed" };
    CommandReport::failure(
        vec![format!(
            "pyenv: no {scope} versions match the prefix `{prefix}'"
        )],
        1,
    )
}

pub fn latest_version_from_names<S>(prefix: &str, names: &[S]) -> Option<String>
where
    S: AsRef<str>,
{
    let trimmed = prefix.trim();
    if trimmed.is_empty() {
        return None;
    }

    if names.iter().any(|name| name.as_ref() == trimmed) {
        return Some(trimmed.to_string());
    }

    let (prefix_core, require_t) = split_t_suffix(trimmed);
    let mut candidates = names
        .iter()
        .map(AsRef::as_ref)
        .filter(|name| matches_latest_prefix(name, prefix_core, require_t))
        .filter(|name| !is_excluded_latest_candidate(name))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    candidates.sort_by(|lhs, rhs| compare_version_names(lhs, rhs).reverse());
    candidates.into_iter().next()
}

fn known_catalog_entries() -> Vec<CatalogEntry> {
    known_version_names()
        .iter()
        .cloned()
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
                entry.name.to_ascii_lowercase().contains(filter)
                    || entry.family.to_ascii_lowercase().contains(filter)
                    || entry.family_slug.contains(filter)
            })
        })
        .collect()
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

fn split_family_and_tail(name: &str) -> (String, String) {
    if name.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        return (String::new(), name.to_ascii_lowercase());
    }

    if let Some((family, tail)) = name.split_once('-')
        && family
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.')
    {
        return (family.to_ascii_lowercase(), tail.to_ascii_lowercase());
    }

    (name.to_ascii_lowercase(), String::new())
}

fn extract_numeric_parts(value: &str) -> Vec<u32> {
    value
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|segment| !segment.is_empty())
        .filter_map(|segment| segment.parse::<u32>().ok())
        .collect()
}

fn compare_numeric_parts(lhs: &[u32], rhs: &[u32]) -> Ordering {
    for index in 0..lhs.len().max(rhs.len()) {
        let left = lhs.get(index).copied().unwrap_or(0);
        let right = rhs.get(index).copied().unwrap_or(0);
        match left.cmp(&right) {
            Ordering::Equal => continue,
            ordering => return ordering,
        }
    }
    Ordering::Equal
}

fn split_t_suffix(value: &str) -> (&str, bool) {
    if value.len() > 1
        && value.ends_with('t')
        && value
            .chars()
            .nth_back(1)
            .is_some_and(|ch| ch.is_ascii_digit())
    {
        (&value[..value.len() - 1], true)
    } else {
        (value, false)
    }
}

fn is_t_variant(value: &str) -> bool {
    split_t_suffix(value).1
}

fn matches_latest_prefix(candidate: &str, prefix_core: &str, require_t: bool) -> bool {
    let lowered = candidate.to_ascii_lowercase();
    let lowered_prefix = prefix_core.to_ascii_lowercase();
    if lowered_prefix.is_empty() || !lowered.starts_with(&lowered_prefix) {
        return false;
    }

    if require_t != is_t_variant(&lowered) {
        return false;
    }

    lowered[prefix_core.len()..]
        .chars()
        .next()
        .is_some_and(|ch| ch == '.' || ch == '-')
}

fn is_excluded_latest_candidate(candidate: &str) -> bool {
    let lowered = candidate.to_ascii_lowercase();
    if lowered.ends_with("-dev") || lowered.ends_with("-src") || lowered.ends_with("-latest") {
        return true;
    }

    if lowered.contains("/envs/") || lowered.contains("\\envs\\") {
        return true;
    }

    let prerelease_probe = split_t_suffix(&lowered).0;
    let split_index = prerelease_probe
        .rfind(|ch: char| !ch.is_ascii_digit())
        .map(|index| index + 1)
        .unwrap_or(0);
    let (head, digits) = prerelease_probe.split_at(split_index);
    !digits.is_empty() && (head.ends_with("rc") || head.ends_with('a') || head.ends_with('b'))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::{
        CatalogSourceKind, InstallListOptions, cmd_install_list, cmd_latest, compare_version_names,
        installed_version_names, latest_version_from_names,
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
            path_ext: None,
            config: AppConfig::default(),
        };

        (temp, ctx)
    }

    #[test]
    fn latest_prefers_highest_cpython_match() {
        let names = vec!["3.5.6", "3.10.8", "3.10.6"];
        assert_eq!(
            latest_version_from_names("3", &names),
            Some("3.10.8".to_string())
        );
    }

    #[test]
    fn latest_filters_prereleases_and_t_variants_without_t_prefix() {
        let names = vec![
            "3.8.5-dev",
            "3.8.5-src",
            "3.8.5-latest",
            "3.8.5a2",
            "3.8.5b3",
            "3.8.5rc2",
            "3.8.5t",
            "3.8.1",
            "3.8.1/envs/demo",
        ];
        assert_eq!(
            latest_version_from_names("3.8", &names),
            Some("3.8.1".to_string())
        );
    }

    #[test]
    fn latest_honors_t_suffix_requests() {
        let names = vec!["3.13.2t", "3.13.5", "3.13.5t", "3.14.6"];
        assert_eq!(
            latest_version_from_names("3t", &names),
            Some("3.13.5t".to_string())
        );
    }

    #[test]
    fn compare_version_names_orders_versions_naturally() {
        let mut values = vec![
            "3.10.8".to_string(),
            "3.5.6".to_string(),
            "3.10.6".to_string(),
        ];
        values.sort_by(|lhs, rhs| compare_version_names(lhs, rhs));
        assert_eq!(values, vec!["3.5.6", "3.10.6", "3.10.8"]);
    }

    #[test]
    fn install_list_groups_families() {
        let (_temp, ctx) = test_context();
        let report = cmd_install_list(
            &ctx,
            &InstallListOptions {
                family: Some("cpython".to_string()),
                json: false,
                pattern: Some("3.13".to_string()),
            },
        );

        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout[0], "Available versions:");
        assert!(report.stdout.iter().any(|line| line == "CPython"));
        assert!(
            report
                .stdout
                .iter()
                .any(|line| line.trim_start().starts_with("3.13."))
        );
        assert!(report.stdout.iter().all(|line| !line.contains("PyPy")));
    }

    #[test]
    fn install_list_json_is_grouped() {
        let (_temp, ctx) = test_context();
        let report = cmd_install_list(
            &ctx,
            &InstallListOptions {
                family: Some("pypy".to_string()),
                json: true,
                pattern: Some("pypy3.11".to_string()),
            },
        );

        assert_eq!(report.exit_code, 0);
        let payload = report.stdout.join("\n");
        assert!(payload.contains("\"family\": \"PyPy\""));
        assert!(payload.contains("\"source\": \"known\""));
    }

    #[test]
    fn installed_version_names_are_sorted() {
        let (_temp, ctx) = test_context();
        for version in ["3.10.8", "3.5.6", "3.10.6"] {
            fs::create_dir_all(ctx.versions_dir().join(version)).expect("version dir");
        }

        assert_eq!(
            installed_version_names(&ctx).expect("installed"),
            vec![
                "3.5.6".to_string(),
                "3.10.6".to_string(),
                "3.10.8".to_string()
            ]
        );
    }

    #[test]
    fn latest_command_supports_bypass() {
        let (_temp, ctx) = test_context();
        let report = cmd_latest(&ctx, "nonexistent", false, true, true);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec!["nonexistent"]);
    }

    #[test]
    fn catalog_source_kind_serializes_in_kebab_case() {
        let value = serde_json::to_string(&CatalogSourceKind::Installed).expect("serialize");
        assert_eq!(value, "\"installed\"");
    }

    #[test]
    fn known_versions_strip_utf8_bom_from_first_entry() {
        assert!(
            super::known_version_names()
                .first()
                .is_some_and(|value| !value.starts_with('\u{feff}'))
        );
    }

    #[test]
    fn latest_version_from_names_resolution() {
        let names = vec!["3.13.12".to_string()];

        // Exact match
        assert_eq!(
            latest_version_from_names("3.13.12", &names),
            Some("3.13.12".to_string())
        );

        // Prefix matches
        assert_eq!(
            latest_version_from_names("3.13", &names),
            Some("3.13.12".to_string())
        );
        assert_eq!(
            latest_version_from_names("3", &names),
            Some("3.13.12".to_string())
        );

        // Non-matches
        assert_eq!(latest_version_from_names("3.12", &names), None);
        assert_eq!(latest_version_from_names("4", &names), None);
    }
}
