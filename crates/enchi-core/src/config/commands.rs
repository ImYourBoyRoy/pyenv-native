// ./crates/enchi-core/src/config/commands.rs
//! Command entrypoints for inspecting and mutating persisted Enchi config values.

use crate::command::CommandReport;
use crate::context::AppContext;

use super::storage::{config_path, save_config};
use super::values::{get_config_value, set_config_value};

pub fn cmd_config_path(ctx: &AppContext) -> CommandReport {
    CommandReport::success_one(config_path(&ctx.root).display().to_string())
}

/// Drop `[section]` headers that have no keys under them, so `config show` prints only what is set.
pub(super) fn without_empty_sections(contents: &str) -> Vec<String> {
    let lines: Vec<&str> = contents.lines().collect();
    let mut kept = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.starts_with('[') {
            let next = lines[index + 1..]
                .iter()
                .find(|candidate| !candidate.trim().is_empty());
            if next.is_none_or(|candidate| candidate.starts_with('[')) {
                continue;
            }
        }
        let blank_after_blank = line.trim().is_empty()
            && kept
                .last()
                .is_some_and(|last: &String| last.trim().is_empty());
        if !blank_after_blank {
            kept.push((*line).to_string());
        }
    }
    while kept.first().is_some_and(|line| line.trim().is_empty()) {
        kept.remove(0);
    }
    while kept.last().is_some_and(|line| line.trim().is_empty()) {
        kept.pop();
    }
    kept
}

pub fn cmd_config_show(ctx: &AppContext) -> CommandReport {
    match toml::to_string_pretty(&ctx.config) {
        Ok(contents) => CommandReport::success(without_empty_sections(&contents)),
        Err(error) => CommandReport::failure(
            vec![format!("enchi: failed to serialize config: {error}")],
            1,
        ),
    }
}

pub fn cmd_config_get(ctx: &AppContext, key: &str) -> CommandReport {
    match get_config_value(&ctx.config, key) {
        Ok(value) => CommandReport::success_one(value),
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

pub fn cmd_config_set(ctx: &mut AppContext, key: &str, value: &str) -> CommandReport {
    let mut config = ctx.config.clone();
    match set_config_value(&mut config, key, value).and_then(|_| save_config(&ctx.root, &config)) {
        Ok(_) => {
            ctx.config = config;
            if key == "ui.language" {
                crate::i18n::apply(&ctx.config);
            }
            CommandReport::empty_success()
        }
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}
