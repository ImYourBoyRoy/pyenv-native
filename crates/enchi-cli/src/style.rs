// ./crates/enchi-cli/src/style.rs
//! Terminal styling for human-readable output.
//!
//! Colour is added only when the stream is an interactive terminal, `NO_COLOR` is unset and the
//! terminal is not `dumb`. Pipes, redirects, `--json` documents and anything a script captures are
//! never touched, so machine consumers always see exactly the plain text the core produced.

use std::io::IsTerminal;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";

/// Whether styled output is appropriate for a stream, given its terminal-ness and the environment.
fn wants_color(is_terminal: bool) -> bool {
    if !is_terminal {
        return false;
    }
    if std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty()) {
        return false;
    }
    if std::env::var("CLICOLOR").is_ok_and(|value| value == "0") {
        return false;
    }
    if std::env::var("TERM").is_ok_and(|value| value == "dumb") {
        return false;
    }
    if cfg!(windows) {
        // Legacy consoles print raw escape codes; Windows Terminal and friends advertise themselves.
        return ["WT_SESSION", "TERM", "ANSICON", "ConEmuANSI"]
            .iter()
            .any(|name| std::env::var_os(name).is_some());
    }
    true
}

/// Colour decisions for one run: one switch per output stream.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Style {
    stdout: bool,
    stderr: bool,
}

impl Style {
    pub(crate) fn detect() -> Self {
        Self {
            stdout: wants_color(std::io::stdout().is_terminal()),
            stderr: wants_color(std::io::stderr().is_terminal()),
        }
    }

    #[cfg(test)]
    fn forced(stdout: bool, stderr: bool) -> Self {
        Self { stdout, stderr }
    }

    pub(crate) fn stdout_line(&self, line: &str) -> String {
        if self.stdout {
            paint_stdout(line)
        } else {
            line.to_string()
        }
    }

    pub(crate) fn stderr_line(&self, line: &str) -> String {
        if self.stderr {
            paint_stderr(line)
        } else {
            line.to_string()
        }
    }
}

fn wrap(code: &str, text: &str) -> String {
    format!("{code}{text}{RESET}")
}

/// Status markers (`doctor`), the current-version star (`versions`) and dry-run narration.
fn paint_stdout(line: &str) -> String {
    for (marker, code) in [
        ("[OK]", GREEN),
        ("[WARN]", YELLOW),
        ("[FAIL]", RED),
        ("[ERROR]", RED),
        ("[INFO]", CYAN),
    ] {
        if let Some(rest) = line.strip_prefix(marker)
            && rest.starts_with(' ')
        {
            return format!("{}{rest}", wrap(code, marker));
        }
    }
    if line.starts_with("* ") {
        return format!("{BOLD}{GREEN}{line}{RESET}");
    }
    if line.starts_with("Would ") {
        return wrap(CYAN, line);
    }
    if line.starts_with("Dry run:") {
        return wrap(DIM, line);
    }
    line.to_string()
}

/// Errors in red, hints and warnings softened, so the cause stands out from the advice.
fn paint_stderr(line: &str) -> String {
    if let Some(rest) = line.strip_prefix("hint:") {
        return format!("{}{rest}", wrap(CYAN, "hint:"));
    }
    if let Some(rest) = line.strip_prefix("Hint:") {
        return format!("{}{rest}", wrap(CYAN, "Hint:"));
    }
    if line.starts_with("Warning:") || line.starts_with("warning:") {
        return wrap(YELLOW, line);
    }
    if let Some(rest) = line.strip_prefix("enchi:") {
        return format!("{BOLD}{RED}enchi:{RESET}{rest}");
    }
    line.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_passes_through_when_color_is_off() {
        let style = Style::forced(false, false);
        for line in [
            "[OK] root-directory: fine",
            "* 3.15.2",
            "enchi: boom",
            "Would remove x",
        ] {
            assert_eq!(style.stdout_line(line), line);
            assert_eq!(style.stderr_line(line), line);
        }
    }

    #[test]
    fn status_markers_are_coloured_and_the_message_is_not() {
        let style = Style::forced(true, true);
        let ok = style.stdout_line("[OK] shims-on-path: yes");
        assert!(ok.starts_with("\x1b[32m[OK]\x1b[0m"), "{ok:?}");
        assert!(ok.ends_with(" shims-on-path: yes"));
        assert!(style.stdout_line("[FAIL] x").starts_with("\x1b[31m[FAIL]"));
        assert!(style.stdout_line("[WARN] x").starts_with("\x1b[33m[WARN]"));
    }

    #[test]
    fn only_real_markers_are_styled() {
        let style = Style::forced(true, true);
        for line in [
            "[OKAY] no",
            "[OK]no",
            "eval \"$(enchi init -)\"",
            "path=/usr/bin",
            "",
        ] {
            assert_eq!(style.stdout_line(line), line, "{line:?}");
        }
    }

    #[test]
    fn errors_hints_and_warnings_read_differently() {
        let style = Style::forced(true, true);
        assert!(style.stderr_line("enchi: boom").contains("\x1b[31menchi:"));
        assert!(
            style
                .stderr_line("hint: try x")
                .starts_with("\x1b[36mhint:")
        );
        assert!(style.stderr_line("Warning: odd").starts_with("\x1b[33m"));
        assert_eq!(style.stderr_line("plain"), "plain");
    }
}
