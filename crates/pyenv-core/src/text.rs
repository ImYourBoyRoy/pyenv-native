// ./crates/pyenv-core/src/text.rs
//! Cross-platform text normalization for config files, env vars, and shell profiles.

/// Strip a UTF-8 BOM when present.
pub fn strip_utf8_bom(text: &str) -> &str {
    text.strip_prefix('\u{FEFF}').unwrap_or(text)
}

/// Trim whitespace and stray carriage returns from environment values.
pub fn trim_env_value(value: String) -> String {
    value.trim().trim_end_matches('\r').to_string()
}

/// Return whether `text` primarily uses CRLF line endings.
pub fn uses_crlf(text: &str) -> bool {
    text.contains("\r\n")
}

/// Preferred line ending for appending to existing text.
pub fn line_ending_for(text: &str) -> &'static str {
    if uses_crlf(text) { "\r\n" } else { "\n" }
}

/// Normalize a block to the target line ending.
pub fn normalize_block_eol(block: &str, eol: &str) -> String {
    if eol == "\r\n" {
        block.replace("\r\n", "\n").replace('\n', "\r\n")
    } else {
        block.replace("\r\n", "\n")
    }
}

/// Append `block` to `content`, preserving the existing file's line-ending style.
pub fn append_text_block(content: String, block: &str) -> String {
    if content.contains("pyenv init") {
        return content;
    }

    let eol = line_ending_for(&content);
    let normalized_block = normalize_block_eol(block, eol);
    let mut updated = content;
    if !updated.is_empty() && !updated.ends_with('\n') && !updated.ends_with("\r\n") {
        updated.push_str(eol);
    }
    updated.push_str(&normalized_block);
    if !normalized_block.ends_with(eol) {
        updated.push_str(eol);
    }
    updated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_utf8_bom_removes_prefix() {
        let text = "\u{FEFF}hello";
        assert_eq!(strip_utf8_bom(text), "hello");
        assert_eq!(strip_utf8_bom("hello"), "hello");
    }

    #[test]
    fn trim_env_value_strips_carriage_return() {
        assert_eq!(trim_env_value("3.12.0\r\n".to_string()), "3.12.0");
        assert_eq!(trim_env_value("  bash \r".to_string()), "bash");
    }

    #[test]
    fn append_text_block_preserves_crlf() {
        let existing = "# profile\r\nexport FOO=1\r\n";
        let block = "\n# pyenv-native shell initialization\neval \"$(pyenv init - bash)\"\n";
        let updated = append_text_block(existing.to_string(), block);
        assert!(updated.contains("\r\n"));
        assert!(updated.contains("pyenv init - bash"));
    }

    #[test]
    fn append_text_block_skips_when_already_configured() {
        let existing = "eval \"$(pyenv init - bash)\"\n";
        let block = "\n# pyenv-native shell initialization\neval \"$(pyenv init - zsh)\"\n";
        assert_eq!(append_text_block(existing.to_string(), block), existing);
    }
}
