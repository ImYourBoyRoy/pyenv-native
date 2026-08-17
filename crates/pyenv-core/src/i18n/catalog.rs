// ./crates/pyenv-core/src/i18n/catalog.rs
//! Fluent catalog loader and GUI message bundle.
//! Touch this file after editing `locales/**/*.ftl` so fluent-templates recompiles.
//! Last catalog touch: GUI self-update dialog body and related update-check copy.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::OnceLock;

use fluent_templates::fluent_bundle::FluentValue;
use fluent_templates::{Loader, static_loader};
use serde::Serialize;
use unic_langid::LanguageIdentifier;

use super::negotiate::current_lang;

static_loader! {
    static LOCALES = {
        locales: "../../locales",
        fallback_language: "en-US",
        customise: |bundle| bundle.set_use_isolating(false),
    };
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct LocaleInfo {
    pub tag: &'static str,
    pub native_name: &'static str,
    pub english_name: &'static str,
    pub rtl: bool,
}

pub const SUPPORTED_LOCALES: &[LocaleInfo] = &[
    LocaleInfo {
        tag: "en-US",
        native_name: "English",
        english_name: "English",
        rtl: false,
    },
    LocaleInfo {
        tag: "zh-CN",
        native_name: "简体中文",
        english_name: "Simplified Chinese",
        rtl: false,
    },
    LocaleInfo {
        tag: "es",
        native_name: "Español",
        english_name: "Spanish",
        rtl: false,
    },
    LocaleInfo {
        tag: "ja",
        native_name: "日本語",
        english_name: "Japanese",
        rtl: false,
    },
    LocaleInfo {
        tag: "ko",
        native_name: "한국어",
        english_name: "Korean",
        rtl: false,
    },
    LocaleInfo {
        tag: "pt-BR",
        native_name: "Português (Brasil)",
        english_name: "Portuguese (Brazil)",
        rtl: false,
    },
    LocaleInfo {
        tag: "fr",
        native_name: "Français",
        english_name: "French",
        rtl: false,
    },
    LocaleInfo {
        tag: "de",
        native_name: "Deutsch",
        english_name: "German",
        rtl: false,
    },
    LocaleInfo {
        tag: "ru",
        native_name: "Русский",
        english_name: "Russian",
        rtl: false,
    },
    LocaleInfo {
        tag: "fa",
        native_name: "فارسی",
        english_name: "Persian",
        rtl: true,
    },
    LocaleInfo {
        tag: "ar",
        native_name: "العربية",
        english_name: "Arabic",
        rtl: true,
    },
    LocaleInfo {
        tag: "hi",
        native_name: "हिन्दी",
        english_name: "Hindi",
        rtl: false,
    },
    LocaleInfo {
        tag: "it",
        native_name: "Italiano",
        english_name: "Italian",
        rtl: false,
    },
    LocaleInfo {
        tag: "tr",
        native_name: "Türkçe",
        english_name: "Turkish",
        rtl: false,
    },
];

#[derive(Debug, Clone, Serialize)]
pub struct LocaleBundle {
    pub lang: String,
    pub dir: &'static str,
    pub native_name: &'static str,
    pub english_name: &'static str,
    pub rtl: bool,
    pub messages: HashMap<String, String>,
    pub locales: &'static [LocaleInfo],
}

pub fn locale_info(tag: &str) -> Option<&'static LocaleInfo> {
    SUPPORTED_LOCALES
        .iter()
        .find(|info| info.tag.eq_ignore_ascii_case(tag))
}

pub fn lookup(id: &str) -> String {
    lookup_lang(&current_lang(), id)
}

#[cfg(test)]
pub(crate) fn lookup_for_tag(tag: &str, id: &str) -> String {
    let lang: LanguageIdentifier = tag.parse().expect("valid BCP-47 tag");
    lookup_lang(&lang, id)
}

#[cfg(test)]
pub(crate) fn lookup_for_tag_with_args(
    tag: &str,
    id: &str,
    args: &HashMap<String, String>,
) -> String {
    let lang: LanguageIdentifier = tag.parse().expect("valid BCP-47 tag");
    lookup_lang_with_args(&lang, id, args)
}

pub fn lookup_with_args(id: &str, args: &HashMap<String, String>) -> String {
    lookup_lang_with_args(&current_lang(), id, args)
}

pub fn format_message(id: &str, args: &HashMap<String, String>) -> String {
    if args.is_empty() {
        lookup(id)
    } else {
        lookup_with_args(id, args)
    }
}

