// ./crates/pyenv-core/src/meta/prompt.rs
//! High-speed prompt summary command for shell integrations.

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::venv::resolve_managed_venv;
use crate::version::resolve_selected_versions;

pub fn cmd_prompt(ctx: &AppContext) -> CommandReport {
    let selection = resolve_selected_versions(ctx, false);

    if selection.versions.is_empty() {
        return CommandReport::success(vec!["".to_string()]); // No active version
    }

    let primary = &selection.versions[0];

    match resolve_managed_venv(ctx, primary) {
        Ok(info) => {
            CommandReport::success(vec![format!("(venv:{}) {}", info.name, info.base_version)])
        }
        Err(_) => CommandReport::success(vec![primary.clone()]),
    }
}
