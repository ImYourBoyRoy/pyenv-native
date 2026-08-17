// ./crates/pyenv-core/src/i18n/negotiate.rs
//! Resolve PYENV_LANG, config ui.language, and OS UI languages onto a supported tag.

use std::env;
use std::sync::{Mutex, OnceLock};

use fluent_langneg::{NegotiationStrategy, negotiate_languages};
use unic_langid::{LanguageIdentifier, langid};

use super::catalog::SUPPORTED_LOCALES;

const FALLBACK: LanguageIdentifier = langid!("en-US");

static CURRENT: OnceLock<Mutex<LanguageIdentifier>> = OnceLock::new();

fn current_lock() -> &'static Mutex<LanguageIdentifier> {
    CURRENT.get_or_init(|| Mutex::new(FALLBACK.clone()))
}

pub fn current_lang() -> LanguageIdentifier {
    current_lock()
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| FALLBACK.clone())
}

pub fn current_lang_tag() -> String {
    current_lang().to_string()
}

pub fn set_lang_tag(tag: &str) {
    let resolved = negotiate(&[tag.to_string()]);
    if let Ok(mut guard) = current_lock().lock() {
        *guard = resolved;
    }
}

/// OS (and PYENV_LANG) negotiation. Safe to call more than once.
pub fn init() {
    if env::var_os("PYENV_LANG").is_some() {
        let requested = env::var("PYENV_LANG").unwrap_or_default();
        set_requested(&requested);
        return;
    }
    let os_tags: Vec<String> = sys_locale::get_locales().collect();
    let resolved = negotiate(&os_tags);
    if let Ok(mut guard) = current_lock().lock() {
        *guard = resolved;
    }
}

/// Config wins over OS unless language is `auto`. PYENV_LANG still wins.
pub fn apply_config_language(configured: &str) {
    if env::var_os("PYENV_LANG").is_some() {
        return;
    }
    let trimmed = configured.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
        return;
    }
    set_requested(trimmed);
}

fn set_requested(tag: &str) {
    let resolved = negotiate(&[tag.to_string()]);
    if let Ok(mut guard) = current_lock().lock() {
        *guard = resolved;
    }
}

/// Map aliases (zh, zh-TW, pt, en-GB, …) onto the catalogs we ship.
pub fn negotiate(requested: &[String]) -> LanguageIdentifier {
    let available: Vec<LanguageIdentifier> = SUPPORTED_LOCALES
        .iter()
        .filter_map(|info| info.tag.parse().ok())
        .collect();

    let mut requested_ids: Vec<LanguageIdentifier> = Vec::new();
    for raw in requested {
        let mapped = map_alias(raw);
        if let Ok(id) = mapped.parse::<LanguageIdentifier>() {
            requested_ids.push(id);
        }
    }

    negotiate_languages(
        &requested_ids,
        &available,
        Some(&FALLBACK),
        NegotiationStrategy::Filtering,
    )
    .into_iter()
    .next()
    .cloned()
    .unwrap_or(FALLBACK)
}

fn map_alias(raw: &str) -> String {
    let trimmed = raw.trim().replace('_', "-");
    if trimmed.is_empty() {
        return "en-US".to_string();
    }
    let lower = trimmed.to_ascii_lowercase();
    match lower.as_str() {
        "zh" | "zh-cn" | "zh-hans" | "zh-hans-cn" | "zh-sg" | "zh-tw" | "zh-hant"
        | "zh-hant-tw" | "zh-hk" | "zh-mo" => "zh-CN".to_string(),
        "pt" | "pt-pt" | "pt-br" => "pt-BR".to_string(),
        "en" | "en-us" | "en-gb" | "en-au" | "en-ca" => "en-US".to_string(),
        "es-mx" | "es-ar" | "es-es" | "es-419" => "es".to_string(),
        "fa-ir" | "fa-af" => "fa".to_string(),
        "ar-sa" | "ar-eg" | "ar-ae" => "ar".to_string(),
        other => {
            // Keep region for tags we ship (pt-BR); otherwise language subtag only.
            if SUPPORTED_LOCALES
                .iter()
                .any(|info| info.tag.eq_ignore_ascii_case(other))
            {
                return SUPPORTED_LOCALES
                    .iter()
                    .find(|info| info.tag.eq_ignore_ascii_case(other))
                    .map(|info| info.tag.to_string())
                    .unwrap_or_else(|| trimmed);
            }
            trimmed.split('-').next().unwrap_or("en").to_string()
        }
    }
}