pub fn locale_bundle() -> LocaleBundle {
    let lang = current_lang();
    let tag = lang.to_string();
    let info = locale_info(&tag).unwrap_or(&SUPPORTED_LOCALES[0]);
    let mut messages = HashMap::new();
    for id in message_ids() {
        messages.insert(id.to_string(), lookup_lang(&lang, id));
    }
    LocaleBundle {
        lang: tag,
        dir: if info.rtl { "rtl" } else { "ltr" },
        native_name: info.native_name,
        english_name: info.english_name,
        rtl: info.rtl,
        messages,
        locales: SUPPORTED_LOCALES,
    }
}

fn lookup_lang(lang: &LanguageIdentifier, id: &str) -> String {
    if let Some(value) = try_lookup_lang(lang, id) {
        return value;
    }
    let args = placeholder_args_for(id);
    if args.is_empty() {
        return id.to_string();
    }
    lookup_lang_with_args(lang, id, &args)
}

fn lookup_lang_with_args(
    lang: &LanguageIdentifier,
    id: &str,
    args: &HashMap<String, String>,
) -> String {
    let fluent_args: HashMap<Cow<'static, str>, FluentValue<'_>> = args
        .iter()
        .map(|(key, value)| (Cow::Owned(key.clone()), FluentValue::from(value.as_str())))
        .collect();
    LOCALES
        .try_lookup_with_args(lang, id, &fluent_args)
        .or_else(|| {
            LOCALES.try_lookup_with_args(&"en-US".parse().expect("en-US"), id, &fluent_args)
        })
        .unwrap_or_else(|| id.to_string())
}

fn try_lookup_lang(lang: &LanguageIdentifier, id: &str) -> Option<String> {
    LOCALES
        .try_lookup(lang, id)
        .or_else(|| LOCALES.try_lookup(&"en-US".parse().expect("en-US"), id))
}

const ENGLISH_CATALOG: &str = concat!(
    include_str!("../../../../locales/en-US/gui.ftl"),
    "\n",
    include_str!("../../../../locales/en-US/cli.ftl"),
    "\n",
    include_str!("../../../../locales/en-US/doctor.ftl"),
    "\n",
    include_str!("../../../../locales/en-US/errors.ftl"),
    "\n",
    include_str!("../../../../locales/en-US/install.ftl"),
);

fn message_ids() -> Vec<&'static str> {
    parse_ftl_ids(ENGLISH_CATALOG)
}

fn placeholder_args_for(id: &str) -> HashMap<String, String> {
    let Some(template) = english_template(id) else {
        return HashMap::new();
    };
    placeholder_args_from_template(&template)
}

fn english_template(id: &str) -> Option<String> {
    static MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
    MAP.get_or_init(|| parse_ftl_messages(ENGLISH_CATALOG))
        .get(id)
        .cloned()
}

fn placeholder_args_from_template(template: &str) -> HashMap<String, String> {
    let mut args = HashMap::new();
    let mut rest = template;
    while let Some(start) = rest.find("{ $") {
        let after = &rest[start + 3..];
        let Some(end) = after.find('}') else {
            break;
        };
        let name = after[..end].trim();
        if !name.is_empty() {
            args.insert(name.to_string(), format!("{{ ${name} }}"));
        }
        rest = &after[end + 1..];
    }
    args
}

fn parse_ftl_messages(source: &str) -> HashMap<String, String> {
    let mut messages = HashMap::new();
    let mut current_id: Option<String> = None;
    let mut current_val = String::new();
    for line in source.lines() {
        if current_id.is_some() && (line.starts_with(' ') || line.starts_with('\t')) {
            let part = line.trim();
            if !part.is_empty() {
                if !current_val.is_empty() {
                    current_val.push('\n');
                }
                current_val.push_str(part);
            }
            continue;
        }
        if let Some(id) = current_id.take() {
            messages.insert(id, std::mem::take(&mut current_val));
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('.') {
            continue;
        }
        let Some(idx) = trimmed.find('=') else {
            continue;
        };
        let id = trimmed[..idx].trim();
        if id.is_empty()
            || !id
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        {
            continue;
        }
        let value = trimmed[idx + 1..].trim();
        if value.is_empty() {
            current_id = Some(id.to_string());
            current_val.clear();
        } else {
            messages.insert(id.to_string(), value.to_string());
        }
    }
    if let Some(id) = current_id {
        messages.insert(id, current_val);
    }
    messages
}

fn parse_ftl_ids(source: &'static str) -> Vec<&'static str> {
    let mut ids = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('.') {
            continue;
        }
        if let Some(idx) = trimmed.find('=') {
            let id = trimmed[..idx].trim();
            if !id.is_empty()
                && id
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
            {
                ids.push(id);
            }
        }
    }
    ids
}
