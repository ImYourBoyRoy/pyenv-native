// ./crates/pyenv-core/src/runtime/search.rs
//! Path-search helpers for runtime prefixes, executable name expansion, and Windows trap avoidance.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub fn prefix_bin_dirs(prefix: &Path) -> Vec<PathBuf> {
    vec![
        prefix.to_path_buf(),
        prefix.join("Scripts"),
        prefix.join("bin"),
    ]
}

pub fn find_command_in_prefix(
    prefix: &Path,
    command: &str,
    path_ext: Option<&OsStr>,
) -> Option<PathBuf> {
    search_path_entries(&prefix_bin_dirs(prefix), command, path_ext)
}

pub fn search_path_entries(
    directories: &[PathBuf],
    command: &str,
    path_ext: Option<&OsStr>,
) -> Option<PathBuf> {
    for directory in directories {
        if !directory.is_dir() {
            continue;
        }

        #[cfg(windows)]
        {
            let lowered = directory.to_string_lossy().to_ascii_lowercase();
            if lowered.contains("windowsapps\\python") || lowered.contains("windowsapps/python") {
                continue;
            }
        }

        for candidate in candidate_file_names(command, path_ext) {
            let path = directory.join(&candidate);
            if path.is_file() {
                return Some(path);
            }
        }
    }

    None
}

pub fn candidate_file_names(command: &str, path_ext: Option<&OsStr>) -> Vec<String> {
    let command_path = Path::new(command);
    if command_path.extension().is_some() {
        return vec![command.to_string()];
    }

    let mut names = vec![command.to_string()];
    let mut seen = HashSet::new();
    seen.insert(command.to_ascii_lowercase());

    let path_exts = executable_extensions(path_ext);
    for extension in path_exts {
        let candidate = format!("{command}{extension}");
        if seen.insert(candidate.to_ascii_lowercase()) {
            names.push(candidate);
        }
    }

    names
}

pub fn executable_extensions(path_ext: Option<&OsStr>) -> Vec<String> {
    path_ext
        .and_then(OsStr::to_str)
        .map(|value| {
            value
                .split(';')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|extensions| !extensions.is_empty())
        .unwrap_or_else(|| {
            vec![
                ".EXE".to_string(),
                ".BAT".to_string(),
                ".CMD".to_string(),
                ".COM".to_string(),
                ".PS1".to_string(),
            ]
        })
}
