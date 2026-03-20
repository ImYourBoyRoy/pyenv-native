// ./crates/pyenv-core/src/shell/emit/quotes.rs
//! Shared quoting helpers for shell-specific code emitters.

pub(crate) fn ps_single_quote(value: &str) -> String {
    value.replace('\'', "''")
}

pub(crate) fn ps_double_quote(value: &str) -> String {
    value.replace('`', "``").replace('"', "`\"")
}

pub(crate) fn sh_single_quote(value: &str) -> String {
    value.replace('\'', "'\\''")
}

pub(crate) fn fish_single_quote(value: &str) -> String {
    value.replace('\'', "\\'")
}

pub(crate) fn cmd_quote(value: &str) -> String {
    if value.is_empty()
        || value
            .chars()
            .any(|ch| ch.is_whitespace() || matches!(ch, '"' | '&' | '|' | '<' | '>' | '^'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
