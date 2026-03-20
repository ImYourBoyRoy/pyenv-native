// ./crates/pyenv-core/src/self_update/versioning.rs
//! Release tag normalization and semver-ish comparison helpers for self-update.

use std::cmp::Ordering;

pub(super) fn normalize_tag(tag: &str) -> String {
    let trimmed = tag.trim();
    if trimmed.starts_with('v') || trimmed.starts_with('V') {
        format!("v{}", &trimmed[1..])
    } else {
        format!("v{trimmed}")
    }
}

pub(super) fn compare_release_versions(left: &str, right: &str) -> Ordering {
    let left_parsed = parse_semverish(left);
    let right_parsed = parse_semverish(right);
    compare_semverish(&left_parsed, &right_parsed)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParsedVersion {
    pub numeric: Vec<u64>,
    pub pre_release: Option<String>,
}

pub(super) fn parse_semverish(value: &str) -> ParsedVersion {
    let normalized = value.trim().trim_start_matches(['v', 'V']);
    let (core, pre_release) = match normalized.split_once('-') {
        Some((core, pre)) => (core, Some(pre.to_string())),
        None => (normalized, None),
    };

    let numeric = core
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect::<Vec<_>>();

    ParsedVersion {
        numeric,
        pre_release,
    }
}

fn compare_semverish(left: &ParsedVersion, right: &ParsedVersion) -> Ordering {
    let max_len = left.numeric.len().max(right.numeric.len());
    for index in 0..max_len {
        let left_value = *left.numeric.get(index).unwrap_or(&0);
        let right_value = *right.numeric.get(index).unwrap_or(&0);
        match left_value.cmp(&right_value) {
            Ordering::Equal => continue,
            ordering => return ordering,
        }
    }

    match (&left.pre_release, &right.pre_release) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(left_pre), Some(right_pre)) => left_pre.cmp(right_pre),
    }
}
