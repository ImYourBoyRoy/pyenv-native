// ./xtask/src/check.rs
//! `cargo xtask check`: the local and pre-push quality gate.
//!
//! Runs format, clippy, tests, version sync, repository policy checks, the Python bootstrap tests,
//! and (when the tools exist) the GUI accessibility check and `cargo audit`. Steps that need an
//! optional tool are skipped with a notice unless `--strict` is given.

use std::path::Path;
use std::process::Command;

use anyhow::{Result, bail};
use clap::Args;

use crate::{policy, repo_root, version};

#[derive(Args)]
pub struct CheckArgs {
    /// Fail instead of skipping when an optional tool (pnpm, cargo-audit, sh, python) is missing.
    #[arg(long)]
    strict: bool,
    /// Skip the slower GUI accessibility (pnpm) and `cargo audit` steps.
    #[arg(long)]
    quick: bool,
    /// Also build `pyenv` and run the platform's shell smoke test (`scripts/smoke-shells.*`).
    #[arg(long)]
    smoke: bool,
}

fn available(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn heading(title: &str) {
    println!("\n--- {title} ---");
}

fn step(root: &Path, program: &str, args: &[&str]) -> Result<()> {
    run_in(root, program, args, &[])
}

fn run_in(dir: &Path, program: &str, args: &[&str], env: &[(&str, &str)]) -> Result<()> {
    let mut command = Command::new(program);
    command.args(args).current_dir(dir);
    for (key, value) in env {
        command.env(key, value);
    }
    let status = command.status()?;
    if !status.success() {
        bail!("`{program} {}` failed ({status})", args.join(" "));
    }
    Ok(())
}

fn skip_or_fail(strict: bool, what: &str, hint: &str) -> Result<()> {
    if strict {
        bail!("{what} is required in --strict mode ({hint})");
    }
    println!("skipping: {what} not found ({hint})");
    Ok(())
}

fn python() -> Option<&'static str> {
    ["python3", "python"]
        .into_iter()
        .find(|candidate| available(candidate))
}

pub fn run_checks(args: &CheckArgs) -> Result<()> {
    let root = repo_root();

    heading("Format");
    step(&root, "cargo", &["fmt", "--all", "--check"])?;

    heading("Clippy (workspace)");
    step(
        &root,
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;

    heading("Tests (workspace)");
    step(&root, "cargo", &["test", "--workspace"])?;

    heading("Version sync");
    version::check(&version::CheckArgs::default())?;

    heading("Repository policy checks");
    policy::run_in(&root, &[], args.strict)?;

    if args.smoke {
        heading("Shell smoke tests");
        step(&root, "cargo", &["build", "-p", "pyenv-cli"])?;
        if cfg!(windows) {
            step(
                &root,
                "pwsh",
                &[
                    "-NoLogo",
                    "-NoProfile",
                    "-File",
                    "./scripts/smoke-shells.ps1",
                ],
            )?;
        } else {
            step(
                &root,
                "bash",
                &["./scripts/smoke-shells.sh", "./target/debug/pyenv"],
            )?;
        }
    }

    heading("Python bootstrap tests");
    match python() {
        Some(py) => run_in(
            &root,
            py,
            &[
                "-m",
                "unittest",
                "discover",
                "-s",
                "./python-package/tests",
                "-p",
                "test_*.py",
            ],
            &[("PYTHONPATH", "./python-package/src")],
        )?,
        None => skip_or_fail(args.strict, "python", "needed for python-package tests")?,
    }

    if args.quick {
        println!("\nskipping GUI accessibility and cargo audit (--quick)");
    } else {
        heading("GUI accessibility (pnpm)");
        if available("pnpm") {
            let a11y = root.join("scripts").join("gui-a11y");
            step(&a11y, "pnpm", &["install", "--frozen-lockfile"])?;
            step(&a11y, "pnpm", &["audit", "--prod"])?;
            step(&a11y, "pnpm", &["run", "check"])?;
        } else {
            skip_or_fail(
                args.strict,
                "pnpm",
                "corepack enable && corepack prepare pnpm@latest --activate",
            )?;
        }

        heading("cargo audit");
        if available("cargo-audit") {
            step(&root, "cargo", &["audit"])?;
        } else {
            skip_or_fail(
                args.strict,
                "cargo-audit",
                "cargo install cargo-audit --locked",
            )?;
        }
    }

    println!("\n--- All checks PASSED ---");
    Ok(())
}

pub fn run(args: &CheckArgs) -> Result<()> {
    run_checks(args)
}
