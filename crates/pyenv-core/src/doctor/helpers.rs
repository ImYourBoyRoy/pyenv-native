// ./crates/pyenv-core/src/doctor/helpers.rs
//! Shared PATH, shell, and platform helpers for doctor diagnostics and fixes.

use std::env;
use std::ffi::{OsStr, OsString};
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

fn path_entries(path_env: Option<&OsStr>) -> Vec<PathBuf> {
    path_env
        .map(env::split_paths)
        .into_iter()
        .flatten()
        .filter(|entry| !entry.as_os_str().is_empty())
        .collect()
}

/// PATH used to decide whether Store python stubs can intercept a shell.
///
/// Desktop GUI sessions prepend shims onto process PATH, which would hide a
/// terminal problem. Those sessions inspect the launch PATH instead.
pub(super) fn store_alias_path_env(ctx: &AppContext, desktop_session: bool) -> Option<OsString> {
    if desktop_session {
        Some(crate::launch_path())
    } else {
        ctx.path_env.clone()
    }
}

/// Locate a Microsoft Store `python.exe` App Execution Alias on PATH.
///
/// Lookup helpers skip this stub so `system` does not resolve to it. Doctor still
/// reports it when a bare `python` in the user's shell could hit it first.
pub(super) fn find_windows_store_python_alias_in(path_env: Option<&OsStr>) -> Option<PathBuf> {
    for entry in path_entries(path_env) {
        if !is_windows_apps_dir(&entry) {
            continue;
        }
        for name in WINDOWS_STORE_PYTHON_STUB_NAMES {
            let candidate = entry.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub(super) fn windows_store_alias_precedes_shims_in(
    shims_dir: &Path,
    path_env: Option<&OsStr>,
) -> bool {
    let entries = path_entries(path_env);
    let shims_pos = entries
        .iter()
        .position(|entry| paths_equal(entry, shims_dir));
    let alias_pos = entries.iter().position(|entry| is_windows_apps_dir(entry));
    match (shims_pos, alias_pos) {
        (_, None) => false,
        (None, Some(_)) => true,
        (Some(shims), Some(alias)) => alias < shims,
    }
}

pub(super) fn windows_store_alias_needs_fix_in(ctx: &AppContext, path_env: Option<&OsStr>) -> bool {
    find_windows_store_python_alias_in(path_env).is_some()
        && windows_store_alias_precedes_shims_in(&ctx.shims_dir(), path_env)
}

const WINDOWS_STORE_PYTHON_STUB_NAMES: &[&str] = &[
    "python.exe",
    "python3.exe",
    "pythonw.exe",
    "python",
    "python3",
];

pub(super) fn windows_apps_user_dir() -> Option<PathBuf> {
    env::var_os("LOCALAPPDATA")
        .map(|root| PathBuf::from(root).join("Microsoft").join("WindowsApps"))
}

pub(super) fn windows_store_python_stub_dirs(ctx: &AppContext) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = windows_apps_user_dir() {
        dirs.push(dir);
    }
    if let Some(path_env) = ctx.path_env.as_ref() {
        for entry in env::split_paths(path_env) {
            if entry.as_os_str().is_empty() || !is_windows_apps_dir(&entry) {
                continue;
            }
            if !dirs.iter().any(|existing| paths_equal(existing, &entry)) {
                dirs.push(entry);
            }
        }
    }
    dirs
}

/// Remove App Installer `python.exe` / `python3.exe` stubs from a WindowsApps directory.
///
/// Only those filenames are deleted. Other aliases such as `winget.exe` are left alone.
pub(super) fn remove_windows_store_python_stubs_in(dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !is_windows_apps_dir(dir) {
        return Err(format!(
            "refusing to remove python stubs outside a WindowsApps directory: {}",
            dir.display()
        ));
    }
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut removed = Vec::new();
    for name in WINDOWS_STORE_PYTHON_STUB_NAMES {
        let candidate = dir.join(name);
        if !candidate.is_file() {
            continue;
        }
        std::fs::remove_file(&candidate).map_err(|error| {
            format!(
                "failed to remove Store python alias {}: {error}",
                candidate.display()
            )
        })?;
        removed.push(candidate);
    }
    Ok(removed)
}

pub(super) fn remove_windows_store_python_alias_stubs(
    ctx: &AppContext,
) -> Result<Vec<PathBuf>, String> {
    let mut removed = Vec::new();
    let mut errors = Vec::new();
    for dir in windows_store_python_stub_dirs(ctx) {
        match remove_windows_store_python_stubs_in(&dir) {
            Ok(paths) => removed.extend(paths),
            Err(error) => errors.push(error),
        }
    }
    if removed.is_empty() && !errors.is_empty() {
        return Err(errors.join("; "));
    }
    Ok(removed)
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
