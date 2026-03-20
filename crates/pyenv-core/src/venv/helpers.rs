// ./crates/pyenv-core/src/venv/helpers.rs
//! Shared helper functions for managed-venv path validation, prompt handling, interpreter
//! lookup, and JSON rendering.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::command::CommandReport;
use crate::error::PyenvError;
use crate::runtime::find_command_in_prefix;

use super::types::ManagedVenvInfo;

pub(super) fn interpreter_for_prefix(prefix: &Path) -> Option<PathBuf> {
    for command in ["python", "python3", "pypy3"] {
        if let Some(path) = find_command_in_prefix(prefix, command, None) {
            return Some(path);
        }
    }
    None
}

pub(super) fn pip_for_prefix(prefix: &Path) -> Option<PathBuf> {
    for command in ["pip", "pip3"] {
        if let Some(path) = find_command_in_prefix(prefix, command, None) {
            return Some(path);
        }
    }
    None
}

pub(super) fn is_safe_env_name(value: &str) -> bool {
    !value.trim().is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
}

pub(super) fn format_collision_error(name: &str, collisions: &[ManagedVenvInfo]) -> String {
    format!(
        "pyenv: managed venv name `{name}` already exists; use a unique name or reference one of: {}",
        collisions
            .iter()
            .map(|info| format!("`{}`", info.spec))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(super) fn confirm_action(prompt: &str) -> bool {
    let _ = write!(io::stdout(), "{prompt}");
    let _ = io::stdout().flush();

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes"),
        Err(_) => false,
    }
}

pub(super) fn io_error(error: std::io::Error) -> PyenvError {
    PyenvError::Io(format!("pyenv: {error}"))
}

pub(super) fn json_success<T: Serialize>(value: &T) -> CommandReport {
    match serde_json::to_string_pretty(value) {
        Ok(payload) => CommandReport::success(payload.lines().map(ToOwned::to_owned).collect()),
        Err(error) => CommandReport::failure(
            vec![format!("pyenv: failed to serialize JSON output: {error}")],
            1,
        ),
    }
}
