// ./xtask/src/release.rs
//! `cargo xtask release <version>`: local preflight, tagging, and pushes that trigger the
//! workflow-driven GitHub release.
//!
//! The public GitHub Release itself is produced by `.github/workflows/release.yml` once the
//! pushed tag lands. Every mutating step honours `--dry-run`.

use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::Args;

use crate::repo_root;

#[derive(Args)]
pub struct ReleaseArgs {
    /// Release version, with or without a leading `v`.
    version: String,
    #[arg(long, default_value = "origin")]
    remote: String,
    /// Python interpreter for the install-package checks.
    #[arg(long, env = "XTASK_PYTHON")]
    python: Option<String>,
    /// `owner/repo` (default: derived from the git remote).
    #[arg(long)]
    github_repo: Option<String>,
    /// Rewrite every manifest to this version first (`cargo xtask set-version`).
    #[arg(long)]
    sync_version: bool,
    #[arg(long)]
    skip_validation: bool,
    #[arg(long)]
    skip_build: bool,
    #[arg(long)]
    push_branch: bool,
    #[arg(long)]
    push_tag: bool,
    /// Follow the GitHub Actions run with `gh run watch`.
    #[arg(long)]
    watch: bool,
    #[arg(long)]
    allow_dirty: bool,
    /// Print mutating commands instead of running them.
    #[arg(long)]
    dry_run: bool,
}

/// `1.2.3` → `v1.2.3`; `v1.2.3` is unchanged.
pub fn release_tag(version: &str) -> String {
    if version.starts_with('v') {
        version.to_string()
    } else {
        format!("v{version}")
    }
}

/// `owner/repo` from `git@github.com:owner/repo.git`, `https://github.com/owner/repo(.git)`, …
pub fn github_repo_from_url(url: &str) -> Option<String> {
    let rest = url.trim().split("github.com").nth(1)?;
    let rest = rest.trim_start_matches([':', '/']).trim_end_matches('/');
    let rest = rest.strip_suffix(".git").unwrap_or(rest);
    let mut parts = rest.split('/');
    let (owner, repo) = (parts.next()?, parts.next()?);
    (!owner.is_empty() && !repo.is_empty() && parts.next().is_none())
        .then(|| format!("{owner}/{repo}"))
}

struct Runner {
    dry_run: bool,
}

impl Runner {
    fn mutate(&self, program: &str, args: &[&str]) -> Result<()> {
        let rendered = format!("{program} {}", args.join(" "));
        if self.dry_run {
            println!("DRY-RUN: {rendered}");
            return Ok(());
        }
        let status = Command::new(program)
            .args(args)
            .current_dir(repo_root())
            .status()
            .with_context(|| format!("failed to start `{program}`"))?;
        if !status.success() {
            bail!("`{rendered}` failed ({status})");
        }
        Ok(())
    }
}

fn git_output(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root())
        .output()
        .context("failed to start git")?;
    if !output.status.success() {
        bail!("`git {}` failed", args.join(" "));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn step(message: &str) {
    println!("[release] {message}");
}

pub fn run(args: &ReleaseArgs) -> Result<()> {
    let tag = release_tag(&args.version);
    let bare = tag.trim_start_matches('v').to_string();
    let runner = Runner {
        dry_run: args.dry_run,
    };

    if !repo_root().join(".git").exists() {
        bail!("this directory is not a git checkout; clone the final GitHub remote first");
    }
    let repo = args.github_repo.clone().or_else(|| {
        git_output(&["remote", "get-url", &args.remote])
            .ok()
            .and_then(|url| github_repo_from_url(&url))
    });
    if !args.allow_dirty && !git_output(&["status", "--porcelain"])?.is_empty() {
        bail!(
            "git working tree is not clean; commit or stash local changes (or pass --allow-dirty)"
        );
    }

    if args.sync_version {
        step(&format!("syncing every manifest to {bare}"));
        runner.mutate("cargo", &["xtask", "set-version", &bare])?;
    }

    // Validation and builds are read-only with respect to git, so they run even in --dry-run
    // unless skipped; this makes a dry run a real preflight.
    let python_flag: Vec<String> = args
        .python
        .iter()
        .flat_map(|p| ["--python".to_string(), p.clone()])
        .collect();
    let python_flag: Vec<&str> = python_flag.iter().map(String::as_str).collect();
    if !args.skip_validation {
        step("running the quality gate");
        run_always("cargo", &["xtask", "check", "--quick"])?;
        step("running the Python install-package tests");
        let mut test = vec!["xtask", "python", "test"];
        test.extend(&python_flag);
        run_always("cargo", &test)?;
    }
    if !args.skip_build {
        step("building local release artifacts for a final sanity check");
        run_always("cargo", &["xtask", "bundle", "--output-root", "dist"])?;
        let mut build = vec!["xtask", "python", "build"];
        build.extend(&python_flag);
        run_always("cargo", &build)?;
        step("the Windows setup is built by the release workflow");
    }

    if git_output(&["tag", "--list", &tag])?.is_empty() {
        step(&format!("creating git tag {tag}"));
        runner.mutate("git", &["tag", "-a", &tag, "-m", &format!("Release {tag}")])?;
    } else {
        step(&format!(
            "git tag {tag} already exists; leaving it in place"
        ));
    }
    if args.push_branch {
        let branch = git_output(&["branch", "--show-current"])?;
        if branch.is_empty() {
            bail!("unable to determine the current git branch for push");
        }
        step(&format!("pushing branch {branch} to {}", args.remote));
        runner.mutate("git", &["push", &args.remote, &branch])?;
    }
    if args.push_tag {
        step(&format!("pushing tag {tag} to {}", args.remote));
        runner.mutate("git", &["push", &args.remote, &tag])?;
    }
    if args.watch {
        step("watching the latest GitHub Actions run");
        match &repo {
            Some(repo) => runner.mutate("gh", &["run", "watch", "--repo", repo])?,
            None => runner.mutate("gh", &["run", "watch"])?,
        }
    }

    println!(
        "version: {bare}\ntag: {tag}\ngithub_repo: {}\ndry_run: {}",
        repo.as_deref().unwrap_or("(unknown)"),
        args.dry_run
    );
    Ok(())
}

fn run_always(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .current_dir(repo_root())
        .status()
        .with_context(|| format!("failed to start `{program}`"))?;
    if !status.success() {
        bail!("`{program} {}` failed ({status})", args.join(" "));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_normalised() {
        assert_eq!(release_tag("1.2.3"), "v1.2.3");
        assert_eq!(release_tag("v1.2.3"), "v1.2.3");
    }

    #[test]
    fn github_repos_parse_from_common_remote_urls() {
        for url in [
            "git@github.com:imyourboyroy/pyenv-native.git",
            "https://github.com/imyourboyroy/pyenv-native.git",
            "https://github.com/imyourboyroy/pyenv-native",
            "https://github.com/imyourboyroy/pyenv-native/",
            "ssh://git@github.com/imyourboyroy/pyenv-native.git",
        ] {
            assert_eq!(
                github_repo_from_url(url).as_deref(),
                Some("imyourboyroy/pyenv-native"),
                "{url}"
            );
        }
        assert_eq!(github_repo_from_url("https://gitlab.com/a/b.git"), None);
        assert_eq!(github_repo_from_url("https://github.com/only-owner"), None);
    }
}
