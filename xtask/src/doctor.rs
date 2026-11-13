// ./xtask/src/doctor.rs
//! `cargo xtask doctor`: check that this machine can build pyenv-native.
//!
//! Replaces `scripts/Check-Env.ps1`, now on every platform. Rust and Cargo are required.
//! Android cross-build tooling (rustup target, cargo-ndk, NDK) is advisory unless
//! `--require-android` is given, because CI builds the Android artifacts anyway.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Result, bail};
use clap::Args;

#[derive(Args)]
pub struct DoctorArgs {
    /// Fail when the Android cross-build prerequisites are incomplete.
    #[arg(long)]
    require_android: bool,
}

const ANDROID_TARGET: &str = "aarch64-linux-android";

#[derive(Clone, Copy)]
enum Level {
    Ok,
    Warn,
    Fail,
    Info,
}

fn say(level: Level, message: impl AsRef<str>) {
    let tag = match level {
        Level::Ok => "OK",
        Level::Warn => "WARN",
        Level::Fail => "FAIL",
        Level::Info => "INFO",
    };
    println!("[{tag}] {}", message.as_ref());
}

/// First non-empty line a tool prints for `args`, looking at stdout then stderr (`java -version`
/// writes to stderr). `None` when the program is missing or exits non-zero.
fn first_line(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

pub fn run(args: &DoctorArgs) -> Result<()> {
    println!("--- pyenv-native dev environment check ---");
    let mut ready = true;
    let mut android_ready = true;

    for (program, hint) in [
        ("cargo", "install Rust from https://rustup.rs"),
        ("rustc", "install Rust from https://rustup.rs"),
    ] {
        match first_line(program, &["--version"]) {
            Some(version) => say(Level::Ok, format!("{program} found: {version}")),
            None => {
                say(Level::Fail, format!("{program} not found: {hint}"));
                ready = false;
            }
        }
    }

    match first_line("git", &["--version"]) {
        Some(version) => say(Level::Ok, format!("git found: {version}")),
        None => say(
            Level::Warn,
            "git not found; `cargo xtask install-hooks` and releases need it.",
        ),
    }

    if let Some(version) = first_line("rustup", &["--version"]) {
        say(Level::Ok, format!("rustup found: {version}"));
        let installed = Command::new("rustup")
            .args(["target", "list", "--installed"])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
            .unwrap_or_default();
        if installed.lines().any(|line| line.trim() == ANDROID_TARGET) {
            say(
                Level::Ok,
                format!("Rust Android target installed: {ANDROID_TARGET}"),
            );
        } else {
            say(
                Level::Warn,
                format!(
                    "Rust Android target missing: run `rustup target add {ANDROID_TARGET}` \
                     to build Android artifacts locally."
                ),
            );
            android_ready = false;
        }
    } else {
        say(
            Level::Warn,
            "rustup not found; the Android target cannot be verified.",
        );
        android_ready = false;
    }

    match first_line("pyenv", &["--version"]) {
        Some(version) => say(Level::Info, format!("pyenv on PATH: {version}")),
        None => say(
            Level::Info,
            "pyenv is not on PATH. That is expected before you install it.",
        ),
    }

    match first_line("cargo-ndk", &["--version"]) {
        Some(version) => say(Level::Ok, format!("cargo-ndk found: {version}")),
        None => {
            say(
                Level::Warn,
                "cargo-ndk not found. Install it with `cargo install cargo-ndk --locked` \
                 for local Android bundle builds.",
            );
            android_ready = false;
        }
    }

    match find_ndk(&|name| std::env::var(name).ok()) {
        Some(path) => say(Level::Ok, format!("Android NDK found: {}", path.display())),
        None => {
            say(
                Level::Warn,
                "Android NDK not found. Set ANDROID_NDK_HOME or ANDROID_NDK_ROOT, or install \
                 the NDK under an Android SDK (ANDROID_HOME).",
            );
            android_ready = false;
        }
    }

    match first_line("java", &["-version"]) {
        Some(version) => say(Level::Ok, format!("Java found: {version}")),
        None => say(
            Level::Warn,
            "Java not found. Some Android SDK/NDK management workflows are easier with a JDK.",
        ),
    }

    if android_ready {
        say(Level::Ok, "Android cross-build prerequisites look ready.");
    } else if args.require_android {
        say(
            Level::Fail,
            "Android cross-build prerequisites are missing, and --require-android was given.",
        );
        ready = false;
    } else {
        say(
            Level::Info,
            "Android cross-build prerequisites are incomplete locally, but GitHub Actions \
             still builds the Android artifacts.",
        );
    }

    if !ready {
        bail!("the environment is missing prerequisites");
    }
    println!("--- Environment looks ready for development ---");
    Ok(())
}

/// Where an Android NDK could be, best guess first: the explicit variables, then the newest NDK
/// under each Android SDK location.
fn ndk_candidates(var: &dyn Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = ["ANDROID_NDK_HOME", "ANDROID_NDK_ROOT"]
        .into_iter()
        .filter_map(var)
        .map(PathBuf::from)
        .collect();

    let mut sdk_roots: Vec<PathBuf> = ["ANDROID_HOME", "ANDROID_SDK_ROOT"]
        .into_iter()
        .filter_map(var)
        .map(PathBuf::from)
        .collect();
    if let Some(local) = var("LOCALAPPDATA") {
        sdk_roots.push(PathBuf::from(local).join("Android").join("Sdk"));
    }
    if let Some(home) = var("HOME").or_else(|| var("USERPROFILE")) {
        let home = PathBuf::from(home);
        sdk_roots.push(home.join("Android").join("Sdk"));
        sdk_roots.push(home.join("Library").join("Android").join("sdk"));
    }

    for sdk in sdk_roots {
        let Ok(entries) = std::fs::read_dir(sdk.join("ndk")) else {
            continue;
        };
        let mut versions: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_dir())
            .collect();
        versions.sort();
        versions.reverse();
        candidates.extend(versions);
    }
    candidates
}

