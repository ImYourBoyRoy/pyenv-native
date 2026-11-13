// ./xtask/src/gui.rs
//! `cargo xtask gui`: build and launch the desktop GUI for local testing.
//!
//! Replaces `scripts/launch_gui.ps1`, now on every platform: stop a running `pyenv-gui`, build
//! the crate, and start the fresh binary without waiting for it to exit.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use clap::Args;

use crate::repo_root;

#[derive(Args)]
pub struct GuiArgs {
    /// Build and launch the release profile instead of debug.
    #[arg(long)]
    release: bool,
}

const GUI_PACKAGE: &str = "pyenv-gui";

pub fn run(args: &GuiArgs) -> Result<()> {
    let root = repo_root();
    let profile = if args.release { "release" } else { "debug" };

    stop_running();

    println!("[1/2] Building {GUI_PACKAGE} ({profile})...");
    let mut build = Command::new("cargo");
    build
        .args(["build", "--package", GUI_PACKAGE])
        .current_dir(&root);
    if args.release {
        build.arg("--release");
    }
    let status = build.status().context("running cargo")?;
    if !status.success() {
        bail!("building {GUI_PACKAGE} failed ({status})");
    }

    let exe = gui_exe(&root, profile);
    println!("[2/2] Launching {}...", exe.display());
    Command::new(&exe)
        .current_dir(&root)
        .stdin(Stdio::null())
        .spawn()
        .with_context(|| format!("launching {}", exe.display()))?;
    println!("GUI is running. Close its window to exit.");
    Ok(())
}

fn gui_exe(root: &Path, profile: &str) -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    target
        .join(profile)
        .join(format!("{GUI_PACKAGE}{}", std::env::consts::EXE_SUFFIX))
}

/// Stop a previous instance so the linker can replace the binary. Nothing running is not an error.
fn stop_running() {
    let (program, args): (&str, Vec<String>) = if cfg!(windows) {
        (
            "taskkill",
            vec!["/IM".into(), format!("{GUI_PACKAGE}.exe"), "/F".into()],
        )
    } else {
        ("pkill", vec!["-x".into(), GUI_PACKAGE.into()])
    };
    let _ = Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_path_follows_the_profile_and_platform() {
        let root = Path::new("/repo");
        let exe = gui_exe(root, "release");
        let name = exe.file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(name, format!("pyenv-gui{}", std::env::consts::EXE_SUFFIX));
        assert!(exe.components().any(|part| part.as_os_str() == "release"));
    }
}
