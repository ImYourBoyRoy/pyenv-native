// ./crates/pyenv-core/src/meta/help.rs
//! Help rendering for built-in commands and plugin-provided external commands with lightweight
//! header parsing.

use std::fs;
use std::path::Path;

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::plugin::find_plugin_command;

use super::docs::command_doc;
use super::public_docs::PUBLIC_COMMAND_DOCS;

pub fn cmd_help(ctx: &AppContext, command: Option<&str>, usage_only: bool) -> CommandReport {
    let target = command
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "pyenv");

    let Some(command_name) = target else {
        return render_top_level_help(usage_only);
    };

    if let Some(doc) = command_doc(command_name) {
        if usage_only {
            return CommandReport::success_one(doc.usage);
        }

        let mut stdout = vec![doc.usage.to_string()];
        if !doc.help.is_empty() {
            stdout.push(String::new());
            stdout.extend(doc.help.iter().map(|line| (*line).to_string()));
        }
        return CommandReport::success(stdout);
    }

    if let Some(command_path) = find_plugin_command(ctx, command_name) {
        let external_doc = parse_external_command_doc(&command_path, command_name);
        let usage = external_doc
            .as_ref()
            .map(|doc| doc.usage.clone())
            .unwrap_or_else(|| format!("Usage: pyenv {command_name} [<args>]"));
        if usage_only {
            return CommandReport::success_one(usage);
        }

        if let Some(doc) = external_doc {
            let mut stdout = vec![doc.usage];
            if !doc.help.is_empty() {
                stdout.push(String::new());
                stdout.extend(doc.help);
            }
            return CommandReport::success(stdout);
        }

        return CommandReport::success(vec![
            usage,
            String::new(),
            format!(
                "`{command_name}` is provided by an external pyenv plugin and does not expose built-in help text."
            ),
        ]);
    }

    CommandReport::failure(vec![format!("pyenv: no such command `{command_name}`")], 1)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExternalCommandDoc {
    usage: String,
    help: Vec<String>,
}

fn render_top_level_help(usage_only: bool) -> CommandReport {
    if usage_only {
        return CommandReport::success_one("Usage: pyenv <command> [<args>]");
    }

    let mut stdout = vec![
        "Usage: pyenv <command> [<args>]".to_string(),
        String::new(),
        "Some useful pyenv commands are:".to_string(),
    ];
    stdout.extend(
        PUBLIC_COMMAND_DOCS
            .iter()
            .map(|doc| format!("   {:<12} {}", doc.name, doc.summary)),
    );
    stdout.push(String::new());
    stdout.push("CORE CONCEPTS:".to_string());
    stdout.push("  Shims:       Lightweight executables (like `python` or `pip`) that intercept your commands".to_string());
    stdout.push("               and route them to the correct Python version based on your current environment.".to_string());
    stdout.push(
        "               Run `pyenv rehash` to refresh these after installing new pip packages."
            .to_string(),
    );
    stdout.push("  Versions:    Python environments installed via `pyenv install`. Located in `~/.pyenv/versions`.".to_string());
    stdout.push("  Managed envs: Named virtual environments can live under `~/.pyenv/venvs/<runtime>/<name>`.".to_string());
    stdout.push("               Use `pyenv venv create 3.13 api` and bind a folder with `pyenv venv use api`.".to_string());
    stdout.push("  Discovery:   Search installable runtimes with `pyenv install --list 3.13` or `pyenv available 3.13`.".to_string());
    stdout.push("  Selection:   Pyenv decides which Python version to use in this order (highest priority first):".to_string());
    stdout.push(
        "                 1. PYENV_VERSION environment variable (set via `pyenv shell`)"
            .to_string(),
    );
    stdout.push(
        "                 2. .python-version file in the current directory (set via `pyenv local`)"
            .to_string(),
    );
    stdout.push("                 3. The global version file (set via `pyenv global`)".to_string());
    stdout.push(String::new());
    stdout.push("See `pyenv help <command>` for information on a specific command.".to_string());
    stdout.push(
        "For full documentation, see: https://github.com/imyourboyroy/pyenv-native".to_string(),
    );
    CommandReport::success(stdout)
}

fn parse_external_command_doc(path: &Path, command_name: &str) -> Option<ExternalCommandDoc> {
    let content = fs::read(path).ok()?;
    let text = String::from_utf8_lossy(&content);
    let header = extract_external_doc_header(&text);
    if header.is_empty() {
        return None;
    }

    let usage = parse_header_usage(&header)
        .unwrap_or_else(|| format!("Usage: pyenv {command_name} [<args>]"));
    let summary = header
        .iter()
        .find_map(|line| {
            line.strip_prefix("Summary:")
                .map(str::trim)
                .map(ToOwned::to_owned)
        })
        .filter(|value| !value.is_empty());
    let help = parse_header_help(&header, summary.as_deref());

    Some(ExternalCommandDoc { usage, help })
}

fn extract_external_doc_header(text: &str) -> Vec<String> {
    let mut header = Vec::new();
    let mut collecting = false;

    for line in text.lines() {
        if !collecting && line.starts_with("#!") {
            continue;
        }

        if let Some(comment) = strip_doc_comment_prefix(line) {
            collecting = true;
            header.push(comment);
            continue;
        }

        if collecting && line.trim().is_empty() {
            header.push(String::new());
            continue;
        }

        if collecting {
            break;
        }
    }

    while header.first().is_some_and(|line| line.trim().is_empty()) {
        header.remove(0);
    }
    while header.last().is_some_and(|line| line.trim().is_empty()) {
        header.pop();
    }

    header
}

fn strip_doc_comment_prefix(line: &str) -> Option<String> {
    let trimmed_start = line.trim_start();
    if trimmed_start.starts_with('#') && !trimmed_start.starts_with("#!") {
        return Some(
            trimmed_start[1..]
                .strip_prefix(' ')
                .unwrap_or(&trimmed_start[1..])
                .to_string(),
        );
    }
    if let Some(rest) = trimmed_start.strip_prefix("::") {
        return Some(rest.strip_prefix(' ').unwrap_or(rest).to_string());
    }
    if trimmed_start
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("rem"))
    {
        let rest = &trimmed_start[3..];
        return Some(rest.strip_prefix(' ').unwrap_or(rest).to_string());
    }
    None
}

fn parse_header_usage(header: &[String]) -> Option<String> {
    let start = header
        .iter()
        .position(|line| line.trim_start().starts_with("Usage:"))?;
    let mut lines = Vec::new();
    lines.push(header[start].trim_start().to_string());

    for line in &header[start + 1..] {
        if line.starts_with("Summary:")
            || (!line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t'))
        {
            break;
        }
        lines.push(line.to_string());
    }

    Some(lines.join("\n"))
}

fn parse_header_help(header: &[String], summary: Option<&str>) -> Vec<String> {
    let Some(start) = header
        .iter()
        .position(|line| line.trim_start().starts_with("Summary:"))
    else {
        return summary
            .map(|value| vec![value.to_string()])
            .unwrap_or_default();
    };

    let mut help = header[start + 1..].to_vec();
    while help.first().is_some_and(|line| line.trim().is_empty()) {
        help.remove(0);
    }
    while help.last().is_some_and(|line| line.trim().is_empty()) {
        help.pop();
    }

    if help.iter().any(|line| !line.trim().is_empty()) {
        help
    } else {
        summary
            .map(|value| vec![value.to_string()])
            .unwrap_or_default()
    }
}
