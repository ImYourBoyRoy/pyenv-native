// ./crates/pyenv-core/src/self_update/runner.rs
//! Self-update execution flow, installer download, and platform-specific launcher handling.

use crate::process::PyenvCommandExt;
use std::cmp::Ordering;
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::http::build_blocking_client;

use super::github::{DEFAULT_GITHUB_REPO, resolve_release_target};
use super::types::{ReleaseTarget, SelfUpdateOptions};

pub fn cmd_self_update(ctx: &AppContext, options: &SelfUpdateOptions) -> CommandReport {
    match run_self_update(ctx, options) {
        Ok(lines) => CommandReport::success(lines),
        Err(message) => CommandReport::failure(vec![message], 1),
    }
}

fn run_self_update(ctx: &AppContext, options: &SelfUpdateOptions) -> Result<Vec<String>, String> {
    let repo = options
        .github_repo
        .clone()
        .unwrap_or_else(|| DEFAULT_GITHUB_REPO.to_string());
    ensure_portable_install(ctx, options.restart_gui)?;

    let target = resolve_release_target(&repo, options.tag.as_deref())?;
    if options.check {
        return Ok(render_check_lines(&target));
    }

    if target.comparison == Ordering::Equal && !options.force {
        return Ok(vec![format!(
            "pyenv-native {} is already up to date.",
            target.current_tag
        )]);
    }

    confirm_self_update(ctx, &target, options.yes, options.force)?;

    let installer_path = download_installer_script(&repo, &target.target_tag)?;
    if cfg!(windows) {
        spawn_windows_update(ctx, &target, &installer_path)?;
        Ok(vec![
            format!(
                "Started pyenv-native update to {} for install root {}.",
                target.target_tag,
                ctx.root.display()
            ),
            "Keep this shell open until the updater finishes.".to_string(),
        ])
    } else {
        let current_exe = env::current_exe().unwrap_or_default();
        let bin_dir = ctx.root.join("bin");
        let is_gui = is_gui_launch(&current_exe, &bin_dir) || options.restart_gui;
        if is_gui {
            spawn_posix_update(ctx, &target, &installer_path)?;
            Ok(vec![
                format!(
                    "Started pyenv-native update to {} for install root {}.",
                    target.target_tag,
                    ctx.root.display()
                ),
                "The application will close, apply the update, and relaunch automatically."
                    .to_string(),
            ])
        } else {
            run_posix_update(ctx, &target, &installer_path)?;
            Ok(vec![format!(
                "Updated pyenv-native to {} in {}.",
                target.target_tag,
                ctx.root.display()
            )])
        }
    }
}

fn ensure_portable_install(ctx: &AppContext, allow_gui_launcher: bool) -> Result<(), String> {
    let current_exe = env::current_exe()
        .map_err(|error| format!("pyenv: failed to resolve current executable path: {error}"))?;

    // We allow self-update to be triggered from either the CLI (pyenv) or the GUI companion (pyenv-gui)
    // provided they are running from the expected installation directory.
    let bin_dir = ctx.root.join("bin");

    let is_pyenv = same_path(&current_exe, &portable_executable_path(&ctx.root));
    let is_gui = is_gui_launch(&current_exe, &bin_dir);

    if is_pyenv || is_gui || allow_gui_launcher {
        return Ok(());
    }

    Err(format!(
        "pyenv: self-update only supports portable installs launched from `{}`; current executable is `{}`",
        bin_dir.join("pyenv").display(),
        current_exe.display()
    ))
}

fn portable_executable_path(root: &Path) -> PathBuf {
    let mut candidate = root.join("bin").join("pyenv");
    if cfg!(windows) {
        candidate.set_extension("exe");
    }
    candidate
}

fn gui_executable_path(bin_dir: &Path) -> PathBuf {
    let mut candidate = bin_dir.join("pyenv-gui");
    if cfg!(windows) {
        candidate.set_extension("exe");
    }
    candidate
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(a), Ok(b)) => a == b,
        _ => left == right,
    }
}

fn is_gui_launch(current_exe: &Path, bin_dir: &Path) -> bool {
    if same_path(current_exe, &gui_executable_path(bin_dir)) {
        return true;
    }

    current_exe
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            let lowered = name.to_ascii_lowercase();
            lowered == "pyenv-gui" || lowered == "pyenv-gui.exe"
        })
        .unwrap_or(false)
}

