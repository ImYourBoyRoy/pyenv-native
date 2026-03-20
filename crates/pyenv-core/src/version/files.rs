// ./crates/pyenv-core/src/version/files.rs
//! Version-file discovery, parsing, validation, and persistence helpers for `.python-version`
//! and global version files.

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::context::AppContext;
use crate::error::PyenvError;

use super::types::{GLOBAL_VERSION_FILE, LOCAL_VERSION_FILE, ParsedVersionFile};

pub fn find_local_version_file(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();

    loop {
        let candidate = current.join(LOCAL_VERSION_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }

        if !current.pop() {
            return None;
        }
    }
}

pub fn version_file_path(ctx: &AppContext, target_dir: Option<&Path>) -> PathBuf {
    if let Some(target_dir) = target_dir {
        find_local_version_file(target_dir).unwrap_or_else(|| ctx.root.join(GLOBAL_VERSION_FILE))
    } else {
        find_local_version_file(&ctx.dir).unwrap_or_else(|| ctx.root.join(GLOBAL_VERSION_FILE))
    }
}

pub fn read_version_file(path: &Path) -> Result<Vec<String>, Vec<PyenvError>> {
    parse_version_file(path).map(|parsed| parsed.versions)
}

pub(super) fn parse_version_file(path: &Path) -> Result<ParsedVersionFile, Vec<PyenvError>> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(_) => return Err(Vec::new()),
    };

    let mut versions = Vec::new();
    let mut errors = Vec::new();

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some(version) = trimmed.split_whitespace().next() else {
            continue;
        };

        if is_version_safe(version) {
            versions.push(version.to_string());
        } else {
            errors.push(PyenvError::InvalidVersion(
                version.to_string(),
                path.display().to_string(),
            ));
        }
    }

    if versions.is_empty() {
        Err(errors)
    } else {
        Ok(ParsedVersionFile {
            versions,
            warnings: errors,
        })
    }
}

pub(super) fn write_version_file(path: &Path, versions: &[String]) -> Result<(), PyenvError> {
    if versions.is_empty() {
        return Err(PyenvError::Io("pyenv: no versions specified".to_string()));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| PyenvError::Io(format!("pyenv: {error}")))?;
    }

    let mut contents = versions.join("\n");
    contents.push('\n');
    fs::write(path, contents).map_err(|error| PyenvError::Io(format!("pyenv: {error}")))?;
    Ok(())
}

fn is_version_safe(version: &str) -> bool {
    let path = Path::new(version);
    if path.is_absolute() {
        return false;
    }

    let mut depth = 0usize;
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(_) => depth += 1,
            Component::ParentDir => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }

    true
}
