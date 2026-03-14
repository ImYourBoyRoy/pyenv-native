// ./crates/pyenv-core/src/runtime.rs
//! Managed runtime path helpers for executable lookup, prefix resolution, and shim inventory.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::context::AppContext;
use crate::error::PyenvError;
use crate::version::installed_version_dir;

pub const BASE_VENV_DIR_NAME: &str = ".pyenv-base-venv";

pub fn managed_search_roots_for_version(ctx: &AppContext, version: &str) -> Vec<PathBuf> {
    if version == "system" {
        return Vec::new();
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
    let version_dir = installed_version_dir(ctx, version);
    let envs_dir = version_dir.join("envs");

    if envs_dir.is_dir() {
        let mut env_roots = fs::read_dir(&envs_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect::<Vec<_>>();
        env_roots.sort_by_key(|path| {
            path.file_name()
                .map(|value| value.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default()
        });
        roots.extend(env_roots);
    }

    roots
}

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

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::{
        BASE_VENV_DIR_NAME, collect_shim_names_from_prefix, find_command_in_prefix,
        managed_search_roots_for_version, prefix_bin_dirs,
    };

    fn test_path_ext() -> Option<OsString> {
        if cfg!(windows) {
            Some(OsString::from(".exe;.bat;.cmd"))
        } else {
            None
        }
    }

    fn test_context() -> (TempDir, AppContext) {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        let dir = temp.path().join("work");
        fs::create_dir_all(root.join("versions")).expect("versions dir");
        fs::create_dir_all(&dir).expect("work dir");

        let ctx = AppContext {
            root,
            dir,
            exe_path: std::path::PathBuf::from("pyenv"),
            env_version: None,
            env_shell: None,
            path_env: None,
            path_ext: test_path_ext(),
            config: AppConfig::default(),
        };

        (temp, ctx)
    }

    #[test]
    fn search_roots_prefer_base_venv_when_enabled() {
        let (_temp, mut ctx) = test_context();
        ctx.config.venv.auto_use_base_venv = true;
        let version_dir = ctx.versions_dir().join("3.12.1");
        fs::create_dir_all(version_dir.join(BASE_VENV_DIR_NAME)).expect("base venv");

        let roots = managed_search_roots_for_version(&ctx, "3.12.1");
        assert_eq!(
            roots,
            vec![version_dir.join(BASE_VENV_DIR_NAME), version_dir,]
        );
    }

    #[test]
    fn find_command_checks_prefix_root_and_scripts() {
        let temp = TempDir::new().expect("tempdir");
        let prefix = temp.path().join("runtime");
        let python_path = if cfg!(windows) {
            fs::create_dir_all(prefix.join("Scripts")).expect("scripts");
            let path = prefix.join("python.exe");
            fs::write(&path, "").expect("python");
            fs::write(prefix.join("Scripts").join("pip.cmd"), "").expect("pip");
            path
        } else {
            fs::create_dir_all(prefix.join("bin")).expect("bin");
            let path = prefix.join("python");
            fs::write(&path, "").expect("python");
            fs::write(prefix.join("bin").join("pip"), "").expect("pip");
            path
        };
        let pip_path = if cfg!(windows) {
            prefix.join("Scripts").join("pip.cmd")
        } else {
            prefix.join("bin").join("pip")
        };

        let python =
            find_command_in_prefix(&prefix, "python", test_path_ext().as_deref()).expect("python");
        let pip = find_command_in_prefix(&prefix, "pip", test_path_ext().as_deref()).expect("pip");

        assert_eq!(python, python_path);
        assert_eq!(pip, pip_path);
        assert_eq!(prefix_bin_dirs(&prefix).len(), 3);
    }

    #[test]
    fn shim_inventory_normalizes_extensions() {
        let temp = TempDir::new().expect("tempdir");
        let prefix = temp.path().join("runtime");
        if cfg!(windows) {
            fs::create_dir_all(prefix.join("Scripts")).expect("scripts");
            fs::write(prefix.join("python.exe"), "").expect("python");
            fs::write(prefix.join("Scripts").join("pip3.13.cmd"), "").expect("pip");
            fs::write(prefix.join("Scripts").join("activate.bat"), "").expect("activate");
            fs::write(prefix.join("Scripts").join("pythonw.dll"), "").expect("dll");

            let names = collect_shim_names_from_prefix(&prefix, test_path_ext().as_deref())
                .expect("inventory");
            assert_eq!(names, vec!["activate", "pip3.13", "python"]);
        } else {
            fs::create_dir_all(prefix.join("bin")).expect("bin");
            let python = prefix.join("bin").join("python");
            let pip = prefix.join("bin").join("pip3.13");
            let activate = prefix.join("bin").join("activate");
            fs::write(&python, "").expect("python");
            fs::write(&pip, "").expect("pip");
            fs::write(&activate, "").expect("activate");
            fs::write(prefix.join("bin").join("pythonw.dll"), "").expect("dll");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                for path in [&python, &pip, &activate] {
                    let metadata = fs::metadata(path).expect("metadata");
                    let mut permissions = metadata.permissions();
                    permissions.set_mode(0o755);
                    fs::set_permissions(path, permissions).expect("chmod");
                }
            }

            let names = collect_shim_names_from_prefix(&prefix, None).expect("inventory");
            assert_eq!(names, vec!["activate", "pip3.13", "python"]);
        }
    }
}
