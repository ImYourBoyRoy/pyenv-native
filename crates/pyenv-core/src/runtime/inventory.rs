// ./crates/pyenv-core/src/runtime/inventory.rs
//! Managed runtime and venv inventory helpers plus shim-name normalization logic.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::context::AppContext;
use crate::error::PyenvError;
use crate::venv_paths::{managed_venv_dir_from_spec, managed_venv_entries_for_base};
use crate::version::installed_version_dir;

use super::search::{executable_extensions, prefix_bin_dirs};

pub const BASE_VENV_DIR_NAME: &str = ".pyenv-base-venv";

pub fn managed_search_roots_for_version(ctx: &AppContext, version: &str) -> Vec<PathBuf> {
    if version == "system" {
        return Vec::new();
    }

    if let Some(venv_dir) = managed_venv_dir_from_spec(ctx, version)
        && venv_dir.is_dir()
    {
        return vec![venv_dir];
    }

    let version_dir = installed_version_dir(ctx, version);
    let mut roots = Vec::new();

    if ctx.config.venv.auto_use_base_venv {
        let base_venv = version_dir.join(BASE_VENV_DIR_NAME);
        if base_venv.is_dir() {
            roots.push(base_venv);
        }
    }

    roots.push(version_dir);
    roots
}

pub fn inventory_roots_for_version(ctx: &AppContext, version: &str) -> Vec<PathBuf> {
    let mut roots = managed_search_roots_for_version(ctx, version);
    if version.contains("/envs/") || version.contains("\\envs\\") {
        return roots;
    }

    roots.extend(
        managed_venv_entries_for_base(ctx, version)
            .unwrap_or_default()
            .into_iter()
            .map(|(_, path)| path),
    );

    roots
}

pub fn collect_shim_names_from_prefix(
    prefix: &Path,
    path_ext: Option<&OsStr>,
) -> Result<Vec<String>, PyenvError> {
    let mut names = HashSet::new();

    for directory in prefix_bin_dirs(prefix) {
        if !directory.is_dir() {
            continue;
        }

        for entry in fs::read_dir(&directory).map_err(io_error)? {
            let entry = entry.map_err(io_error)?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if let Some(name) = normalize_shim_name(&path, path_ext) {
                names.insert(name);
            }
        }
    }

    let mut values = names.into_iter().collect::<Vec<_>>();
    values.sort_by_key(|value| value.to_ascii_lowercase());
    Ok(values)
}

pub fn normalize_shim_name(path: &Path, path_ext: Option<&OsStr>) -> Option<String> {
    let file_name = path.file_name()?.to_string_lossy().to_string();
    let stem = path.file_stem()?.to_string_lossy().to_string();
    let extension = path
        .extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase());

    match extension.as_deref() {
        Some(ext)
            if executable_extensions(path_ext)
                .iter()
                .any(|candidate| candidate.trim_start_matches('.').eq_ignore_ascii_case(ext)) =>
        {
            Some(stem)
        }
        None => Some(file_name),
        _ => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                let metadata = fs::metadata(path).ok()?;
                if metadata.permissions().mode() & 0o111 != 0 {
                    return Some(file_name);
                }
            }

            None
        }
    }
}

fn io_error(error: std::io::Error) -> PyenvError {
    PyenvError::Io(format!("pyenv: {error}"))
}
