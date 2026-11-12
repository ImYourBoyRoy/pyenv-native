// ./xtask/src/package.rs
//! `cargo xtask package homebrew`: publish-ready Homebrew formula.
//!
//! Consumes the release bundles and their `.sha256` sidecars produced by `cargo xtask bundle`;
//! it submits nothing anywhere. The formula carries url/sha256 blocks per detected Linux/macOS
//! bundle.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};

use crate::{bundle, repo_root};

// Parsed once at startup; boxing the argument structs would only add noise.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
pub enum PackageCommand {
    /// Generate a Homebrew formula from Linux/macOS bundle checksum files.
    Homebrew(HomebrewArgs),
}

pub fn run(command: &PackageCommand) -> Result<()> {
    match command {
        PackageCommand::Homebrew(args) => homebrew(args),
    }
}

/// First 64-hex-digit token in a checksum file, lowercased.
pub fn parse_checksum(text: &str) -> Option<String> {
    text.split(|c: char| !c.is_ascii_hexdigit())
        .find(|token| token.len() == 64)
        .map(|token| token.to_ascii_lowercase())
}

// ---------------------------------------------------------------------------------------------
// homebrew
// ---------------------------------------------------------------------------------------------

#[derive(Args)]
pub struct HomebrewArgs {
    #[arg(long, default_value = "imyourboyroy/pyenv-native")]
    github_repo: String,
    /// Release tag (default: `v<workspace version>`).
    #[arg(long)]
    tag: Option<String>,
    /// Directories scanned (recursively) for `*.tar.gz.sha256` files
    /// (default: dist, dist/linux, dist/macos).
    #[arg(long = "asset-root", value_name = "DIR")]
    asset_roots: Vec<PathBuf>,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, default_value = "pyenv-native")]
    formula_name: String,
    #[arg(
        long,
        default_value = "Cross-platform, native-first reimplementation of pyenv"
    )]
    description: String,
    #[arg(long)]
    homepage: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrewAsset {
    pub os: String,
    pub arch: String,
    pub url: String,
    pub sha256: String,
}

/// `pyenv-native-(linux|macos)-(x64|arm64).tar.gz` → `(os, arch)`.
pub fn parse_brew_asset_name(name: &str) -> Option<(String, String)> {
    let stem = name
        .strip_prefix("pyenv-native-")?
        .strip_suffix(".tar.gz")?;
    let (os, arch) = stem.split_once('-')?;
    matches!(os, "linux" | "macos")
        .then_some(())
        .and(matches!(arch, "x64" | "arm64").then_some(()))?;
    Some((os.to_string(), arch.to_string()))
}

fn collect_checksum_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect_checksum_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("sha256") {
            out.push(path);
        }
    }
}

fn scan_brew_assets(roots: &[PathBuf], repo: &str, tag: &str) -> Result<Vec<BrewAsset>> {
    let mut files = Vec::new();
    for root in roots {
        collect_checksum_files(root, &mut files);
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut assets = Vec::new();
    for file in files {
        let Some(asset_name) = file
            .file_stem()
            .and_then(|s| s.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        let Some((os, arch)) = parse_brew_asset_name(&asset_name) else {
            continue;
        };
        if !seen.insert(asset_name.clone()) {
            continue;
        }
        let first_line = fs::read_to_string(&file)?
            .lines()
            .next()
            .unwrap_or("")
            .to_string();
        let sha256 = parse_checksum(&first_line).with_context(|| {
            format!(
                "checksum file {} did not contain a valid SHA-256 digest",
                file.display()
            )
        })?;
        assets.push(BrewAsset {
            url: format!("https://github.com/{repo}/releases/download/{tag}/{asset_name}"),
            os,
            arch,
            sha256,
        });
    }
    Ok(assets)
}

/// `pyenv-native` → `PyenvNative`.
pub fn formula_class_name(name: &str) -> String {
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            let first = chars.next().expect("non-empty").to_ascii_uppercase();
            format!("{first}{}", chars.as_str())
        })
        .collect()
}

fn conditional_body(assets: &[&BrewAsset], formula: &str) -> Result<String> {
    let mut ordered: Vec<&&BrewAsset> = assets.iter().collect();
    ordered.sort_by(|a, b| a.arch.cmp(&b.arch));
    let mut out = String::new();
    for (index, asset) in ordered.iter().enumerate() {
        let condition = match asset.arch.as_str() {
            "arm64" => "Hardware::CPU.arm?",
            "x64" => "Hardware::CPU.intel?",
            other => bail!("unsupported formula architecture '{other}'"),
        };
        let keyword = if index == 0 { "if" } else { "elsif" };
        out.push_str(&format!(
            "    {keyword} {condition}\n      url \"{}\"\n      sha256 \"{}\"\n",
            asset.url, asset.sha256
        ));
    }
    out.push_str(&format!(
        "    else\n      odie \"Unsupported architecture for {formula}\"\n    end\n"
    ));
    Ok(out)
}

/// The static part of the formula, after the per-platform url/sha256 blocks.
const FORMULA_TAIL: &str = r##"  def install
    bin.install "pyenv", "pyenv-mcp"
    prefix.install "LICENSE", "README.md"
  end

  def caveats
    <<~EOS
      pyenv-native keeps managed Python runtimes under ~/.pyenv by default.

      Add it to your shell manually:
        eval "$("#{bin}/pyenv" init - bash)"

      Or let pyenv wire your shell profile and shims for you:
        pyenv self-install --yes --install-root ~/.pyenv --shell bash
    EOS
  end

  test do
    ENV["PYENV_ROOT"] = testpath/".pyenv"
    assert_equal ENV["PYENV_ROOT"], shell_output("#{bin}/pyenv root").strip
  end