fn render_check_lines(target: &ReleaseTarget) -> Vec<String> {
    match target.comparison {
        Ordering::Less => vec![format!(
            "Installed version {} is newer than the latest published release {}.",
            target.current_tag, target.target_tag
        )],
        Ordering::Equal => vec![format!(
            "pyenv-native {} is up to date.",
            target.current_tag
        )],
        Ordering::Greater => {
            let mut lines = vec![format!(
                "Update available: {} (current {}).",
                target.target_tag, target.current_tag
            )];
            if let Some(url) = &target.release_url {
                lines.push(format!("Release: {url}"));
            }
            lines.push("Run `pyenv self-update --yes` to install it.".to_string());
            lines
        }
    }
}

fn confirm_self_update(
    ctx: &AppContext,
    target: &ReleaseTarget,
    yes: bool,
    force: bool,
) -> Result<(), String> {
    if yes {
        return Ok(());
    }

    if !io::stdin().is_terminal() {
        return Err(
            "pyenv: self-update requires confirmation; rerun with `pyenv self-update --yes` for non-interactive use"
                .to_string(),
        );
    }

    let action = if target.comparison == Ordering::Equal || force {
        "Reinstall"
    } else {
        "Update"
    };

    print!(
        "{action} pyenv-native {} under {}? [y/N] ",
        target.target_tag,
        ctx.root.display()
    );
    let _ = io::stdout().flush();

    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .map_err(|error| format!("pyenv: failed to read confirmation: {error}"))?;

    match answer.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => Ok(()),
        _ => Err("pyenv: self-update cancelled".to_string()),
    }
}

fn download_installer_script(repo: &str, tag: &str) -> Result<PathBuf, String> {
    let extension = if cfg!(windows) { "ps1" } else { "sh" };

    // Primary: Try fetching from the release assets (stable)
    let release_asset_url =
        format!("https://github.com/{repo}/releases/download/{tag}/install.{extension}");
    // Secondary: Fallback to raw GitHub content (useful for dev/main installs if assets are missing)
    let raw_url = format!("https://raw.githubusercontent.com/{repo}/{tag}/install.{extension}");

    let temp_dir = env::temp_dir().join(format!(
        "pyenv-native-self-update-{}-{}",
        std::process::id(),
        timestamp_suffix()
    ));
    fs::create_dir_all(&temp_dir)
        .map_err(|error| format!("pyenv: failed to create update temp directory: {error}"))?;

    let installer_path = temp_dir.join(format!("install.{extension}"));
    let client = build_blocking_client()
        .map_err(|error| format!("pyenv: failed to construct HTTP client: {error}"))?;

    // Try primary URL first
    let response = match client
        .get(&release_asset_url)
        .send()
        .and_then(|r| r.error_for_status())
    {
        Ok(r) => Ok(r),
        Err(e) => {
            // Fallback to raw URL
            client.get(&raw_url).send().and_then(|r| r.error_for_status()).map_err(|fallback_err| {
                format!("pyenv: failed to download installer from either {release_asset_url} ({e}) or {raw_url} ({fallback_err})")
            })
        }
    }?;

    let bytes = response
        .bytes()
        .map_err(|error| format!("pyenv: failed to read installer download: {error}"))?;
    fs::write(&installer_path, &bytes)
        .map_err(|error| format!("pyenv: failed to write installer script: {error}"))?;
    Ok(installer_path)
}

fn spawn_windows_update(
    ctx: &AppContext,
    target: &ReleaseTarget,
    installer_path: &Path,
) -> Result<(), String> {
    let current_exe = env::current_exe().unwrap_or_default();
    let bin_dir = ctx.root.join("bin");
    let is_gui = same_path(&current_exe, &gui_executable_path(&bin_dir));

    let launcher_path = installer_path.with_file_name("run-self-update.ps1");
    let launcher = render_windows_launcher(ctx, target, installer_path, is_gui);
    fs::write(&launcher_path, launcher)
        .map_err(|error| format!("pyenv: failed to write Windows updater helper: {error}"))?;

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
        .map_err(|error| format!("pyenv: failed to launch updater helper: {error}"))?;

    Ok(())
}

