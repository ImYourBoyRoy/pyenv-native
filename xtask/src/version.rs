// ./xtask/src/version.rs
//! Version synchronisation across the Rust workspace, Python package, and Tauri config.
//!
//! These four files must always agree: a mismatch once left PyPI stuck while GitHub tags advanced.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Args;

use crate::repo_root;

#[derive(Args, Default)]
pub struct CheckArgs {
    /// Also require the shared version to equal this value (for tag-driven releases).
    #[arg(long, env = "EXPECTED_VERSION")]
    expected: Option<String>,
}

#[derive(Args)]
pub struct SetArgs {
    /// New semantic version, such as 0.7.0 or 0.7.0-rc.1.
    version: String,
}

/// One manifest and the line that carries its version.
struct Target {
    label: &'static str,
    path: PathBuf,
    /// Text that starts the matching line, up to the opening quote of the version.
    prefix: &'static str,
    /// Whether `prefix` includes exact leading indentation (JSON) rather than a top-level key.
    indent_exact: bool,
}

fn targets() -> Vec<Target> {
    let root = repo_root();
    vec![
        Target {
            label: "Cargo.toml",
            path: root.join("Cargo.toml"),
            prefix: "version = ",
            indent_exact: false,
        },
        Target {
            label: "python-package/pyproject.toml",
            path: root.join("python-package").join("pyproject.toml"),
            prefix: "version = ",
            indent_exact: false,
        },
        Target {
            label: "python-package __init__.__version__",
            path: root
                .join("python-package")
                .join("src")
                .join("pyenv_native_bootstrap")
                .join("__init__.py"),
            prefix: "__version__ = ",
            indent_exact: false,
        },
        Target {
            label: "crates/pyenv-gui/tauri.conf.json",
            path: root
                .join("crates")
                .join("pyenv-gui")
                .join("tauri.conf.json"),
            prefix: "  \"version\": ",
            indent_exact: true,
        },
    ]
}

/// Find the first line that starts with `target.prefix` followed by a quoted value.
fn locate(text: &str, target: &Target) -> Option<(usize, usize)> {
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let candidate = if target.indent_exact {
            line.starts_with(target.prefix)
        } else {
            line.trim_start().starts_with(target.prefix) && !line.starts_with(char::is_whitespace)
        };
        if candidate {
            let after = &line[line.find(target.prefix).unwrap() + target.prefix.len()..];
            if let Some(open) = after.strip_prefix('"')
                && let Some(close) = open.find('"')
            {
                let start = offset + line.len() - after.len() + 1;
                return Some((start, start + close));
            }
        }
        offset += line.len();
    }
    None
}

fn read_version(target: &Target) -> Result<String> {
    let text = fs::read_to_string(&target.path)
        .with_context(|| format!("cannot read {}", target.path.display()))?;
    let (start, end) = locate(&text, target)
        .with_context(|| format!("could not read the version from {}", target.label))?;
    Ok(text[start..end].to_string())
}

pub fn check(args: &CheckArgs) -> Result<()> {
    let mut versions = Vec::new();
    for target in targets() {
        versions.push((target.label, read_version(&target)?));
    }
    let shared = versions[0].1.clone();
    if versions.iter().any(|(_, version)| *version != shared) {
        let mut message = String::from("version mismatch across release metadata:\n");
        for (label, version) in &versions {
            message.push_str(&format!("  {label:<40} {version}\n"));
        }
        message.push_str("Fix with: cargo xtask set-version <semver>");
        bail!(message);
    }
    if let Some(expected) = args.expected.as_deref().filter(|value| !value.is_empty())
        && shared != expected
    {
        bail!("workspace version {shared} does not match expected {expected}");
    }
    println!("Version sync OK: {shared}");
    Ok(())
}

/// Accept `X.Y.Z` optionally followed by a `-`/`.` pre-release or build suffix.
fn is_semver_like(value: &str) -> bool {
    let (core, suffix) = split_core(value);
    let parts: Vec<&str> = core.split('.').collect();
    let numeric = parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()));
    numeric
        && suffix.is_none_or(|rest| {
            !rest.is_empty()
                && rest
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')
        })
}

