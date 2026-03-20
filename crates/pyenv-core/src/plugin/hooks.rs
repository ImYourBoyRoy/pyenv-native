// ./crates/pyenv-core/src/plugin/hooks.rs
//! Hook execution plumbing and hook-action parsing for pyenv-compatible plugins.

use std::path::PathBuf;

use crate::context::AppContext;
use crate::error::PyenvError;

use super::discovery::discover_hook_scripts;
use super::process::run_process;
use super::types::{HookActions, HookResult};

pub const DEFAULT_HOOK_COMMANDS: &[&str] = &[
    "exec",
    "install",
    "rehash",
    "uninstall",
    "version-name",
    "version-origin",
    "which",
];

pub fn run_hook_scripts(
    ctx: &AppContext,
    hook: &str,
    extra_env: &[(&str, String)],
) -> Result<Vec<HookResult>, PyenvError> {
    let mut results = Vec::new();

    for script in discover_hook_scripts(ctx, hook)? {
        let mut env_pairs = vec![
            ("PYENV_HOOK", hook.to_string()),
            ("PYENV_COMMAND", hook.to_string()),
        ];
        env_pairs.extend(extra_env.iter().map(|(key, value)| (*key, value.clone())));

        let (exit_code, stdout, stderr) = run_process(&script, &[], ctx, &env_pairs, true)?;
        if exit_code != 0 {
            let detail = if !stderr.is_empty() {
                stderr.join("\n")
            } else {
                format!("exit code {exit_code}")
            };
            return Err(PyenvError::Io(format!(
                "pyenv: hook `{}` failed for {}: {detail}",
                script.display(),
                hook
            )));
        }

        results.push(HookResult {
            path: script,
            stdout,
        });
    }

    Ok(results)
}

pub fn parse_hook_actions(lines: &[String]) -> super::types::HookActions {
    let mut actions = HookActions::default();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("PYENV_COMMAND_PATH=") {
            if !value.trim().is_empty() {
                actions.command_path = Some(PathBuf::from(value.trim()));
            }
            continue;
        }

        if let Some(value) = trimmed
            .strip_prefix("PATH+=")
            .or_else(|| trimmed.strip_prefix("PYENV_PATH+="))
        {
            if !value.trim().is_empty() {
                actions.prepend_paths.push(PathBuf::from(value.trim()));
            }
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("ENV:")
            && let Some((key, rest)) = value.split_once('=')
        {
            let key = key.trim();
            if !key.is_empty() {
                actions
                    .env_pairs
                    .push((key.to_string(), rest.trim().to_string()));
                continue;
            }
        }

        if let Some((key, value)) = parse_shell_assignment(trimmed) {
            if key.eq_ignore_ascii_case("PYENV_COMMAND_PATH") {
                if !value.trim().is_empty() {
                    actions.command_path = Some(PathBuf::from(value.trim()));
                }
                continue;
            }

            if key.eq_ignore_ascii_case("PATH") {
                actions.env_pairs.push((key.to_string(), value.to_string()));
                continue;
            }

            if key.starts_with("PYENV_") {
                actions.env_pairs.push((key.to_string(), value.to_string()));
                continue;
            }
        }

        actions.passthrough_lines.push(trimmed.to_string());
    }

    actions
}

fn parse_shell_assignment(line: &str) -> Option<(&str, &str)> {
    let candidate = line
        .strip_prefix("export ")
        .or_else(|| line.strip_prefix("setenv "))
        .unwrap_or(line)
        .trim();
    let (key, raw_value) = candidate.split_once('=')?;
    let key = key.trim();
    if key.is_empty()
        || !key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    Some((key, strip_assignment_quotes(raw_value.trim())))
}

fn strip_assignment_quotes(value: &str) -> &str {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let first = bytes.first().copied();
        let last = bytes.last().copied();
        if matches!(
            (first, last),
            (Some(b'"'), Some(b'"')) | (Some(b'\''), Some(b'\''))
        ) {
            return &value[1..value.len() - 1];
        }
    }
    value
}
