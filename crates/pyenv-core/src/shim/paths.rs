// ./crates/pyenv-core/src/shim/paths.rs
//! PATH shaping and shim artifact path helpers.

use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::context::AppContext;

pub(super) fn adjusted_path(ctx: &AppContext, prefix_dirs: &[PathBuf]) -> Option<OsString> {
    let mut combined = Vec::new();
    let mut seen = HashSet::new();

    for path in prefix_dirs {
        if !path.as_os_str().is_empty()
            && !paths_equal(path, &ctx.shims_dir())
            && seen.insert(path_key(path))
        {
            combined.push(path.clone());
        }
    }

    for path in ctx
        .path_env
        .clone()
        .or_else(|| env::var_os("PATH"))
        .as_ref()
        .map(env::split_paths)
        .into_iter()
        .flatten()
    {
        if !path.as_os_str().is_empty()
            && !paths_equal(&path, &ctx.shims_dir())
            && seen.insert(path_key(&path))
        {
            combined.push(path);
        }
    }

    env::join_paths(combined).ok()
}

pub(super) fn remove_shim_artifacts(shims_dir: &Path, command: &str) {
    for path in [
        shim_native_path(shims_dir, command),
        shim_cmd_path(shims_dir, command),
        shim_bat_path(shims_dir, command),
        shim_ps1_path(shims_dir, command),
        shim_posix_path(shims_dir, command),
    ] {
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }
}

pub(super) fn shim_native_path(shims_dir: &Path, command: &str) -> PathBuf {
    if cfg!(windows) {
        shims_dir.join(format!("{command}.exe"))
    } else {
        shims_dir.join(command)
    }
}

pub(super) fn shim_cmd_path(shims_dir: &Path, command: &str) -> PathBuf {
    shims_dir.join(format!("{command}.cmd"))
}

pub(super) fn shim_ps1_path(shims_dir: &Path, command: &str) -> PathBuf {
    shims_dir.join(format!("{command}.ps1"))
}

pub(super) fn shim_bat_path(shims_dir: &Path, command: &str) -> PathBuf {
    shims_dir.join(format!("{command}.bat"))
}

pub(super) fn shim_posix_path(shims_dir: &Path, command: &str) -> PathBuf {
    shims_dir.join(command)
}

fn paths_equal(lhs: &Path, rhs: &Path) -> bool {
    if cfg!(windows) {
        lhs.to_string_lossy()
            .replace('/', "\\")
            .eq_ignore_ascii_case(&rhs.to_string_lossy().replace('/', "\\"))
    } else {
        lhs == rhs
    }
}

fn path_key(path: &Path) -> String {
    if cfg!(windows) {
        path.to_string_lossy()
            .replace('/', "\\")
            .to_ascii_lowercase()
    } else {
        path.to_string_lossy().to_string()
    }
}
