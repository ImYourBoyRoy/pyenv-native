// ./xtask/src/checksums.rs
//! `cargo xtask checksums <dir>`: write one `SHA256SUMS` covering every release asset in `dir`.
//!
//! Each asset already has a `<name>.sha256` sidecar for the updater; the aggregate file lets users
//! verify a whole release with `sha256sum -c SHA256SUMS`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Args;

use crate::bundle::sha256_hex;

#[derive(Args)]
pub struct ChecksumArgs {
    /// Directory holding the release assets (searched non-recursively).
    #[arg(default_value = "dist")]
    dir: PathBuf,
}

/// Files that are checksums themselves or build leftovers rather than release assets.
fn is_asset(name: &str) -> bool {
    !(name == "SHA256SUMS"
        || name.ends_with(".sha256")
        || name.ends_with(".nsi")
        || name.starts_with('.'))
}

/// `<hex>  <name>` lines for every asset, sorted by name.
pub fn render(dir: &Path) -> Result<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .with_context(|| format!("cannot read {}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| is_asset(name))
        .collect();
    names.sort();
    if names.is_empty() {
        bail!("no release assets found in {}", dir.display());
    }
    let mut text = String::new();
    for name in names {
        text.push_str(&format!("{}  {name}\n", sha256_hex(&dir.join(&name))?));
    }
    Ok(text)
}

pub fn run(args: &ChecksumArgs) -> Result<()> {
    let text = render(&args.dir)?;
    let output = args.dir.join("SHA256SUMS");
    fs::write(&output, &text)?;
    println!(
        "wrote {} ({} assets)",
        output.display(),
        text.lines().count()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_assets_sorted_and_skips_sidecars() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("b.zip"), "b").unwrap();
        fs::write(temp.path().join("a.tar.gz"), "a").unwrap();
        fs::write(temp.path().join("a.tar.gz.sha256"), "ignored").unwrap();
        fs::write(temp.path().join("SHA256SUMS"), "stale").unwrap();
        let text = render(temp.path()).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].ends_with("  a.tar.gz"));
        assert!(lines[1].ends_with("  b.zip"));
        assert_eq!(lines[0].split("  ").next().unwrap().len(), 64);
    }

    #[test]
    fn an_empty_directory_is_an_error() {
        let temp = tempfile::tempdir().unwrap();
        assert!(render(temp.path()).is_err());
    }
}