end
"##;

pub fn render_formula(
    name: &str,
    description: &str,
    homepage: &str,
    version: &str,
    assets: &[BrewAsset],
) -> Result<String> {
    let mut out = format!(
        "# ./packaging/homebrew/Formula/{name}.rb\nclass {} < Formula\n  desc \"{description}\"\n  homepage \"{homepage}\"\n  license \"MIT\"\n  version \"{version}\"\n\n",
        formula_class_name(name)
    );
    for (os, block) in [("macos", "on_macos"), ("linux", "on_linux")] {
        let subset: Vec<&BrewAsset> = assets.iter().filter(|a| a.os == os).collect();
        if !subset.is_empty() {
            out.push_str(&format!(
                "  {block} do\n{}  end\n\n",
                conditional_body(&subset, name)?
            ));
        }
    }
    out.push_str(FORMULA_TAIL);
    Ok(out)
}

fn homebrew(args: &HomebrewArgs) -> Result<()> {
    let root = repo_root();
    let version = bundle::workspace_version(&root)?;
    let tag = args.tag.clone().unwrap_or_else(|| format!("v{version}"));
    let homepage = args
        .homepage
        .clone()
        .unwrap_or_else(|| format!("https://github.com/{}", args.github_repo));
    let roots = if args.asset_roots.is_empty() {
        let dist = root.join("dist");
        vec![dist.clone(), dist.join("linux"), dist.join("macos")]
    } else {
        args.asset_roots.clone()
    };
    let assets = scan_brew_assets(&roots, &args.github_repo, &tag)?;
    if assets.is_empty() {
        bail!(
            "no Linux/macOS bundle checksum files were found; build or download release assets first"
        );
    }
    let formula = render_formula(
        &args.formula_name,
        &args.description,
        &homepage,
        &version,
        &assets,
    )?;
    let output = args.output.clone().unwrap_or_else(|| {
        root.join("packaging")
            .join("homebrew")
            .join("Formula")
            .join(format!("{}.rb", args.formula_name))
    });
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, formula)?;
    println!("output_path: {}", output.display());
    println!("tag: {tag}");
    println!(
        "macos_assets: {}",
        assets.iter().filter(|a| a.os == "macos").count()
    );
    println!(
        "linux_assets: {}",
        assets.iter().filter(|a| a.os == "linux").count()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn checksum_parsing_accepts_sidecar_formats() {
        assert_eq!(parse_checksum(SHA).as_deref(), Some(SHA));
        assert_eq!(
            parse_checksum(&format!("{}  file.zip\n", SHA.to_uppercase())).as_deref(),
            Some(SHA)
        );
        assert_eq!(parse_checksum("not a digest"), None);
        assert_eq!(parse_checksum(&SHA[..63]), None);
    }

    #[test]
    fn brew_asset_names_are_recognised() {
        assert_eq!(
            parse_brew_asset_name("pyenv-native-macos-arm64.tar.gz"),
            Some(("macos".into(), "arm64".into()))
        );
        assert_eq!(parse_brew_asset_name("pyenv-native-windows-x64.zip"), None);
        assert_eq!(
            parse_brew_asset_name("pyenv-native-linux-riscv.tar.gz"),
            None
        );
        assert_eq!(parse_brew_asset_name("other-linux-x64.tar.gz"), None);
    }

    #[test]
    fn formula_class_names_are_camel_cased() {
        assert_eq!(formula_class_name("pyenv-native"), "PyenvNative");
        assert_eq!(formula_class_name("a_b.c"), "ABC");
    }

    fn asset(os: &str, arch: &str) -> BrewAsset {
        BrewAsset {
            os: os.into(),
            arch: arch.into(),
            url: format!("https://dl/{os}-{arch}.tar.gz"),
            sha256: SHA.into(),
        }
    }

    #[test]
    fn formula_has_per_platform_blocks_and_installs_both_binaries() {
        let assets = vec![
            asset("linux", "x64"),
            asset("macos", "x64"),
            asset("macos", "arm64"),
        ];
        let rb = render_formula("pyenv-native", "desc", "https://h", "1.2.3", &assets).unwrap();
        assert!(rb.contains("class PyenvNative < Formula"));
        assert!(rb.contains("on_macos do\n    if Hardware::CPU.arm?"));
        assert!(rb.contains("elsif Hardware::CPU.intel?"));
        assert!(rb.contains("on_linux do\n    if Hardware::CPU.intel?"));
        assert!(rb.contains("bin.install \"pyenv\", \"pyenv-mcp\""));
        assert!(rb.contains("pyenv self-install --yes"));
        assert!(rb.contains("eval \"$(\"#{bin}/pyenv\" init - bash)\""));
        assert!(rb.matches("odie").count() == 2);
    }

    #[test]
    fn formula_omits_platform_blocks_without_assets() {
        let rb = render_formula("pyenv-native", "d", "h", "1", &[asset("linux", "x64")]).unwrap();
        assert!(!rb.contains("on_macos"));
    }

    #[test]
    fn asset_scan_dedupes_by_name_and_ignores_other_files() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::write(
            a.join("pyenv-native-linux-x64.tar.gz.sha256"),
            format!("{SHA}  x\n"),
        )
        .unwrap();
        fs::write(
            b.join("pyenv-native-linux-x64.tar.gz.sha256"),
            "f".repeat(64),
        )
        .unwrap();
        fs::write(a.join("pyenv-native-windows-x64.zip.sha256"), SHA).unwrap();
        let assets = scan_brew_assets(&[a, b], "o/r", "v1").unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].sha256, SHA);
        assert_eq!(
            assets[0].url,
            "https://github.com/o/r/releases/download/v1/pyenv-native-linux-x64.tar.gz"
        );
    }
}
