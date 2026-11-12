// ./xtask/src/known_versions.rs
//! `cargo xtask sync-known-versions`: regenerate the embedded offline version-name seed.
//!
//! The seed (`crates/pyenv-core/data/known_versions.txt`) is only the fallback used when
//! python.org cannot be reached, plus non-CPython family names. Installable CPython revisions are
//! discovered at runtime from <https://www.python.org/ftp/python/> and cached, so this file is
//! not the live catalog.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Args;

use crate::repo_root;

#[derive(Args)]
pub struct SyncArgs {
    /// A clone of upstream pyenv (default: a sibling `pyenv` checkout).
    #[arg(long)]
    upstream_root: Option<PathBuf>,
}

/// Sorted, deduplicated definition names rendered as one per line with a trailing newline.
pub fn render(mut names: Vec<String>) -> String {
    names.sort();
    names.dedup();
    let mut text = names.join("\n");
    text.push('\n');
    text
}

pub fn run(args: &SyncArgs) -> Result<()> {
    let root = repo_root();
    let upstream = args
        .upstream_root
        .clone()
        .unwrap_or_else(|| root.join("..").join("pyenv"));
    let definitions = upstream
        .join("plugins")
        .join("python-build")
        .join("share")
        .join("python-build");
    if !definitions.is_dir() {
        bail!(
            "python-build definitions directory not found at {}",
            definitions.display()
        );
    }
    let names: Vec<String> = fs::read_dir(&definitions)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    let count = names.len();
    let output = root
        .join("crates")
        .join("pyenv-core")
        .join("data")
        .join("known_versions.txt");
    fs::write(&output, render(names)).with_context(|| format!("writing {}", output.display()))?;
    println!("Wrote {count} version definitions to {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_is_sorted_unique_and_newline_terminated() {
        let text = render(vec![
            "3.9.1".into(),
            "3.10.0".into(),
            "3.9.1".into(),
            "anaconda3-5.0".into(),
        ]);
        assert_eq!(text, "3.10.0\n3.9.1\nanaconda3-5.0\n");
    }
}
