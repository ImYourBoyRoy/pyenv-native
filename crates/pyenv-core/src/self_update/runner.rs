// ./crates/pyenv-core/src/self_update/runner.rs
//! Self-update execution flow, installer download, and platform-specific launcher handling.

use std::cmp::Ordering;
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::command::CommandReport;
use crate::context::AppContext;

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
    ensure_portable_install(ctx)?;

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
        run_posix_update(ctx, &target, &installer_path)?;
        Ok(vec![format!(
            "Updated pyenv-native to {} in {}.",
            target.target_tag,
            ctx.root.display()
        )])
    }
}

fn ensure_portable_install(ctx: &AppContext) -> Result<(), String> {
    let current_exe = env::current_exe()
        .map_err(|error| format!("pyenv: failed to resolve current executable path: {error}"))?;
    let expected = portable_executable_path(&ctx.root);
    if same_path(&current_exe, &expected) {
        return Ok(());
    }

    Err(format!(
        "pyenv: self-update only supports portable installs launched from `{}`; current executable is `{}`",
        expected.display(),
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

fn same_path(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(a), Ok(b)) => a == b,
        _ => left == right,
    }
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
    let installer_url =
        format!("https://raw.githubusercontent.com/{repo}/{tag}/install.{extension}");

    let temp_dir = env::temp_dir().join(format!(
        "pyenv-native-self-update-{}-{}",
        std::process::id(),
        timestamp_suffix()
    ));
    fs::create_dir_all(&temp_dir)
        .map_err(|error| format!("pyenv: failed to create update temp directory: {error}"))?;

    let installer_path = temp_dir.join(format!("install.{extension}"));
    let bytes = reqwest::blocking::Client::builder()
        .user_agent(format!("pyenv-native/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("pyenv: failed to construct HTTP client: {error}"))?
        .get(&installer_url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| {
            format!("pyenv: failed to download installer from {installer_url}: {error}")
        })?
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
    let launcher_path = installer_path.with_file_name("run-self-update.ps1");
    let launcher = render_windows_launcher(ctx, target, installer_path);
    fs::write(&launcher_path, launcher)
        .map_err(|error| format!("pyenv: failed to write Windows updater helper: {error}"))?;

    Command::new("powershell.exe")
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
) -> String {
    let mut installer_args = vec![
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
    ];

    if target.comparison == Ordering::Equal {
        installer_args.push("-Force".to_string());
    }

    let rendered_args = installer_args
        .iter()
        .map(|arg| format!("'{}'", escape_powershell_single_quoted(arg)))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "param([int]$ParentPid)\n\
         $ErrorActionPreference = 'Stop'\n\
         for ($attempt = 0; $attempt -lt 240; $attempt++) {{\n\
           if (-not (Get-Process -Id $ParentPid -ErrorAction SilentlyContinue)) {{ break }}\n\
           Start-Sleep -Milliseconds 500\n\
         }}\n\
         $installerArgs = @({rendered_args})\n\
         & powershell.exe @installerArgs\n\
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
    let mut command = Command::new("sh");
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
        .arg("--yes");

    let status = command
        .status()
        .map_err(|error| format!("pyenv: failed to launch installer: {error}"))?;
    if status.success() {
        return Ok(());
    }

    Err(format!(
        "pyenv: self-update installer exited with status {:?}",
        status.code()
    ))
}

fn timestamp_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
