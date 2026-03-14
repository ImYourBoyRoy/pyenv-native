// ./crates/pyenv-core/src/context.rs
//! Runtime context built from process environment and persisted configuration.

use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

use crate::config::{AppConfig, load_config, resolve_cache_dir, resolve_versions_dir};
use crate::error::PyenvError;

#[derive(Debug, Clone)]
pub struct AppContext {
    pub root: PathBuf,
    pub dir: PathBuf,
    pub exe_path: PathBuf,
    pub env_version: Option<String>,
    pub env_shell: Option<String>,
    pub path_env: Option<OsString>,
    pub path_ext: Option<OsString>,
    pub config: AppConfig,
}

impl AppContext {
    pub fn from_system() -> Result<Self, PyenvError> {
        let exe_path = env::current_exe().unwrap_or_else(|_| PathBuf::from("pyenv"));
        let root = resolve_root(
            env::var_os("PYENV_ROOT"),
            env::var_os("USERPROFILE"),
            env::var_os("HOME"),
            Some(&exe_path),
        )?;
        let dir = resolve_dir(env::var_os("PYENV_DIR"))?;
        let env_version = env::var("PYENV_VERSION")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let env_shell = env::var("PYENV_SHELL")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let path_env = env::var_os("PATH");
        let path_ext = env::var_os("PATHEXT");
        let config = load_config(&root)?;
        Ok(Self {
            root,
            dir,
            exe_path,
            env_version,
            env_shell,
            path_env,
            path_ext,
            config,
        })
    }

    pub fn versions_dir(&self) -> PathBuf {
        resolve_versions_dir(&self.root, &self.config)
    }

    pub fn cache_dir(&self) -> PathBuf {
        resolve_cache_dir(&self.root, &self.config)
    }

    pub fn shims_dir(&self) -> PathBuf {
        self.root.join("shims")
    }
}

pub fn resolve_root(
    explicit_root: Option<OsString>,
    user_profile: Option<OsString>,
    home: Option<OsString>,
    exe_path: Option<&PathBuf>,
) -> Result<PathBuf, PyenvError> {
    if let Some(root) = explicit_root.filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(root));
    }

    if let Some(inferred) = exe_path.and_then(infer_root_from_exe) {
        return Ok(inferred);
    }

    if let Some(home_dir) = user_profile.or(home) {
        return Ok(PathBuf::from(home_dir).join(".pyenv"));
    }

    Err(PyenvError::MissingHome)
}

pub fn resolve_dir(explicit_dir: Option<OsString>) -> Result<PathBuf, PyenvError> {
    let dir = explicit_dir
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    if !dir.exists() || !dir.is_dir() {
        return Err(PyenvError::InvalidDirectory(dir.display().to_string()));
    }

    Ok(dir)
}

fn infer_root_from_exe(exe_path: &PathBuf) -> Option<PathBuf> {
    let bin_dir = exe_path.parent()?;
    let bin_name = bin_dir.file_name()?.to_string_lossy().to_ascii_lowercase();
    if bin_name == "bin" {
        bin_dir.parent().map(PathBuf::from)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::resolve_root;

    #[test]
    fn resolve_root_prefers_explicit_root() {
        let sample_home = if cfg!(windows) {
            OsString::from("C:\\Users\\Roy")
        } else {
            OsString::from("/home/roy")
        };
        let sample_exe = if cfg!(windows) {
            PathBuf::from("C:\\portable\\.pyenv\\bin\\pyenv.exe")
        } else {
            PathBuf::from("/portable/.pyenv/bin/pyenv")
        };
        let root = resolve_root(
            Some(OsString::from("C:\\custom-root")),
            Some(sample_home),
            None,
            Some(&sample_exe),
        )
        .expect("root");
        assert_eq!(root, PathBuf::from("C:\\custom-root"));
    }

    #[test]
    fn resolve_root_can_infer_portable_root_from_exe_path() {
        let sample_home = if cfg!(windows) {
            OsString::from("C:\\Users\\Roy")
        } else {
            OsString::from("/home/roy")
        };
        let sample_exe = if cfg!(windows) {
            PathBuf::from("C:\\portable\\.pyenv\\bin\\pyenv.exe")
        } else {
            PathBuf::from("/portable/.pyenv/bin/pyenv")
        };
        let expected_root = if cfg!(windows) {
            PathBuf::from("C:\\portable\\.pyenv")
        } else {
            PathBuf::from("/portable/.pyenv")
        };
        let root = resolve_root(None, Some(sample_home), None, Some(&sample_exe)).expect("root");
        assert_eq!(root, expected_root);
    }
}
