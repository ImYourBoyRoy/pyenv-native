// ./crates/pyenv-core/src/shell/mod.rs
//! Shell command orchestration for init output, shell-scoped selection, and managed-venv
//! activation compatibility. This module keeps the public shell API stable while delegating
//! rendering and validation logic into focused submodules.

mod emit;
mod helpers;
mod init;
#[cfg(test)]
mod tests;
mod types;

use crate::command::CommandReport;
use crate::context::AppContext;

use self::emit::{
    render_cmd_exec_line, shell_emit_activate, shell_emit_deactivate, shell_emit_rehash,
    shell_emit_revert, shell_emit_set, shell_emit_show_current, shell_emit_unset,
};
use self::helpers::{resolve_activation_target, validate_shell_versions, virtualenv_bin_dir};
use self::init::{
    effective_shell, ensure_init_dirs, parse_init_args, render_detect_shell, render_init_help,
    render_init_path, render_init_print,
};
pub use self::types::InitCommandOptions;
use self::types::InitMode;

pub fn cmd_shell(_ctx: &AppContext, _args: &[String]) -> CommandReport {
    CommandReport::failure(
        vec![
            "pyenv: shell integration not enabled. Run `pyenv init' for instructions.".to_string(),
        ],
        1,
    )
}

pub fn cmd_activate(_ctx: &AppContext, _args: &[String]) -> CommandReport {
    CommandReport::failure(
        vec![
            "pyenv: shell integration not enabled. Run `pyenv init` or `pyenv virtualenv-init -` first.".to_string(),
        ],
        1,
    )
}

pub fn cmd_deactivate(_ctx: &AppContext, _args: &[String]) -> CommandReport {
    CommandReport::failure(
        vec![
            "pyenv: shell integration not enabled. Run `pyenv init` or `pyenv virtualenv-init -` first.".to_string(),
        ],
        1,
    )
}

pub fn cmd_virtualenv_init(ctx: &AppContext, args: &[String]) -> CommandReport {
    cmd_init(ctx, args)
}

pub fn cmd_sh_shell(ctx: &AppContext, args: &[String]) -> CommandReport {
    let shell = effective_shell(ctx);
    let args = strip_shell_separator(args);

    if args.is_empty() {
        return match &ctx.env_version {
            Some(_) => CommandReport::success(shell_emit_show_current(shell)),
            None => CommandReport::failure(
                vec!["pyenv: no shell-specific version configured".to_string()],
                1,
            ),
        };
    }

    if args.len() == 1 && args[0] == "--unset" {
        return CommandReport::success(shell_emit_unset(shell));
    }

    if args.len() == 1 && args[0] == "-" {
        return CommandReport::success(shell_emit_revert(shell));
    }

    let requested = args
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if requested.is_empty() {
        return CommandReport::failure(
            vec!["pyenv: no shell-specific version configured".to_string()],
            1,
        );
    }

    if let Err(error) = validate_shell_versions(ctx, &requested) {
        return CommandReport::failure(vec![error.to_string()], 1);
    }

    let version_value = requested.join(":");
    if ctx.env_version.as_deref() == Some(version_value.as_str()) {
        return CommandReport::empty_success();
    }

    CommandReport::success(shell_emit_set(shell, &version_value))
}

pub fn cmd_sh_activate(ctx: &AppContext, args: &[String]) -> CommandReport {
    let shell = effective_shell(ctx);
    let args = strip_shell_separator(args);

    let info = match resolve_activation_target(ctx, args.first().map(String::as_str)) {
        Ok(info) => info,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };

    let Some(bin_dir) = virtualenv_bin_dir(&info.path) else {
        return CommandReport::failure(
            vec![format!(
                "pyenv: managed venv `{}` is missing its executable directory under {}",
                info.spec,
                info.path.display()
            )],
            1,
        );
    };

    CommandReport::success(shell_emit_activate(
        shell,
        &info.spec,
        &info.path.display().to_string(),
        &bin_dir.display().to_string(),
    ))
}

pub fn cmd_sh_deactivate(ctx: &AppContext, _args: &[String]) -> CommandReport {
    CommandReport::success(shell_emit_deactivate(effective_shell(ctx)))
}

pub fn cmd_sh_rehash(ctx: &AppContext) -> CommandReport {
    CommandReport::success(shell_emit_rehash(
        effective_shell(ctx),
        &ctx.exe_path.display().to_string(),
    ))
}

pub fn cmd_sh_cmd(ctx: &AppContext, args: &[String]) -> CommandReport {
    let args = strip_shell_separator(args);
    let Some((command, rest)) = args.split_first() else {
        return CommandReport::success(vec![format!("\"{}\"", ctx.exe_path.display())]);
    };

    match command.to_ascii_lowercase().as_str() {
        "shell" => cmd_sh_shell(ctx, rest),
        "activate" => cmd_sh_activate(ctx, rest),
        "deactivate" => cmd_sh_deactivate(ctx, rest),
        "rehash" => cmd_sh_rehash(ctx),
        _ => CommandReport::success(vec![render_cmd_exec_line(&ctx.exe_path, args)]),
    }
}

pub fn cmd_init(ctx: &AppContext, args: &[String]) -> CommandReport {
    let options = match parse_init_args(ctx, args) {
        Ok(options) => options,
        Err(error) => return CommandReport::failure(vec![error], 1),
    };

    if let Err(error) = ensure_init_dirs(ctx) {
        return CommandReport::failure(vec![error.to_string()], 1);
    }

    match options.mode {
        InitMode::Help => CommandReport {
            stdout: Vec::new(),
            stderr: render_init_help(options.shell),
            exit_code: 1,
        },
        InitMode::DetectShell => CommandReport::success(render_detect_shell(options.shell)),
        InitMode::Path => CommandReport::success(render_init_path(ctx, &options)),
        InitMode::Print => CommandReport::success(render_init_print(ctx, &options)),
    }
}

fn strip_shell_separator(args: &[String]) -> &[String] {
    if matches!(args.first().map(String::as_str), Some("--")) {
        &args[1..]
    } else {
        args
    }
}