fn find_ndk(var: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    ndk_candidates(var)
        .into_iter()
        .find(|path| path.is_dir())
        .map(|path| path.canonicalize().unwrap_or(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;

    fn env_of(pairs: &[(&str, String)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.clone()))
            .collect()
    }

    #[test]
    fn explicit_ndk_variable_wins_over_sdk_discovery() {
        let dir = tempfile::tempdir().unwrap();
        let explicit = dir.path().join("my-ndk");
        fs::create_dir_all(&explicit).unwrap();
        fs::create_dir_all(dir.path().join("sdk/ndk/26.1.0")).unwrap();
        let vars = env_of(&[
            ("ANDROID_NDK_HOME", explicit.display().to_string()),
            ("ANDROID_HOME", dir.path().join("sdk").display().to_string()),
        ]);
        let found = find_ndk(&|name| vars.get(name).cloned()).unwrap();
        assert_eq!(found, explicit.canonicalize().unwrap());
    }

    #[test]
    fn newest_sdk_ndk_is_chosen() {
        let dir = tempfile::tempdir().unwrap();
        for version in ["25.2.9519653", "26.1.10909125", "27.0.12077973"] {
            fs::create_dir_all(dir.path().join("sdk/ndk").join(version)).unwrap();
        }
        let vars = env_of(&[("ANDROID_HOME", dir.path().join("sdk").display().to_string())]);
        let found = find_ndk(&|name| vars.get(name).cloned()).unwrap();
        assert!(found.ends_with("27.0.12077973"), "{}", found.display());
    }

    #[test]
    fn a_stale_variable_falls_through_to_a_real_ndk() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("sdk/ndk/26.1.0")).unwrap();
        let vars = env_of(&[
            (
                "ANDROID_NDK_HOME",
                dir.path().join("gone").display().to_string(),
            ),
            ("ANDROID_HOME", dir.path().join("sdk").display().to_string()),
        ]);
        let found = find_ndk(&|name| vars.get(name).cloned()).unwrap();
        assert!(found.ends_with("26.1.0"));
    }

    #[test]
    fn no_ndk_is_reported_as_none() {
        assert!(find_ndk(&|_| None).is_none());
    }

    #[test]
    fn a_missing_program_has_no_version_line() {
        assert!(first_line("pyenv-native-definitely-not-installed", &["--version"]).is_none());
    }
}
