// ./crates/pyenv-core/src/doctor/helpers.rs
//! Shared PATH, shell, and platform helpers for doctor diagnostics and fixes.

use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::context::AppContext;
use crate::runtime::is_windows_apps_dir;

pub(super) fn shell_init_hint(ctx: &AppContext, platform: &str) -> String {
    match platform {
        "windows" => match ctx.env_shell.as_deref() {
            Some("cmd") => {
                "Add `for /f \"delims=\" %i in ('pyenv init - cmd') do %i` to your shell startup or rerun the Windows installer".to_string()
            }
            _ => "Add `$__pyenv_init = (pyenv init - pwsh) -join \"`n\"; if ($__pyenv_init) { Invoke-Expression $__pyenv_init }` to your PowerShell profile or rerun the Windows installer".to_string(),
        },
        _ => match ctx.env_shell.as_deref() {
            Some("zsh") => "Add `eval \"$(pyenv init - zsh)\"` to ~/.zshrc".to_string(),
            Some("fish") => "Add `pyenv init - fish | source` to your Fish config".to_string(),
            Some("sh") => "Add `eval \"$(pyenv init - sh)\"` to your shell profile".to_string(),
            _ => "Add `eval \"$(pyenv init - bash)\"` to ~/.bashrc (or the equivalent profile for your shell)".to_string(),
        },
    }
}

pub(super) fn user_shell_profiles_configured() -> bool {
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from);
    let Some(home) = home else {
        return false;
    };
    let candidates = [
        home.join(".bashrc"),
        home.join(".bash_profile"),
        home.join(".profile"),
        home.join(".zshrc"),
        home.join(".zprofile"),
        home.join(".config").join("fish").join("config.fish"),
        home.join(".config")
            .join("powershell")
            .join("Microsoft.PowerShell_profile.ps1"),
        home.join("Documents")
            .join("PowerShell")
            .join("Microsoft.PowerShell_profile.ps1"),
        home.join("Documents")
            .join("WindowsPowerShell")
            .join("Microsoft.PowerShell_profile.ps1"),
    ];
    candidates.into_iter().any(|path| {
        path.is_file()
            && std::fs::read_to_string(&path)
                .map(|content| {
                    let lower = content.to_ascii_lowercase();
                    lower.contains("pyenv init")
                        || lower.contains("pyenv-native")
                        || (lower.contains("pyenv_root") && lower.contains("shims"))
                })
                .unwrap_or(false)
    })
}

pub(super) fn is_termux_environment() -> bool {
    crate::preflight::is_termux_environment()
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

/// Locate a Microsoft Store `python.exe` App Execution Alias on PATH.
///
/// Lookup helpers skip this stub so `system` does not resolve to it. Doctor still
/// reports it when a bare `python` in the user's shell could hit it first.
pub(super) fn find_windows_store_python_alias(ctx: &AppContext) -> Option<PathBuf> {
    let entries = ctx
        .path_env
        .as_ref()
        .map(env::split_paths)
        .into_iter()
        .flatten()
        .filter(|entry| !entry.as_os_str().is_empty())
        .collect::<Vec<_>>();
    let names = [
        "python.exe",
        "python3.exe",
        "pythonw.exe",
        "python",
        "python3",
    ];
    for entry in entries {
        if !is_windows_apps_dir(&entry) {
            continue;
        }
        for name in names {
            let candidate = entry.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub(super) fn windows_store_alias_precedes_shims(ctx: &AppContext) -> bool {
    let shims_dir = ctx.shims_dir();
    let entries = ctx
        .path_env
        .as_ref()
        .map(env::split_paths)
        .into_iter()
        .flatten()
        .filter(|entry| !entry.as_os_str().is_empty())
        .collect::<Vec<_>>();
    let shims_pos = entries
        .iter()
        .position(|entry| paths_equal(entry, &shims_dir));
    let alias_pos = entries.iter().position(|entry| is_windows_apps_dir(entry));
    match (shims_pos, alias_pos) {
        (_, None) => false,
        (None, Some(_)) => true,
        (Some(shims), Some(alias)) => alias < shims,
    }
}

pub(super) fn windows_store_alias_needs_manual_fix(ctx: &AppContext) -> bool {
    find_windows_store_python_alias(ctx).is_some() && windows_store_alias_precedes_shims(ctx)
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
