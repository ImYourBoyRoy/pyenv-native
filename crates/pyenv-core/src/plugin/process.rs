// ./crates/pyenv-core/src/plugin/process.rs
//! Cross-shell process launching for plugin commands and hook scripts.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::context::AppContext;
use crate::error::PyenvError;

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
            let mut command = Command::new("powershell");
            command.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"]);
            command.arg(path);
            command
        }
        Some("cmd" | "bat") => {
            let mut command = Command::new("cmd");
            command.arg("/C");
            command.arg(path);
            command
        }
        Some("sh" | "bash") => {
            let mut command = Command::new(if extension.as_deref() == Some("bash") {
                "bash"
            } else {
                "sh"
            });
            command.arg(path);
            command
        }
        _ => Command::new(path),
    };

    command.args(args);
    command.current_dir(&ctx.dir);
    command.env("PYENV_ROOT", &ctx.root);
    command.env("PYENV_DIR", &ctx.dir);
    command.env("PYENV_EXE", &ctx.exe_path);
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
            PyenvError::Io(format!("pyenv: failed to run {}: {error}", path.display()))
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
                PyenvError::Io(format!("pyenv: failed to run {}: {error}", path.display()))
            })?;
        Ok((status.code().unwrap_or(1), Vec::new(), Vec::new()))
    }
}

fn split_lines(value: &str) -> Vec<String> {
    value.lines().map(ToOwned::to_owned).collect()
}
