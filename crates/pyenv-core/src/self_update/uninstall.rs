// ./crates/pyenv-core/src/self_update/uninstall.rs
//! Self-uninstall command to remove pyenv-native from the system.

use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::process::Command;

use crate::command::CommandReport;
use crate::context::AppContext;

pub fn cmd_self_uninstall(ctx: &AppContext, yes: bool) -> CommandReport {
    match run_self_uninstall(ctx, yes) {
        Ok(lines) => CommandReport::success(lines),
        Err(message) => CommandReport::failure(vec![message], 1),
    }
}

fn run_self_uninstall(ctx: &AppContext, yes: bool) -> Result<Vec<String>, String> {
    confirm_self_uninstall(ctx, yes)?;

    if cfg!(windows) {
        spawn_windows_uninstall(ctx)?;
        Ok(vec![
            format!(
                "Started pyenv-native uninstallation for install root {}.",
                ctx.root.display()
            ),
            "Pyenv will be removed shortly after this process exits.".to_string(),
        ])
    } else {
        run_posix_uninstall(ctx)?;
        Ok(vec![format!(
            "Uninstalled pyenv-native from {}.",
            ctx.root.display()
        )])
    }
}

fn confirm_self_uninstall(ctx: &AppContext, yes: bool) -> Result<(), String> {
    if yes {
        return Ok(());
    }

    if !io::stdin().is_terminal() {
        return Err(
            "pyenv: self-uninstall requires confirmation; rerun with `pyenv self-uninstall --yes` for non-interactive use"
                .to_string(),
        );
    }

    println!(
        "WARNING: This will completely remove pyenv-native and ALL installed Python versions and virtual environments."
    );
    print!(
        "Are you sure you want to completely remove {} ? [y/N] ",
        ctx.root.display()
    );
    let _ = io::stdout().flush();

    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .map_err(|error| format!("pyenv: failed to read confirmation: {error}"))?;

    match answer.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => Ok(()),
        _ => Err("pyenv: self-uninstall cancelled".to_string()),
    }
}

fn spawn_windows_uninstall(ctx: &AppContext) -> Result<(), String> {
    let temp_dir = env::temp_dir().join(format!("pyenv-native-uninstall-{}", std::process::id()));
    fs::create_dir_all(&temp_dir)
        .map_err(|error| format!("pyenv: failed to create uninstall temp directory: {error}"))?;

    let launcher_path = temp_dir.join("run-self-uninstall.ps1");
    let launcher = render_windows_launcher(ctx);
    fs::write(&launcher_path, launcher)
        .map_err(|error| format!("pyenv: failed to write Windows uninstall helper: {error}"))?;

    use crate::process::PyenvCommandExt;
    Command::new("powershell.exe")
        .headless()
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            launcher_path.to_string_lossy().as_ref(),
            "-ParentPid",
            &std::process::id().to_string(),
        ])
        .spawn()
        .map_err(|error| format!("pyenv: failed to launch uninstall helper: {error}"))?;

    Ok(())
}

fn render_windows_launcher(ctx: &AppContext) -> String {
    let root_path = ctx.root.display().to_string().replace('\'', "''");

    // We remove the user path entries and the Pyenv root directory
    format!(
        "param([int]$ParentPid)\n\
         $ErrorActionPreference = 'Stop'\n\
         for ($attempt = 0; $attempt -lt 240; $attempt++) {{\n\
           if (-not (Get-Process -Id $ParentPid -ErrorAction SilentlyContinue)) {{ break }}\n\
           Start-Sleep -Milliseconds 500\n\
         }}\n\
         $root = '{root_path}'\n\
         $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')\n\
         if ($userPath) {{\n\
             $paths = $userPath -split ';' | Where-Object {{ $_ -and -not $_.StartsWith($root, [System.StringComparison]::OrdinalIgnoreCase) }}\n\
             [Environment]::SetEnvironmentVariable('Path', ($paths -join ';'), 'User')\n\
         }}\n\
         [Environment]::SetEnvironmentVariable('PYENV_ROOT', $null, 'User')\n\
         if (Test-Path $root) {{\n\
             Remove-Item -Recurse -Force $root -ErrorAction SilentlyContinue\n\
         }}\n\
         exit 0\n"
    )
}

fn run_posix_uninstall(ctx: &AppContext) -> Result<(), String> {
    if let Err(error) = fs::remove_dir_all(&ctx.root) {
        return Err(format!(
            "pyenv: failed to remove {}: {error}",
            ctx.root.display()
        ));
    }

    println!(
        "Please manually remove any pyenv initialization lines from your ~/.bashrc, ~/.zshrc, or other shell profiles."
    );
    Ok(())
}
