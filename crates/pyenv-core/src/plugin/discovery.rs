// ./crates/pyenv-core/src/plugin/discovery.rs
//! Plugin binary and hook root discovery for pyenv-compatible extension points.

use std::collections::HashSet;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use crate::context::AppContext;
use crate::error::PyenvError;
use crate::runtime::{candidate_file_names, search_path_entries};

pub(super) fn discover_hook_scripts(
    ctx: &AppContext,
    hook: &str,
) -> Result<Vec<PathBuf>, PyenvError> {
    if hook.trim().is_empty() {
        return Err(PyenvError::Io("Usage: pyenv hooks <command>".to_string()));
    }

    let mut scripts = Vec::new();
    for hook_root in hook_search_roots(ctx) {
        let hook_dir = hook_root.join(hook);
        if !hook_dir.is_dir() {
            continue;
        }

        let mut entries = fs::read_dir(&hook_dir)
            .map_err(io_error)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter(|path| is_supported_hook_script(path))
            .collect::<Vec<_>>();
        entries.sort_by_key(|path| {
            path.file_name()
                .map(|value| value.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default()
        });
        scripts.extend(entries.into_iter().map(resolve_hook_script_path));
    }

    Ok(scripts)
}

pub fn discover_plugin_commands(ctx: &AppContext) -> Vec<String> {
    let mut commands = HashSet::new();

    for bin_dir in plugin_search_dirs(ctx) {
        let entries = fs::read_dir(&bin_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();

        for path in entries {
            let Some(stem) = path
                .file_stem()
                .map(|value| value.to_string_lossy().to_string())
            else {
                continue;
            };
            let Some(command) = stem.strip_prefix("pyenv-") else {
                continue;
            };
            if !command.trim().is_empty() {
                commands.insert(command.to_string());
            }
        }
    }

    let mut values = commands.into_iter().collect::<Vec<_>>();
    values.sort_by_key(|value| value.to_ascii_lowercase());
    values
}

pub fn find_plugin_command(ctx: &AppContext, command: &str) -> Option<PathBuf> {
    let plugin_name = format!("pyenv-{command}");
    for bin_dir in plugin_search_dirs(ctx) {
        if let Some(path) = search_path_entries(
            std::slice::from_ref(&bin_dir),
            &plugin_name,
            ctx.path_ext.as_deref(),
        ) {
            return Some(path);
        }

        for candidate in candidate_file_names(&plugin_name, Some(OsStr::new(".ps1;.sh;.bash"))) {
            let path = bin_dir.join(candidate);
            if path.is_file() {
                return Some(path);
            }
        }
    }

    None
}

fn plugin_search_dirs(ctx: &AppContext) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(extra) = env::var_os("PYENV_PLUGIN_PATH") {
        roots.extend(env::split_paths(&extra));
    }

    roots.extend(default_plugin_bin_dirs(ctx));
    if let Some(path_env) = &ctx.path_env {
        roots.extend(env::split_paths(path_env));
    }
    dedup_paths(roots)
}

fn hook_search_roots(ctx: &AppContext) -> Vec<PathBuf> {
    hook_search_roots_with_extra(ctx, env::var_os("PYENV_HOOK_PATH"))
}

pub(super) fn hook_search_roots_with_extra(
    ctx: &AppContext,
    extra: Option<OsString>,
) -> Vec<PathBuf> {
    let mut roots = extra
        .map(|extra| {
            env::split_paths(&extra)
                .map(resolve_relative_path)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    roots.extend(default_hook_roots(ctx));
    dedup_paths(roots)
}

fn default_plugin_bin_dirs(ctx: &AppContext) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    dirs.extend(collect_plugin_bins_under(&ctx.root.join("plugins")));

    for ancestor in exe_ancestor_roots(ctx) {
        dirs.extend(collect_plugin_bins_under(&ancestor.join("plugins")));
    }

    dirs
}

fn default_hook_roots(ctx: &AppContext) -> Vec<PathBuf> {
    let mut roots = vec![ctx.root.join("pyenv.d")];
    roots.extend(collect_plugin_hook_roots_under(&ctx.root.join("plugins")));
    roots.extend(sibling_hook_roots_for_bin_dirs(&plugin_search_dirs(ctx)));
    roots.extend(system_hook_roots());

    for ancestor in exe_ancestor_roots(ctx) {
        roots.push(ancestor.join("pyenv.d"));
        roots.extend(collect_plugin_hook_roots_under(&ancestor.join("plugins")));
    }

    roots
}

fn exe_ancestor_roots(ctx: &AppContext) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut current = ctx.exe_path.parent().map(Path::to_path_buf);
    for _ in 0..4 {
        let Some(path) = current.take() else {
            break;
        };
        roots.push(path.clone());
        current = path.parent().map(Path::to_path_buf);
    }
    roots
}

fn collect_plugin_bins_under(plugins_dir: &Path) -> Vec<PathBuf> {
    if !plugins_dir.is_dir() {
        return Vec::new();
    }

    fs::read_dir(plugins_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join("bin"))
        .filter(|path| path.is_dir())
        .collect()
}

fn collect_plugin_hook_roots_under(plugins_dir: &Path) -> Vec<PathBuf> {
    if !plugins_dir.is_dir() {
        return Vec::new();
    }

    fs::read_dir(plugins_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join("etc").join("pyenv.d"))
        .filter(|path| path.is_dir())
        .collect()
}

fn sibling_hook_roots_for_bin_dirs(bin_dirs: &[PathBuf]) -> Vec<PathBuf> {
    bin_dirs
        .iter()
        .filter_map(|bin_dir| {
            let plugin_root = bin_dir.parent()?;
            let hook_root = plugin_root.join("etc").join("pyenv.d");
            hook_root.is_dir().then_some(hook_root)
        })
        .collect()
}

fn resolve_hook_script_path(path: PathBuf) -> PathBuf {
    if cfg!(windows) {
        path
    } else {
        fs::canonicalize(&path).unwrap_or(path)
    }
}

fn dedup_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for path in paths {
        let normalized = normalize_path_key(&path);
        if seen.insert(normalized) {
            deduped.push(path);
        }
    }

    deduped
}

fn normalize_path_key(path: &Path) -> String {
    let resolved = resolve_relative_path(path.to_path_buf());
    if cfg!(windows) {
        resolved
            .to_string_lossy()
            .replace('/', "\\")
            .to_ascii_lowercase()
    } else {
        resolved.to_string_lossy().to_string()
    }
}

fn resolve_relative_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        env::current_dir()
            .map(|cwd| cwd.join(&path))
            .unwrap_or(path)
    }
}

pub(super) fn system_hook_roots() -> Vec<PathBuf> {
    if cfg!(windows) {
        return Vec::new();
    }

    vec![
        PathBuf::from("/usr/etc/pyenv.d"),
        PathBuf::from("/usr/local/etc/pyenv.d"),
        PathBuf::from("/etc/pyenv.d"),
        PathBuf::from("/usr/lib/pyenv/hooks"),
    ]
}

fn is_supported_hook_script(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .as_deref(),
        Some("ps1" | "cmd" | "bat" | "exe" | "sh" | "bash") | None
    )
}

fn io_error(error: std::io::Error) -> PyenvError {
    PyenvError::Io(format!("pyenv: {error}"))
}