fn render_windows_launcher(
    ctx: &AppContext,
    target: &ReleaseTarget,
    installer_path: &Path,
    restart_gui: bool,
) -> String {
    let installer_args = vec![
        "-NoProfile".to_string(),
        "-ExecutionPolicy".to_string(),
        "Bypass".to_string(),
        "-File".to_string(),
        installer_path.display().to_string(),
        "-GitHubRepo".to_string(),
        target.repo.clone(),
        "-Tag".to_string(),
        target.target_tag.clone(),
        "-InstallRoot".to_string(),
        ctx.root.display().to_string(),
        "-Shell".to_string(),
        "none".to_string(),
        "-AddToUserPath".to_string(),
        "false".to_string(),
        "-UpdatePowerShellProfile".to_string(),
        "false".to_string(),
        "-RefreshShims".to_string(),
        "true".to_string(),
        "-Yes".to_string(),
        "-Force".to_string(),
    ];

    let rendered_args = installer_args
        .iter()
        .map(|arg| format!("'{}'", escape_powershell_single_quoted(arg)))
        .collect::<Vec<_>>()
        .join(", ");

    let restart_script = if restart_gui {
        let gui_exe = ctx.root.join("bin").join("pyenv-gui.exe");
        format!(
            "\n$guiExe = '{}'\nif (Test-Path $guiExe) {{\n  Start-Process -FilePath $guiExe\n}}\n",
            escape_powershell_single_quoted(&gui_exe.display().to_string())
        )
    } else {
        String::new()
    };

    format!(
        "param([int]$ParentPid)\n\
         $ErrorActionPreference = 'Stop'\n\
         for ($attempt = 0; $attempt -lt 240; $attempt++) {{\n\
           if (-not (Get-Process -Id $ParentPid -ErrorAction SilentlyContinue)) {{ break }}\n\
           Start-Sleep -Milliseconds 500\n\
         }}\n\
         $installerArgs = @({rendered_args})\n\
         & powershell.exe @installerArgs\n\
         {restart_script}\n\
         exit $LASTEXITCODE\n"
    )
}

fn escape_powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}

