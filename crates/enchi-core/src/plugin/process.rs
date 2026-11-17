// ./crates/enchi-core/src/plugin/process.rs
//! Cross-shell process launching for plugin commands and hook scripts.

use crate::process::PyenvCommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::context::AppContext;
use crate::error::PyenvError;
use crate::executable::find_system_command;

pub(crate) fn preferred_powershell_host(_ctx: &AppContext) -> &'static str {
    crate::process::windows_powershell_host()
}

pub(crate) fn powershell_7_available(ctx: &AppContext) -> bool {
    find_system_command(ctx, "pwsh").is_some()
}

/// Registration functions of classic pyenv's hook API. In classic pyenv, `pyenv install` and
/// `pyenv uninstall` define them and then *source* each `.bash` hook, which calls them to register a
/// callback (for example pyenv-virtualenv's `before_uninstall "uninstall_related_virtual_env"`).
/// Enchi runs hooks as their own processes and performs install and uninstall natively (including
/// removing dependent managed venvs), so these are accepted and ignored. Without them every such
/// hook dies with `command not found`, and a failing hook aborts the command.
const CLASSIC_HOOK_API: &str = "before_install() { :; }; after_install() { :; }; \
    before_uninstall() { :; }; after_uninstall() { :; }; . \"$0\" \"$@\"";

pub(super) fn run_process(
    path: &Path,
    args: &[String],
    ctx: &AppContext,
    extra_env: &[(&str, String)],
    capture_output: bool,
) -> Result<(i32, Vec<String>, Vec<String>), PyenvError> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    let mut command = match extension.as_deref() {
        Some("ps1") => {
            let host = preferred_powershell_host(ctx);
            let mut command = Command::new(host);
            command
                .headless()
                .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"]);
            command.arg(path);
            command
        }
        Some("cmd" | "bat") => {
            let mut command = Command::new("cmd");
            command.headless().arg("/C");
            command.arg(path);
            command
        }
        Some("bash") => {
            // Sourced, as classic pyenv does, with its hook API defined (see CLASSIC_HOOK_API).
            let mut command = Command::new("bash");
            command.headless().arg("-c").arg(CLASSIC_HOOK_API).arg(path);
            command
        }
        Some("sh") => {
            let mut command = Command::new("sh");
            command.headless().arg(path);
            command
        }
        _ => {
            let mut c = Command::new(path);
            c.headless();
            c
        }
    };

    command.args(args);
    command.current_dir(&ctx.dir);
    command.env("PYENV_ROOT", &ctx.root);
    command.env("ENCHI_ROOT", &ctx.root);
    command.env("PYENV_DIR", &ctx.dir);
    command.env("PYENV_EXE", ctx.cli_exe_path());
    if let Some(version) = &ctx.env_version {
        command.env("PYENV_VERSION", version);
    }
    if let Some(shell) = &ctx.env_shell {
        command.env("PYENV_SHELL", shell);
    }

    for (key, value) in extra_env {
        command.env(key, value);
    }

    if capture_output {
        let output = command.output().map_err(|error| {
            PyenvError::Io(format!("enchi: failed to run {}: {error}", path.display()))
        })?;
        Ok((
            output.status.code().unwrap_or(1),
            split_lines(&String::from_utf8_lossy(&output.stdout)),
            split_lines(&String::from_utf8_lossy(&output.stderr)),
        ))
    } else {
        let status = command
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|error| {
                PyenvError::Io(format!("enchi: failed to run {}: {error}", path.display()))
            })?;
        Ok((status.code().unwrap_or(1), Vec::new(), Vec::new()))
    }
}

fn split_lines(value: &str) -> Vec<String> {
    value.lines().map(ToOwned::to_owned).collect()
}
