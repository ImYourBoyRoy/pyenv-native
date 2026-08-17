// ./crates/pyenv-core/src/i18n/mod.rs
//! Shared Fluent localization: locale negotiation, catalog lookup, and GUI bundles.

mod catalog;
mod negotiate;
mod tests;

pub use catalog::{
    LocaleBundle, LocaleInfo, SUPPORTED_LOCALES, format_message, locale_bundle, locale_info,
    lookup, lookup_with_args,
};
pub use negotiate::{apply_config_language, current_lang_tag, init, negotiate, set_lang_tag};

use crate::config::AppConfig;

/// Apply env/config/OS language to the process-wide catalog.
pub fn apply(config: &AppConfig) {
    init();
    apply_config_language(&config.ui.language);
}
