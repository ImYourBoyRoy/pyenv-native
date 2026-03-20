// ./crates/pyenv-core/src/catalog/commands.rs
//! Public catalog commands for known-version listing and latest-version resolution.

use crate::command::CommandReport;
use crate::context::AppContext;

use super::entries::{filter_catalog_entries, group_entries, known_catalog_entries};
use super::latest::{latest_installed_version, latest_known_version};
use super::types::InstallListOptions;

pub fn cmd_install_list(_ctx: &AppContext, options: &InstallListOptions) -> CommandReport {
    let entries = filter_catalog_entries(known_catalog_entries(), options);
    if entries.is_empty() {
        return CommandReport::failure(
            vec!["pyenv: no known versions match the requested filters".to_string()],
            1,
        );
    }

    let groups = group_entries(entries);
    if options.json {
        match serde_json::to_string_pretty(&groups) {
            Ok(json) => CommandReport::success(json.lines().map(ToOwned::to_owned).collect()),
            Err(error) => CommandReport::failure(
                vec![format!(
                    "pyenv: failed to serialize install catalog: {error}"
                )],
                1,
            ),
        }
    } else {
        let mut stdout = vec!["Available versions:".to_string()];
        for group in &groups {
            stdout.push(String::new());
            stdout.push(group.family.clone());
            stdout.extend(group.versions.iter().map(|version| format!("  {version}")));
        }
        CommandReport::success(stdout)
    }
}

pub fn cmd_latest(
    ctx: &AppContext,
    prefix: &str,
    known: bool,
    bypass: bool,
    force: bool,
) -> CommandReport {
    let resolved = if known {
        latest_known_version(prefix)
    } else {
        latest_installed_version(ctx, prefix)
    };

    if let Some(version) = resolved {
        return CommandReport::success_one(version);
    }

    if bypass {
        return CommandReport {
            stdout: vec![prefix.to_string()],
            stderr: Vec::new(),
            exit_code: if force { 0 } else { 1 },
        };
    }

    let scope = if known { "known" } else { "installed" };
    CommandReport::failure(
        vec![format!(
            "pyenv: no {scope} versions match the prefix `{prefix}'"
        )],
        1,
    )
}
