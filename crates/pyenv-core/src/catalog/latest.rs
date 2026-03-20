// ./crates/pyenv-core/src/catalog/latest.rs
//! Natural version ordering plus prefix/latest resolution for installed and known catalogs.

use std::cmp::Ordering;

use crate::context::AppContext;

use super::entries::{installed_version_names, known_version_names};

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
