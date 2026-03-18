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
        let explicit = PathBuf::from(&root);

        // When PYENV_ROOT was set by pyenv-win (typically ending in `pyenv-win`),
        // the native binary should prefer its own exe-inferred root if available.
        // This lets pyenv-native coexist without manual env-var cleanup.
        if is_pyenv_win_root(&explicit)
            && let Some(inferred) = exe_path.and_then(infer_root_from_exe)
                && !paths_equivalent(&inferred, &explicit) {
                    return Ok(inferred);
                }

        return Ok(explicit);
    }

    if let Some(inferred) = exe_path.and_then(infer_root_from_exe) {
        return Ok(inferred);
    }

    if let Some(home_dir) = user_profile.or(home) {
        return Ok(PathBuf::from(home_dir).join(".pyenv"));
    }

    Err(PyenvError::MissingHome)
}

/// Returns `true` when the env-supplied root looks like it was set by pyenv-win.
///
/// Checks whether the final path component is `pyenv-win` (case-insensitive)
/// which is the layout pyenv-win uses: `~/.pyenv/pyenv-win/`.
pub fn is_pyenv_win_root(root: &std::path::Path) -> bool {
    // Trim any trailing separator so `file_name()` works on `…\pyenv-win\`.
    let cleaned = root
        .to_string_lossy()
        .trim_end_matches(['/', '\\'])
        .to_string();
    std::path::Path::new(&cleaned)
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("pyenv-win"))
}

fn paths_equivalent(lhs: &std::path::Path, rhs: &std::path::Path) -> bool {
    if cfg!(windows) {
        lhs.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .eq_ignore_ascii_case(
                rhs.to_string_lossy()
                    .replace('/', "\\")
                    .trim_end_matches('\\'),
            )
    } else {
        lhs == rhs
    }
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

    use super::{is_pyenv_win_root, resolve_root};

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

    #[test]
    fn resolve_root_overrides_pyenv_win_root_when_native_exe_available() {
        let pyenv_win_root = if cfg!(windows) {
            OsString::from("C:\\Users\\Roy\\.pyenv\\pyenv-win\\")
        } else {
            OsString::from("/home/roy/.pyenv/pyenv-win/")
        };
        let native_exe = if cfg!(windows) {
            PathBuf::from("C:\\Users\\Roy\\.pyenv\\bin\\pyenv.exe")
        } else {
            PathBuf::from("/home/roy/.pyenv/bin/pyenv")
        };
        let expected_root = if cfg!(windows) {
            PathBuf::from("C:\\Users\\Roy\\.pyenv")
        } else {
            PathBuf::from("/home/roy/.pyenv")
        };

        let root = resolve_root(Some(pyenv_win_root), None, None, Some(&native_exe)).expect("root");
        assert_eq!(root, expected_root);
    }

    #[test]
    fn resolve_root_keeps_explicit_root_when_not_pyenv_win() {
        let custom_root = OsString::from("D:\\my-pyenv-root");
        let native_exe = if cfg!(windows) {
            PathBuf::from("C:\\Users\\Roy\\.pyenv\\bin\\pyenv.exe")
        } else {
            PathBuf::from("/home/roy/.pyenv/bin/pyenv")
        };

        let root =
            resolve_root(Some(custom_root.clone()), None, None, Some(&native_exe)).expect("root");
        assert_eq!(root, PathBuf::from("D:\\my-pyenv-root"));
    }

    #[test]
    fn is_pyenv_win_root_detects_pyenv_win_paths() {
        assert!(is_pyenv_win_root(std::path::Path::new(
            "C:\\Users\\Roy\\.pyenv\\pyenv-win\\"
        )));
        assert!(is_pyenv_win_root(std::path::Path::new(
            "C:\\Users\\Roy\\.pyenv\\pyenv-win"
        )));
        assert!(is_pyenv_win_root(std::path::Path::new(
            "C:\\Users\\Roy\\.pyenv\\PYENV-WIN\\"
        )));
        assert!(!is_pyenv_win_root(std::path::Path::new(
            "C:\\Users\\Roy\\.pyenv"
        )));
        assert!(!is_pyenv_win_root(std::path::Path::new("D:\\custom-root")));
    }
}
