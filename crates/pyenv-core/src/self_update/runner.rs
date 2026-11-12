// ./crates/pyenv-core/src/self_update/runner.rs
//! Self-update execution flow, installer download, and platform-specific launcher handling.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::process::PyenvCommandExt;

use super::bundle::stage_release_bundle;
use super::github::{DEFAULT_GITHUB_REPO, resolve_release_target};
use super::types::{ReleaseTarget, SelfUpdateCheck, SelfUpdateOptions};

pub fn cmd_self_update(ctx: &AppContext, options: &SelfUpdateOptions) -> CommandReport {
    match run_self_update(ctx, options) {
        Ok(lines) => CommandReport::success(lines),
        Err(message) => CommandReport::failure(vec![message], 1),
    }
}

pub fn inspect_self_update(
    ctx: &AppContext,
    options: &SelfUpdateOptions,
) -> Result<SelfUpdateCheck, String> {
    let repo = options
        .github_repo
        .clone()
        .unwrap_or_else(|| DEFAULT_GITHUB_REPO.to_string());
    ensure_portable_install(ctx, options.restart_gui)?;
    let target = resolve_release_target(&repo, options.tag.as_deref())?;
    Ok(SelfUpdateCheck::from_target(&target))
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

    let staged = stage_release_bundle(&repo, &target.target_tag)?;
    let current_exe = env::current_exe().unwrap_or_default();
    let bin_dir = ctx.root.join("bin");
    let hand_over_to_background = options.restart_gui || is_gui_launch(&current_exe, &bin_dir);

    let mut command = installer_command(ctx, &staged.pyenv_exe);
    if hand_over_to_background {
        // The running app must exit first; the staged installer waits for it, swaps the
        // binaries, and relaunches the GUI.
        command
            .arg("--wait-for-pid")
            .arg(std::process::id().to_string())
            .arg("--relaunch-gui")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command
            .spawn()
            .map_err(|error| format!("pyenv: failed to launch the staged installer: {error}"))?;
        return Ok(vec![
            format!(
                "Started pyenv-native update to {} for install root {}.",
                target.target_tag,
                ctx.root.display()
            ),
            "The application will close, apply the update, and relaunch automatically.".to_string(),
        ]);
    }

    let output = command
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("pyenv: failed to run the staged installer: {error}"));
    // The staged binary has exited, so its directory can go.
    let _ = fs::remove_dir_all(&staged.staging_dir);
    let output = output?;
    if !output.status.success() {
        return Err(installer_failure(&output));
    }
    Ok(vec![format!(
        "Updated pyenv-native to {} in {}.",
        target.target_tag,
        ctx.root.display()
    )])
}

/// `pyenv self-install` invocation that refreshes binaries in place without touching profiles.
fn installer_command(ctx: &AppContext, staged_pyenv: &Path) -> Command {
    let mut command = Command::new(staged_pyenv);
    command.headless();
    command
        .arg("self-install")
        .arg("--install-root")
        .arg(&ctx.root)
        .args(["--shell", "none"])
        .args(["--add-to-user-path", "false"])
        .args(["--update-shell-profile", "false"])
        .args(["--refresh-shims", "true"])
        .args(["--install-powershell7", "false"])
        .args(["--yes", "--force"]);
    command
}

fn installer_failure(output: &std::process::Output) -> String {
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
    details.join("\n")
}

fn ensure_portable_install(ctx: &AppContext, allow_gui_launcher: bool) -> Result<(), String> {
    let current_exe = env::current_exe()
        .map_err(|error| format!("pyenv: failed to resolve current executable path: {error}"))?;

    let bin_dir = ctx.root.join("bin");
    let is_pyenv = same_path(&current_exe, &portable_executable_path(&ctx.root));
    let is_gui = is_gui_launch(&current_exe, &bin_dir);
    if is_pyenv || is_gui {
        return Ok(());
    }

    if allow_gui_launcher && path_is_under(&current_exe, &ctx.root) {
        return Ok(());
    }

    Err({
        let mut args = HashMap::new();
        args.insert(
            "expected".into(),
            bin_dir.join("pyenv").display().to_string(),
        );
        args.insert("current".into(), current_exe.display().to_string());
        crate::lookup_i18n_args("error-self-update-portable", &args)
    })
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
    same_path(current_exe, &gui_executable_path(bin_dir))
}

fn path_is_under(path: &Path, root: &Path) -> bool {
    match (fs::canonicalize(path), fs::canonicalize(root)) {
        (Ok(resolved), Ok(root)) => resolved.starts_with(&root),
        _ => path.starts_with(root),
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
