// ./crates/pyenv-core/src/meta/shims.rs
//! Shim inventory rendering for `pyenv shims` and completion backends that need shim names.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::runtime::normalize_shim_name;

pub fn cmd_shims(ctx: &AppContext, short: bool) -> CommandReport {
    let entries = match list_shim_entries(ctx) {
        Ok(entries) => entries,
        Err(error) => return CommandReport::failure(vec![error], 1),
    };

    if short {
        return CommandReport::success(entries.into_keys().collect());
    }

    CommandReport::success(
        entries
            .into_values()
            .map(|path| path.display().to_string())
            .collect(),
    )
}

pub(super) fn list_shim_entries(ctx: &AppContext) -> Result<BTreeMap<String, PathBuf>, String> {
    let shims_dir = ctx.shims_dir();
    if !shims_dir.is_dir() {
        return Ok(BTreeMap::new());
    }

    let mut entries = fs::read_dir(&shims_dir)
        .map_err(|error| format!("pyenv: {error}"))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter_map(|path| {
            let normalized = normalize_shim_name(&path, ctx.path_ext.as_deref())?;
            Some((normalized, path))
        })
        .collect::<Vec<_>>();

    entries.sort_by(|(lhs_name, lhs_path), (rhs_name, rhs_path)| {
        lhs_name
            .to_ascii_lowercase()
            .cmp(&rhs_name.to_ascii_lowercase())
            .then_with(|| {
                preferred_shim_rank(lhs_path)
                    .cmp(&preferred_shim_rank(rhs_path))
                    .then_with(|| {
                        lhs_path
                            .display()
                            .to_string()
                            .to_ascii_lowercase()
                            .cmp(&rhs_path.display().to_string().to_ascii_lowercase())
                    })
            })
    });

    let mut selected = BTreeMap::new();
    for (name, path) in entries {
        selected.entry(name).or_insert(path);
    }
    Ok(selected)
}

fn preferred_shim_rank(path: &Path) -> usize {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("exe") => 0,
        Some("cmd") => 1,
        Some("bat") => 2,
        Some("ps1") => 3,
        None => 0,
        _ => 10,
    }
}
