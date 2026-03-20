// ./crates/pyenv-core/src/config/commands.rs
//! Command entrypoints for inspecting and mutating persisted pyenv-native config values.

use crate::command::CommandReport;
use crate::context::AppContext;

use super::storage::{config_path, save_config};
use super::values::{get_config_value, set_config_value};

pub fn cmd_config_path(ctx: &AppContext) -> CommandReport {
    CommandReport::success_one(config_path(&ctx.root).display().to_string())
}

pub fn cmd_config_show(ctx: &AppContext) -> CommandReport {
    match toml::to_string_pretty(&ctx.config) {
        Ok(contents) => CommandReport::success(contents.lines().map(ToOwned::to_owned).collect()),
        Err(error) => CommandReport::failure(
            vec![format!("pyenv: failed to serialize config: {error}")],
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
            CommandReport::empty_success()
        }
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}
