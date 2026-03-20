// ./crates/pyenv-core/src/doctor/helpers.rs
//! Shared PATH, shell, and platform helpers for doctor diagnostics and fixes.

use std::env;
use std::ffi::OsStr;
use std::path::Path;

use crate::context::AppContext;

pub(super) fn shell_init_hint(ctx: &AppContext, platform: &str) -> String {
    match platform {
        "windows" => match ctx.env_shell.as_deref() {
            Some("cmd") => {
                "Add `for /f \"delims=\" %i in ('pyenv init - cmd') do %i` to your shell startup or rerun the Windows installer".to_string()
            }
            _ => "Add `iex ((pyenv init - pwsh) -join \"`n\")` to your PowerShell profile or rerun the Windows installer".to_string(),
        },
        _ => match ctx.env_shell.as_deref() {
            Some("zsh") => "Add `eval \"$(pyenv init - zsh)\"` to ~/.zshrc".to_string(),
            Some("fish") => "Add `pyenv init - fish | source` to your Fish config".to_string(),
            Some("sh") => "Add `eval \"$(pyenv init - sh)\"` to your shell profile".to_string(),
            _ => "Add `eval \"$(pyenv init - bash)\"` to ~/.bashrc (or the equivalent profile for your shell)".to_string(),
        },
    }
}

pub(super) fn is_termux_environment() -> bool {
    env::var_os("TERMUX_VERSION").is_some()
        || env::var_os("PREFIX")
            .map(|value| value.to_string_lossy().contains("/data/data/com.termux"))
            .unwrap_or(false)
}

pub(super) fn path_ext_for_platform<'a>(ctx: &'a AppContext, platform: &str) -> Option<&'a OsStr> {
    if platform == "windows" {
        ctx.path_ext.as_deref()
    } else {
        None
    }
}

pub(super) fn path_contains(path_env: Option<&std::ffi::OsString>, target: &Path) -> bool {
    path_env
        .map(env::split_paths)
        .into_iter()
        .flatten()
        .any(|entry| paths_equal(&entry, target))
}

pub(super) fn paths_equal(lhs: &Path, rhs: &Path) -> bool {
    if cfg!(windows) {
        lhs.to_string_lossy()
            .replace('/', "\\")
            .eq_ignore_ascii_case(&rhs.to_string_lossy().replace('/', "\\"))
    } else {
        lhs == rhs
    }
}
