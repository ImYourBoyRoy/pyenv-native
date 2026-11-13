// ./xtask/src/hooks.rs
//! `cargo xtask install-hooks`: point this clone at the repo-local git hooks.
//!
//! Replaces `scripts/install-git-hooks.sh`. The `pre-push` hook in `.githooks/` runs
//! `cargo xtask check`, so a push cannot skip the quality gate by accident.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::repo_root;

const HOOKS_DIR: &str = ".githooks";
const PRE_PUSH: &str = "pre-push";

pub fn run() -> Result<()> {
    install(&repo_root())?;
    println!("Git hooksPath set to {HOOKS_DIR} (pre-push runs cargo xtask check).");
    Ok(())
}

pub fn install(root: &Path) -> Result<()> {
    let hook = root.join(HOOKS_DIR).join(PRE_PUSH);
    if !hook.is_file() {
        bail!("missing {}", hook.display());
    }
    make_executable(&hook)?;

    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["config", "core.hooksPath", HOOKS_DIR])
        .status()
        .context("running git (is it installed?)")?;
    if !status.success() {
        bail!("`git config core.hooksPath {HOOKS_DIR}` failed ({status})");
    }
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(permissions.mode() | 0o111);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

// Git for Windows runs hooks through its own `sh`, which ignores the executable bit.
#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn sets_hooks_path_and_marks_the_hook_executable() {
        if !git_available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(root)
                .args(["init", "--quiet"])
                .status()
                .unwrap()
                .success()
        );
        fs::create_dir_all(root.join(HOOKS_DIR)).unwrap();
        fs::write(root.join(HOOKS_DIR).join(PRE_PUSH), "#!/bin/sh\n").unwrap();

        install(root).unwrap();

        let configured = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["config", "core.hooksPath"])
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&configured.stdout).trim(),
            HOOKS_DIR
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(root.join(HOOKS_DIR).join(PRE_PUSH))
                .unwrap()
                .permissions()
                .mode();
            assert_ne!(mode & 0o111, 0);
        }
    }

    #[test]
    fn refuses_a_checkout_without_the_hook() {
        let dir = tempfile::tempdir().unwrap();
        let error = install(dir.path()).unwrap_err().to_string();
        assert!(error.contains("missing"), "{error}");
    }
}
