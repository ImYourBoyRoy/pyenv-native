// ./crates/pyenv-core/src/i18n/tests.rs
//! Locale negotiation and catalog lookup coverage.

#[cfg(test)]
mod tests {
    use super::super::catalog::{lookup_for_tag, lookup_for_tag_with_args};
    use super::super::negotiate::negotiate;
    use std::collections::HashMap;

    #[test]
    fn zh_and_zh_tw_negotiate_to_simplified_chinese() {
        assert_eq!(negotiate(&["zh".into()]).to_string(), "zh-CN");
        assert_eq!(negotiate(&["zh-TW".into()]).to_string(), "zh-CN");
        assert_eq!(negotiate(&["zh-Hant-TW".into()]).to_string(), "zh-CN");
        assert_eq!(negotiate(&["pt".into()]).to_string(), "pt-BR");
        assert_eq!(negotiate(&["en-GB".into()]).to_string(), "en-US");
        assert_eq!(negotiate(&["xx-YY".into()]).to_string(), "en-US");
    }

    #[test]
    fn english_error_catalog_matches_legacy_copy() {
        assert_eq!(
            lookup_for_tag("en-US", "error-missing-home"),
            "pyenv: cannot determine home directory for PYENV_ROOT"
        );
        let mut args = HashMap::new();
        args.insert("key".into(), "ui.language".into());
        assert!(
            lookup_for_tag_with_args("en-US", "error-unknown-config-key", &args)
                .contains("ui.language")
        );
    }

    #[test]
    fn simplified_chinese_catalog_is_used_for_gui_chrome() {
        assert_eq!(lookup_for_tag("zh-CN", "gui-nav-settings"), "设置");
        assert_eq!(lookup_for_tag("zh-CN", "gui-language-auto"), "跟随系统");
        assert_eq!(
            lookup_for_tag("zh-CN", "gui-failed-load-installed"),
            "无法加载已安装的运行时。"
        );
    }

    #[test]
    fn leftover_gui_chrome_is_localized() {
        let origin = lookup_for_tag("zh-CN", "gui-origin-global-line");
        assert!(origin.contains("来源"), "got {origin:?}");
        assert!(
            !lookup_for_tag("zh-CN", "gui-uptodate-check").contains("Up to Date"),
            "footer up-to-date copy must not stay English in zh-CN"
        );
        let mut version = HashMap::new();
        version.insert("version".into(), "v0.3.1".into());
        let up_to_date = lookup_for_tag_with_args("zh-CN", "gui-up-to-date-body", &version);
        assert!(up_to_date.contains("v0.3.1"), "got {up_to_date:?}");
        assert!(
            !up_to_date.contains("is up to date"),
            "update dialog body must not stay English in zh-CN: {up_to_date:?}"
        );
        let runtime = lookup_for_tag("zh-CN", "gui-runtime-target");
        assert!(!runtime.starts_with("Runtime:"), "got {runtime:?}");
        let mut args = HashMap::new();
        args.insert("expected".into(), "/home/user/.pyenv/bin/pyenv".into());
        args.insert("current".into(), "/tmp/pyenv-gui".into());
        let portable = lookup_for_tag_with_args("zh-CN", "error-self-update-portable", &args);
        assert!(portable.contains("/tmp/pyenv-gui"), "got {portable:?}");
        assert!(!portable.starts_with("pyenv: self-update only supports"));
    }

    #[test]
    fn missing_id_returns_the_id_not_fluent_unknown_key() {
        assert_eq!(
            lookup_for_tag("en-US", "cli-pyenv-about"),
            "cli-pyenv-about"
        );
        let about = lookup_for_tag("zh-CN", "cli-about");
        assert!(!about.contains("Unknown localization key"));
        assert_eq!(about, "原生优先、跨平台的 Python 版本管理器");
    }
}
