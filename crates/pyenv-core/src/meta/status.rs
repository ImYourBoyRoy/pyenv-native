// ./crates/pyenv-core/src/meta/status.rs
//! High-level environment dashboard command to inspect the current state.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::venv::resolve_managed_venv;
use crate::version::resolve_selected_versions;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentStatus {
    pub root: PathBuf,
    pub active_versions: Vec<String>,
    pub origin: String,
    pub managed_venv: Option<ManagedVenvSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedVenvSummary {
    pub name: String,
    pub spec: String,
    pub base_version: String,
    pub venv_path: PathBuf,
}

pub fn cmd_status(ctx: &AppContext, json: bool) -> CommandReport {
    let status = build_environment_status(ctx);
    if json {
        let payload = serde_json::to_string_pretty(&status).unwrap_or_else(|_| "{}".to_string());
        return CommandReport::success(vec![payload]);
    }

    let mut stdout = Vec::new();
    stdout.push("Pyenv Environment Status:".to_string());
    stdout.push(format!("  Root: {}", status.root.display()));

    if status.active_versions.is_empty() {
        stdout.push("  Active Versions: none".to_string());
    } else {
        stdout.push(format!(
            "  Active Versions: {}",
            status.active_versions.join(", ")
        ));
    }

    stdout.push(format!("  Origin: {}", status.origin));

    if let Some(venv) = status.managed_venv {
        stdout.push(format!(
            "  Managed Venv: {} (base: {})",
            venv.name, venv.base_version
        ));
        stdout.push(format!("  Venv Path: {}", venv.venv_path.display()));
    } else {
        stdout.push("  Managed Venv: none active".to_string());
    }

    CommandReport::success(stdout)
}

pub fn build_environment_status(ctx: &AppContext) -> EnvironmentStatus {
    let selection = resolve_selected_versions(ctx, false);

    let mut managed_venv = None;
    if let Some(primary) = selection.versions.first()
        && let Ok(info) = resolve_managed_venv(ctx, primary)
    {
        managed_venv = Some(ManagedVenvSummary {
            name: info.name,
            spec: info.spec,
            base_version: info.base_version,
            venv_path: info.path,
        });
    }

    EnvironmentStatus {
        root: ctx.root.clone(),
        active_versions: selection.versions,
        origin: selection.origin.to_string(),
        managed_venv,
    }
}
