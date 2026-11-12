// ./xtask/src/main.rs
//! Repository automation for pyenv-native, run as `cargo xtask <command>`.
//!
//! One cross-platform implementation replaces the former pairs of `.sh`/`.ps1` scripts:
//!
//! * `bundle`        build release binaries and assemble the distributable archive
//! * `version-check` verify every manifest carries the same version
//! * `set-version`   rewrite every manifest to a new version
//! * `package`       Winget manifests and the Homebrew formula from release bundles
//! * `python`        test, build, and publish the pip/pipx install package
//! * `release`       preflight, tag, and push to trigger the GitHub release workflow
//! * `sync-known-versions`  refresh the offline CPython name seed from an upstream pyenv clone
//! * `policy`        repository policy checks (shell EOL, GUI patterns, i18n catalogs)
//! * `check`         the local and pre-push quality gate

mod bundle;
mod check;
mod known_versions;
mod package;
mod policy;
mod python;
mod release;
mod version;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask", about = "pyenv-native repository automation", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
enum Command {
    /// Build release binaries and assemble the distributable bundle (.tar.gz or .zip) with a
    /// SHA-256 sidecar.
    Bundle(bundle::BundleArgs),
    /// Verify Cargo, Python package, bootstrap, and Tauri versions agree.
    VersionCheck(version::CheckArgs),
    /// Set the project version in every manifest.
    SetVersion(version::SetArgs),
    /// Generate Winget manifests or a Homebrew formula from release bundles.
    #[command(subcommand)]
    Package(package::PackageCommand),
    /// Test, build, or publish the pip/pipx install package.
    #[command(subcommand)]
    Python(python::PythonCommand),
    /// Preflight, tag, and push a release (the GitHub workflow publishes it).
    Release(release::ReleaseArgs),
    /// Refresh the offline known-versions seed from an upstream pyenv clone.
    SyncKnownVersions(known_versions::SyncArgs),
    /// Run repository policy checks: shell line endings, GUI patterns, i18n catalogs.
    Policy(policy::PolicyArgs),
    /// Run the local quality gate: format, clippy, tests, version sync, i18n, Python, GUI checks.
    Check(check::CheckArgs),
}

/// The repository root (the directory that holds the workspace `Cargo.toml`).
pub(crate) fn repo_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("xtask lives one level below the repository root")
        .to_path_buf()
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result: Result<()> = match cli.command {
        Command::Bundle(args) => bundle::run(&args),
        Command::VersionCheck(args) => version::check(&args),
        Command::SetVersion(args) => version::set(&args),
        Command::Package(command) => package::run(&command),
        Command::Python(command) => python::run(&command),
        Command::Release(args) => release::run(&args),
        Command::SyncKnownVersions(args) => known_versions::run(&args),
        Command::Policy(args) => policy::run(&args),
        Command::Check(args) => check::run(&args),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