/// Split `1.2.3-rc.1` into (`1.2.3`, Some(`rc.1`)).
fn split_core(value: &str) -> (&str, Option<&str>) {
    let mut dots = 0;
    for (index, ch) in value.char_indices() {
        match ch {
            '.' => {
                dots += 1;
                if dots == 3 {
                    return (&value[..index], Some(&value[index + 1..]));
                }
            }
            '-' => return (&value[..index], Some(&value[index + 1..])),
            _ => {}
        }
    }
    (value, None)
}

pub fn set(args: &SetArgs) -> Result<()> {
    if !is_semver_like(&args.version) {
        bail!(
            "invalid version `{}`; expected a semver-like value such as 0.7.0 or 0.7.0-rc.1",
            args.version
        );
    }
    for target in targets() {
        let text = fs::read_to_string(&target.path)
            .with_context(|| format!("cannot read {}", target.path.display()))?;
        let (start, end) = locate(&text, &target)
            .with_context(|| format!("no version field found in {}", target.label))?;
        if text[start..end] == args.version {
            println!("{:<40} already {}", target.label, args.version);
            continue;
        }
        let mut updated = String::with_capacity(text.len());
        updated.push_str(&text[..start]);
        updated.push_str(&args.version);
        updated.push_str(&text[end..]);
        fs::write(&target.path, updated)
            .with_context(|| format!("cannot write {}", target.path.display()))?;
        println!("{:<40} -> {}", target.label, args.version);
    }
    println!("Run `cargo build` to refresh Cargo.lock, then `cargo xtask version-check`.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(prefix: &'static str, indent_exact: bool) -> Target {
        Target {
            label: "test",
            path: PathBuf::new(),
            prefix,
            indent_exact,
        }
    }

    #[test]
    fn finds_top_level_version_lines_only() {
        let toml = "[workspace.dependencies]\nfoo = { version = \"9.9.9\" }\n\n[package]\nversion = \"0.6.0\"\n";
        let (start, end) = locate(toml, &target("version = ", false)).unwrap();
        assert_eq!(&toml[start..end], "0.6.0");
    }

    #[test]
    fn ignores_indented_version_keys_in_toml() {
        let toml = "[x]\n  version = \"1.0.0\"\nversion = \"2.0.0\"\n";
        let (start, end) = locate(toml, &target("version = ", false)).unwrap();
        assert_eq!(&toml[start..end], "2.0.0");
    }

    #[test]
    fn finds_the_python_dunder_version() {
        let py = "\"\"\"doc\"\"\"\n__version__ = \"0.6.0\"\n";
        let (start, end) = locate(py, &target("__version__ = ", false)).unwrap();
        assert_eq!(&py[start..end], "0.6.0");
    }

    #[test]
    fn finds_the_two_space_indented_json_version() {
        let json = "{\n  \"productName\": \"x\",\n  \"version\": \"0.6.0\",\n  \"nested\": {\n    \"version\": \"1\"\n  }\n}\n";
        let (start, end) = locate(json, &target("  \"version\": ", true)).unwrap();
        assert_eq!(&json[start..end], "0.6.0");
    }

    #[test]
    fn validates_semver_like_versions() {
        for good in [
            "0.7.0",
            "1.22.333",
            "0.7.0-rc.1",
            "1.0.0-beta",
            "2.0.0.dev1",
        ] {
            assert!(is_semver_like(good), "{good}");
        }
        for bad in [
            "", "1", "1.2", "v1.2.3", "1.2.x", "1.2.3-", "1.2.3 ", "a.b.c",
        ] {
            assert!(!is_semver_like(bad), "{bad}");
        }
    }

    #[test]
    fn the_real_manifests_are_in_sync() {
        check(&CheckArgs { expected: None }).expect("repository versions must agree");
    }
}
