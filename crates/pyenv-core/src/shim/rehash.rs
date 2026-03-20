// ./crates/pyenv-core/src/shim/rehash.rs
//! Shim manifest, locking, inventory scanning, and rehash execution.

use std::collections::HashSet;
use std::fs;
use std::fs::OpenOptions;
use std::path::Path;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::PyenvError;
use crate::catalog::installed_version_names;
use crate::command::CommandReport;
use crate::context::AppContext;
use crate::plugin::collect_rehash_hook_names;
use crate::runtime::{inventory_roots_for_version, prefix_bin_dirs};

use super::paths::remove_shim_artifacts;
use super::render::write_shim_artifacts;
use super::types::{
    RehashLockGuard, SHIM_LOCK_FILE, SHIM_LOCK_STALE_SECS, SHIM_MANIFEST_FILE, ShimManifest,
};

pub fn cmd_rehash(ctx: &AppContext) -> CommandReport {
    match rehash_shims(ctx) {
        Ok(_) => CommandReport::empty_success(),
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

pub(crate) fn rehash_shims(ctx: &AppContext) -> Result<usize, PyenvError> {
    let shims_dir = ctx.shims_dir();
    fs::create_dir_all(&shims_dir).map_err(io_error)?;
    let _lock = acquire_rehash_lock(&shims_dir)?;

    let commands = collect_rehash_commands(ctx)?;
    let previous = read_shim_manifest(&shims_dir).unwrap_or_default();
    let current = commands.iter().cloned().collect::<HashSet<_>>();

    for command in &commands {
        write_shim_artifacts(ctx, &shims_dir, command)?;
    }

    for stale in previous
        .commands
        .into_iter()
        .filter(|name| !current.contains(name))
    {
        remove_shim_artifacts(&shims_dir, &stale);
    }

    write_shim_manifest(&shims_dir, &commands)?;
    Ok(commands.len())
}

fn collect_rehash_commands(ctx: &AppContext) -> Result<Vec<String>, PyenvError> {
    let versions = installed_version_names(ctx)?;
    let mut commands = HashSet::new();

    for version in versions {
        for prefix in inventory_roots_for_version(ctx, &version) {
            for directory in prefix_bin_dirs(&prefix) {
                if !directory.is_dir() {
                    continue;
                }

                for entry in fs::read_dir(&directory).map_err(io_error)? {
                    let entry = entry.map_err(io_error)?;
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }

                    if let Some(name) =
                        crate::runtime::normalize_shim_name(&path, ctx.path_ext.as_deref())
                    {
                        commands.insert(name);
                    }
                }
            }
        }
    }

    for hook_name in collect_rehash_hook_names(ctx, &[])? {
        commands.insert(hook_name);
    }

    let mut values = commands.into_iter().collect::<Vec<_>>();
    values.sort_by_key(|value| value.to_ascii_lowercase());
    Ok(values)
}

fn read_shim_manifest(shims_dir: &Path) -> Option<ShimManifest> {
    let path = shims_dir.join(SHIM_MANIFEST_FILE);
    let contents = fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

fn write_shim_manifest(shims_dir: &Path, commands: &[String]) -> Result<(), PyenvError> {
    let path = shims_dir.join(SHIM_MANIFEST_FILE);
    let manifest = ShimManifest {
        generated_at_epoch_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        commands: commands.to_vec(),
    };
    let payload = serde_json::to_string_pretty(&manifest).map_err(|error| {
        PyenvError::Io(format!("pyenv: failed to serialize shim manifest: {error}"))
    })?;
    fs::write(path, payload).map_err(io_error)
}

fn acquire_rehash_lock(shims_dir: &Path) -> Result<RehashLockGuard, PyenvError> {
    let path = shims_dir.join(SHIM_LOCK_FILE);
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let payload = format!("pid={}\ncreated_at={created_at}\n", process::id());

    for _ in 0..2 {
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                use std::io::Write as _;

                file.write_all(payload.as_bytes()).map_err(io_error)?;
                file.flush().map_err(io_error)?;
                return Ok(RehashLockGuard { path });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if lock_file_is_stale(&path) {
                    let _ = fs::remove_file(&path);
                    continue;
                }
                return Err(PyenvError::Io(format!(
                    "pyenv: cannot rehash: lock {} already exists",
                    path.display()
                )));
            }
            Err(error) => return Err(io_error(error)),
        }
    }

    Err(PyenvError::Io(format!(
        "pyenv: cannot rehash: failed to acquire lock {}",
        path.display()
    )))
}

fn lock_file_is_stale(path: &Path) -> bool {
    let Ok(contents) = fs::read_to_string(path) else {
        return false;
    };
    let Some(created_at) = contents
        .lines()
        .find_map(|line| line.strip_prefix("created_at="))
        .and_then(|value| value.parse::<u64>().ok())
    else {
        return false;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    now.saturating_sub(created_at) > SHIM_LOCK_STALE_SECS
}

fn io_error(error: std::io::Error) -> PyenvError {
    PyenvError::Io(format!("pyenv: {error}"))
}
