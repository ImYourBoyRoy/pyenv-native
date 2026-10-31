// ./crates/pyenv-core/src/catalog/latest.rs
//! Natural version ordering plus prefix/latest resolution for installed and known catalogs.

use std::cmp::Ordering;

use crate::context::AppContext;

use super::entries::installed_version_names;
use super::remote::merged_known_version_names;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReleaseChannel {
    Alpha,
    Beta,
    ReleaseCandidate,
    Stable,
}

impl ReleaseChannel {
    pub(crate) const fn label(self) -> Option<&'static str> {
        match self {
            Self::Alpha => Some("alpha"),
            Self::Beta => Some("beta"),
            Self::ReleaseCandidate => Some("release candidate"),
            Self::Stable => None,
        }
    }

    const fn rank(self) -> u8 {
        match self {
            Self::Alpha => 0,
            Self::Beta => 1,
            Self::ReleaseCandidate => 2,
            Self::Stable => 3,
        }
    }
}

pub fn latest_installed_version(ctx: &AppContext, prefix: &str) -> Option<String> {
    installed_version_names(ctx)
        .ok()
        .and_then(|versions| latest_version_from_names(prefix, &versions))
}

pub fn latest_known_version(ctx: &AppContext, prefix: &str) -> Option<String> {
    latest_version_from_names(prefix, &merged_known_version_names(ctx))
}

pub fn compare_version_names(lhs: &str, rhs: &str) -> Ordering {
    let (lhs_family, lhs_tail) = split_family_and_tail(lhs);
    let (rhs_family, rhs_tail) = split_family_and_tail(rhs);
    lhs_family
        .cmp(&rhs_family)
        .then_with(|| {
            compare_numeric_parts(
                &extract_release_numeric_parts(lhs, &lhs_tail),
                &extract_release_numeric_parts(rhs, &rhs_tail),
            )
        })
        .then_with(|| compare_release_stages(lhs, rhs))
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

    let canonical_trimmed = canonicalize_version_name(trimmed);
    if let Some(name) = names
        .iter()
        .find(|name| canonicalize_version_name(name.as_ref()) == canonical_trimmed)
    {
        return Some(name.as_ref().to_string());
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

fn extract_release_numeric_parts(value: &str, fallback: &str) -> Vec<u32> {
    let Some(channel) = release_channel(value) else {
        return extract_numeric_parts(fallback);
    };
    if channel == ReleaseChannel::Stable {
        return extract_numeric_parts(fallback);
    }

    let probe = split_t_suffix(value).0;
    let Some((_, _, marker_index)) = prerelease_parts(probe) else {
        return extract_numeric_parts(fallback);
    };
    extract_numeric_parts(&probe[..marker_index])
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
    let lowered = canonicalize_version_name(candidate);
    let lowered_prefix = canonicalize_version_name(prefix_core);
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

    matches!(
        release_channel(&lowered),
        Some(ReleaseChannel::Alpha | ReleaseChannel::Beta | ReleaseChannel::ReleaseCandidate)
    )
}

pub(crate) fn is_cpython_version(value: &str) -> bool {
    release_channel(value).is_some()
}

pub(crate) fn release_channel(value: &str) -> Option<ReleaseChannel> {
    let probe = split_t_suffix(value.trim()).0;
    if probe.is_empty() || !probe.starts_with(|ch: char| ch.is_ascii_digit()) {
        return None;
    }

    if let Some((channel, _, _)) = prerelease_parts(probe) {
        return Some(channel);
    }

    probe
        .bytes()
        .all(|byte| byte.is_ascii_digit() || byte == b'.')
        .then_some(ReleaseChannel::Stable)
}

pub(crate) fn release_channel_label(value: &str) -> Option<&'static str> {
    release_channel(value).and_then(ReleaseChannel::label)
}

pub(crate) fn display_version_name(value: &str) -> String {
    match release_channel_label(value) {
        Some(label) => format!("{value} ({label})"),
        None => value.to_string(),
    }
}

pub(crate) fn version_names_equal(lhs: &str, rhs: &str) -> bool {
    canonicalize_version_name(lhs) == canonicalize_version_name(rhs)
}

pub(crate) fn release_directory_name(value: &str) -> String {
    let (base, _) = split_t_suffix(value.trim());
    if let Some((_, _, marker_index)) = prerelease_parts(base) {
        return base[..marker_index]
            .trim_end_matches(['-', '.'])
            .to_string();
    }
    base.to_string()
}

fn compare_release_stages(lhs: &str, rhs: &str) -> Ordering {
    release_sort_key(lhs).cmp(&release_sort_key(rhs))
}

fn release_sort_key(value: &str) -> (u8, u32) {
    match release_channel(value) {
        Some(channel) => {
            let sequence = prerelease_parts(split_t_suffix(value).0)
                .map(|(_, sequence, _)| sequence)
                .unwrap_or(0);
            (channel.rank(), sequence)
        }
        None => (ReleaseChannel::Stable.rank(), 0),
    }
}

fn prerelease_parts(value: &str) -> Option<(ReleaseChannel, u32, usize)> {
    let lowered = value.to_ascii_lowercase();
    let bytes = lowered.as_bytes();
    for index in 1..bytes.len() {
        let (channel, marker_len) = if bytes[index..].starts_with(b"rc") {
            (ReleaseChannel::ReleaseCandidate, 2)
        } else if bytes[index] == b'a' {
            (ReleaseChannel::Alpha, 1)
        } else if bytes[index] == b'b' {
            (ReleaseChannel::Beta, 1)
        } else {
            continue;
        };

        let previous = bytes[index - 1];
        if !(previous.is_ascii_digit() || previous == b'-' || previous == b'.') {
            continue;
        }
        let digits = &bytes[index + marker_len..];
        if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
            continue;
        }
        let sequence = std::str::from_utf8(digits).ok()?.parse().ok()?;
        return Some((channel, sequence, index));
    }
    None
}

fn canonicalize_version_name(value: &str) -> String {
    let (base, free_threaded) = split_t_suffix(value.trim());
    let lowered = base.to_ascii_lowercase();
    let bytes = lowered.as_bytes();
    let mut canonical = String::with_capacity(lowered.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'-'
            && index > 0
            && bytes[index - 1].is_ascii_digit()
            && (bytes[index + 1..].starts_with(b"rc")
                || bytes[index + 1..].starts_with(b"a")
                || bytes[index + 1..].starts_with(b"b"))
        {
            let marker_len = if bytes[index + 1..].starts_with(b"rc") {
                2
            } else {
                1
            };
            let digits = &bytes[index + 1 + marker_len..];
            if !digits.is_empty() && digits.iter().all(u8::is_ascii_digit) {
                index += 1;
                continue;
            }
        }
        canonical.push(bytes[index] as char);
        index += 1;
    }
    if free_threaded {
        canonical.push('t');
    }
    canonical
}