fn run_posix_update(
    ctx: &AppContext,
    target: &ReleaseTarget,
    installer_path: &Path,
) -> Result<(), String> {
    let output = posix_installer_command(ctx, target, installer_path)
        .output()
        .map_err(|error| format!("pyenv: failed to launch installer: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    posix_installer_failure(output)
}

fn spawn_posix_update(
    ctx: &AppContext,
    target: &ReleaseTarget,
    installer_path: &Path,
) -> Result<(), String> {
    let launcher_path = installer_path.with_file_name("run-self-update.sh");
    let gui_exe = gui_executable_path(&ctx.root.join("bin"));
    let launcher = render_posix_launcher(ctx, target, installer_path, &gui_exe);
    fs::write(&launcher_path, launcher)
        .map_err(|error| format!("pyenv: failed to write POSIX updater helper: {error}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&launcher_path, fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("pyenv: failed to mark updater helper executable: {error}"))?;
    }

    Command::new("sh")
        .headless()
        .arg(&launcher_path)
        .arg(std::process::id().to_string())
        .spawn()
        .map_err(|error| format!("pyenv: failed to launch updater helper: {error}"))?;
    Ok(())
}

fn posix_installer_command(
    ctx: &AppContext,
    target: &ReleaseTarget,
    installer_path: &Path,
) -> Command {
    let mut command = Command::new("sh");
    command.headless();
    command
        .arg(installer_path)
        .arg("--github-repo")
        .arg(&target.repo)
        .arg("--tag")
        .arg(&target.target_tag)
        .arg("--install-root")
        .arg(&ctx.root)
        .arg("--shell")
        .arg("none")
        .arg("--add-to-user-path")
        .arg("false")
        .arg("--update-shell-profile")
        .arg("false")
        .arg("--refresh-shims")
        .arg("true")
        .arg("--yes")
        .arg("--force");
    command
}

fn posix_installer_failure(output: std::process::Output) -> Result<(), String> {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut details = vec![format!(
        "pyenv: self-update installer exited with status {:?}",
        output.status.code()
    )];
    if !stderr.trim().is_empty() {
        details.push(format!("pyenv: installer stderr:\n{}", stderr.trim()));
    }
    if !stdout.trim().is_empty() {
        details.push(format!("pyenv: installer stdout:\n{}", stdout.trim()));
    }
    Err(details.join("\n"))
}

fn render_posix_launcher(
    ctx: &AppContext,
    target: &ReleaseTarget,
    installer_path: &Path,
    gui_exe: &Path,
) -> String {
    let installer = shell_single_quote(&installer_path.display().to_string());
    let gui = shell_single_quote(&gui_exe.display().to_string());
    let root = shell_single_quote(&ctx.root.display().to_string());
    let repo = shell_single_quote(&target.repo);
    let tag = shell_single_quote(&target.target_tag);
    let log_file = shell_single_quote(
        &ctx.root
            .join("logs")
            .join("gui-relaunch.log")
            .display()
            .to_string(),
    );
    let app_id = shell_single_quote("com.pyenv-native.gui");

    format!(
        "#!/bin/sh\n\
         set -eu\n\
         PARENT_PID=\"$1\"\n\
         LOG_FILE={log_file}\n\
         mkdir -p \"$(dirname \"$LOG_FILE\")\"\n\
         {{\n\
           printf '%%s pyenv-native GUI updater started (parent pid %%s)\\n' \"$(date -u '+%Y-%m-%dT%H:%M:%SZ')\" \"$PARENT_PID\"\n\
         }} >>\"$LOG_FILE\" 2>&1\n\
         attempt=0\n\
         while [ \"$attempt\" -lt 240 ]; do\n\
           if ! kill -0 \"$PARENT_PID\" 2>/dev/null; then\n\
             break\n\
           fi\n\
           attempt=$((attempt + 1))\n\
           sleep 0.5\n\
         done\n\
         if sh {installer} \\\n\
           --github-repo {repo} \\\n\
           --tag {tag} \\\n\
           --install-root {root} \\\n\
           --shell none \\\n\
           --add-to-user-path false \\\n\
           --update-shell-profile false \\\n\
           --refresh-shims true \\\n\
           --yes \\\n\
           --force >>\"$LOG_FILE\" 2>&1; then\n\
           printf '%%s installer finished successfully\\n' \"$(date -u '+%Y-%m-%dT%H:%M:%SZ')\" >>\"$LOG_FILE\" 2>&1\n\
         else\n\
           status=$?\n\
           printf '%%s installer failed with exit code %%s\\n' \"$(date -u '+%Y-%m-%dT%H:%M:%SZ')\" \"$status\" >>\"$LOG_FILE\" 2>&1\n\
           exit \"$status\"\n\
         fi\n\
         relaunch_gui() {{\n\
           if [ ! -x {gui} ]; then\n\
             printf '%%s GUI binary missing or not executable: {gui}\\n' \"$(date -u '+%Y-%m-%dT%H:%M:%SZ')\" >>\"$LOG_FILE\" 2>&1\n\
             return 1\n\
           fi\n\
           if command -v gtk-launch >/dev/null 2>&1; then\n\
             if DISPLAY=\"${{DISPLAY:-}}\" WAYLAND_DISPLAY=\"${{WAYLAND_DISPLAY:-}}\" XDG_RUNTIME_DIR=\"${{XDG_RUNTIME_DIR:-}}\" DBUS_SESSION_BUS_ADDRESS=\"${{DBUS_SESSION_BUS_ADDRESS:-}}\" gtk-launch {app_id} >>\"$LOG_FILE\" 2>&1 & then\n\
               printf '%%s relaunched GUI via gtk-launch\\n' \"$(date -u '+%Y-%m-%dT%H:%M:%SZ')\" >>\"$LOG_FILE\" 2>&1\n\
               return 0\n\
             fi\n\
           fi\n\
           DISPLAY=\"${{DISPLAY:-}}\" WAYLAND_DISPLAY=\"${{WAYLAND_DISPLAY:-}}\" XDG_RUNTIME_DIR=\"${{XDG_RUNTIME_DIR:-}}\" DBUS_SESSION_BUS_ADDRESS=\"${{DBUS_SESSION_BUS_ADDRESS:-}}\" nohup {gui} >>\"$LOG_FILE\" 2>&1 &\n\
           printf '%%s relaunched GUI via nohup\\n' \"$(date -u '+%Y-%m-%dT%H:%M:%SZ')\" >>\"$LOG_FILE\" 2>&1\n\
         }}\n\
         relaunch_gui\n"
    )
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn timestamp_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
