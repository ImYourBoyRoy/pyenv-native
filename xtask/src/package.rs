// ./xtask/src/package.rs
//! `cargo xtask package winget|homebrew`: publish-ready package-manager metadata.
//!
//! Both generators consume the release bundles and their `.sha256` sidecars produced by
//! `cargo xtask bundle`; neither submits anything anywhere.
//!
//! * `winget`   — three manifests (version, default locale, installer) under
//!   `packaging/winget/manifests/<l>/<Publisher>/<Package>/<version>/`
//! * `homebrew` — a `pyenv-native.rb` formula with url/sha256 blocks per detected Linux/macOS bundle

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};

use crate::{bundle, repo_root};

// Parsed once at startup; boxing the argument structs would only add noise.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
pub enum PackageCommand {
    /// Generate Winget portable-package manifests from Windows bundle archives.
    Winget(WingetArgs),
    /// Generate a Homebrew formula from Linux/macOS bundle checksum files.
    Homebrew(HomebrewArgs),
}

pub fn run(command: &PackageCommand) -> Result<()> {
    match command {
        PackageCommand::Winget(args) => winget(args),
        PackageCommand::Homebrew(args) => homebrew(args),
    }
}

/// First 64-hex-digit token in a checksum file, lowercased.
pub fn parse_checksum(text: &str) -> Option<String> {
    text.split(|c: char| !c.is_ascii_hexdigit())
        .find(|token| token.len() == 64)
        .map(|token| token.to_ascii_lowercase())
}

fn read_checksum(path: &Path) -> Result<String> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    parse_checksum(&text)
        .with_context(|| format!("{} did not contain a valid SHA-256 digest", path.display()))
}

// ---------------------------------------------------------------------------------------------
// winget
// ---------------------------------------------------------------------------------------------

#[derive(Args)]
pub struct WingetArgs {
    /// Windows bundle archive(s); repeat for multiple architectures.
    #[arg(long = "bundle", value_name = "ZIP")]
    bundles: Vec<PathBuf>,
    /// Checksum file per bundle, in the same order (default: `<bundle>.sha256`).
    #[arg(long = "checksum", value_name = "SHA256")]
    checksums: Vec<PathBuf>,
    #[arg(long)]
    output_root: Option<PathBuf>,
    #[arg(long, default_value = "ImYourBoyRoy.pyenv-native")]
    package_identifier: String,
    #[arg(long, default_value = "en-US")]
    package_locale: String,
    #[arg(long, default_value = "pyenv-native")]
    package_name: String,
    #[arg(long, default_value = "Roy Dawson IV")]
    publisher: String,
    #[arg(long, default_value = "https://github.com/imyourboyroy")]
    publisher_url: String,
    #[arg(
        long,
        default_value = "https://github.com/imyourboyroy/pyenv-native/issues"
    )]
    publisher_support_url: String,
    #[arg(long, default_value = "https://github.com/imyourboyroy/pyenv-native")]
    package_url: String,
    #[arg(long, default_value = "pyenv")]
    moniker: String,
    #[arg(long, default_value = "1.12.0")]
    manifest_version: String,
    #[arg(long, default_value = "imyourboyroy/pyenv-native")]
    github_repo: String,
    /// Release tag (default: `v<bundle version>`).
    #[arg(long)]
    tag: Option<String>,
    /// Override the installer download base URL (default: the GitHub release for `--tag`).
    #[arg(long)]
    release_base_url: Option<String>,
    #[arg(
        long,
        default_value = "Native-first cross-platform Python version manager compatible with pyenv workflows."
    )]
    short_description: String,
    #[arg(
        long,
        default_value = "Portable, native-first Python version manager that preserves familiar pyenv workflows across Windows, Linux, and macOS."
    )]
    description: String,
    /// Run `winget validate` on the generated manifests.
    #[arg(long)]
    validate: bool,
}

struct WingetBundle {
    file_name: String,
    checksum: String,
    architecture: String,
    version: String,
    executable: String,
}

/// Read `bundle-manifest.json` from the root of a bundle zip.
fn zip_manifest(path: &Path) -> Result<serde_json::Value> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("{} is not a zip archive", path.display()))?;
    let mut entry = archive
        .by_name("bundle-manifest.json")
        .with_context(|| format!("bundle-manifest.json was not found in {}", path.display()))?;
    let mut text = String::new();
    entry.read_to_string(&mut text)?;
    serde_json::from_str(&text).context("bundle-manifest.json is not valid JSON")
}

fn winget_bundle(path: &Path, checksum_path: &Path) -> Result<WingetBundle> {
    let manifest = zip_manifest(path)?;
    let field = |key: &str| -> Result<String> {
        manifest[key]
            .as_str()
            .map(str::to_string)
            .with_context(|| format!("bundle-manifest.json in {} lacks `{key}`", path.display()))
    };
    let platform = field("platform")?;
    if platform != "windows" {
        bail!(
            "Winget manifests can only be generated from Windows bundles; found platform '{platform}' in {}",
            path.display()
        );
    }
    Ok(WingetBundle {
        file_name: path
            .file_name()
            .context("bundle path has no file name")?
            .to_string_lossy()
            .into_owned(),
        checksum: read_checksum(checksum_path)?,
        architecture: field("architecture")?,
        version: field("bundle_version")?,
        executable: field("executable")?,
    })
}

/// `manifests/<first letter>/<Publisher>/<Package>[/…]` for a dotted package identifier.
pub fn winget_package_dir(output_root: &Path, identifier: &str) -> Result<PathBuf> {
    let segments: Vec<&str> = identifier.split('.').collect();
    if segments.len() < 2 || segments.iter().any(|segment| segment.is_empty()) {
        bail!("PackageIdentifier must include a publisher and package segment");
    }
    let mut path = output_root.join("manifests");
    path.push(segments[0][..1].to_ascii_lowercase());
    for segment in segments {
        path.push(segment);
    }
    Ok(path)
}

pub struct WingetInputs<'a> {
    pub identifier: &'a str,
    pub version: &'a str,
    pub locale: &'a str,
    pub manifest_version: &'a str,
    pub publisher: &'a str,
    pub publisher_url: &'a str,
    pub publisher_support_url: &'a str,
    pub package_name: &'a str,
    pub package_url: &'a str,
    pub moniker: &'a str,
    pub short_description: &'a str,
    pub description: &'a str,
    pub tag: &'a str,
    pub executable: &'a str,
    /// `(architecture, installer url, sha256)`
    pub installers: Vec<(String, String, String)>,
}

/// The three manifest documents: `(version, defaultLocale, installer)`.
pub fn render_winget(i: &WingetInputs<'_>) -> (String, String, String) {
    let schema = |kind: &str| {
        format!(
            "# yaml-language-server: $schema=https://aka.ms/winget-manifest.{kind}.{}.schema.json",
            i.manifest_version
        )
    };
    let version = format!(
        "{}\nPackageIdentifier: {}\nPackageVersion: {}\nDefaultLocale: {}\nManifestType: version\nManifestVersion: {}\n",
        schema("version"),
        i.identifier,
        i.version,
        i.locale,
        i.manifest_version
    );
    let locale = format!(
        "{schema}\nPackageIdentifier: {id}\nPackageVersion: {ver}\nPackageLocale: {loc}\nPublisher: {publisher}\n\
PublisherUrl: {publisher_url}\nPublisherSupportUrl: {support}\nAuthor: {publisher}\nPackageName: {name}\n\
PackageUrl: {url}\nLicense: MIT\nLicenseUrl: {url}/blob/{tag}/LICENSE\nShortDescription: {short}\n\
Description: {description}\nMoniker: {moniker}\nTags:\n  - pyenv\n  - python\n  - version-manager\n  - virtualenv\n\
ReleaseNotesUrl: {url}/releases/tag/{tag}\nManifestType: defaultLocale\nManifestVersion: {mv}\n",
        schema = schema("defaultLocale"),
        id = i.identifier,
        ver = i.version,
        loc = i.locale,
        publisher = i.publisher,
        publisher_url = i.publisher_url,
        support = i.publisher_support_url,
        name = i.package_name,
        url = i.package_url,
        tag = i.tag,
        short = i.short_description,
        description = i.description,
        moniker = i.moniker,
        mv = i.manifest_version,
    );
    let mut installers = i.installers.clone();
    installers.sort_by(|a, b| a.0.cmp(&b.0));
    let entries: String = installers
        .iter()
        .map(|(arch, url, sha)| {
            format!(
                "  - Architecture: {arch}\n    InstallerUrl: {url}\n    InstallerSha256: {sha}\n"
            )
        })
        .collect();
    let installer = format!(
        "{}\nPackageIdentifier: {}\nPackageVersion: {}\nInstallerType: zip\nNestedInstallerType: portable\n\
NestedInstallerFiles:\n  - RelativeFilePath: {}\n    PortableCommandAlias: pyenv\nArchiveBinariesDependOnPath: true\n\
Installers:\n{}ManifestType: installer\nManifestVersion: {}\n",
        schema("installer"),
        i.identifier,
        i.version,
        i.executable,
        entries,
        i.manifest_version
    );
    (version, locale, installer)
}

fn winget(args: &WingetArgs) -> Result<()> {
    let root = repo_root();
    let bundles = if args.bundles.is_empty() {
        vec![root.join("dist").join("pyenv-native-windows-x64.zip")]
    } else {
        args.bundles.clone()
    };
    if !args.checksums.is_empty() && args.checksums.len() != bundles.len() {
        bail!("--checksum must be given once per --bundle (or not at all)");
    }
    let mut records = Vec::new();
    for (index, bundle_path) in bundles.iter().enumerate() {
        let checksum_path = match args.checksums.get(index) {
            Some(path) => path.clone(),
            None => {
                let mut name = bundle_path.clone().into_os_string();
                name.push(".sha256");
                PathBuf::from(name)
            }
        };
        records.push(winget_bundle(bundle_path, &checksum_path)?);
    }
    let version = records[0].version.clone();
    if records.iter().any(|record| record.version != version) {
        bail!("all Windows bundles must have the same bundle_version");
    }
    let tag = args.tag.clone().unwrap_or_else(|| format!("v{version}"));
    let base = match &args.release_base_url {
        Some(url) => url.trim_end_matches('/').to_string(),
        None => format!(
            "https://github.com/{}/releases/download/{tag}",
            args.github_repo
        ),
    };
    let installers: Vec<(String, String, String)> = records
        .iter()
        .map(|record| {
            (
                record.architecture.clone(),
                format!("{base}/{}", record.file_name),
                record.checksum.clone(),
            )
        })
        .collect();
    let (version_doc, locale_doc, installer_doc) = render_winget(&WingetInputs {
        identifier: &args.package_identifier,
        version: &version,
        locale: &args.package_locale,
        manifest_version: &args.manifest_version,
        publisher: &args.publisher,
        publisher_url: &args.publisher_url,
        publisher_support_url: &args.publisher_support_url,
        package_name: &args.package_name,
        package_url: &args.package_url,
        moniker: &args.moniker,
        short_description: &args.short_description,
        description: &args.description,
        tag: &tag,
        executable: &records[0].executable,
        installers: installers.clone(),
    });

    let output_root = args
        .output_root
        .clone()
        .unwrap_or_else(|| root.join("packaging").join("winget"));
    let directory = winget_package_dir(&output_root, &args.package_identifier)?.join(&version);
    fs::create_dir_all(&directory)?;
    let id = &args.package_identifier;
    let version_path = directory.join(format!("{id}.yaml"));
    let locale_path = directory.join(format!("{id}.locale.{}.yaml", args.package_locale));
    let installer_path = directory.join(format!("{id}.installer.yaml"));
    fs::write(&version_path, version_doc)?;
    fs::write(&locale_path, locale_doc)?;
    fs::write(&installer_path, installer_doc)?;

    if args.validate {
        let status = Command::new("winget")
            .args(["validate", "--manifest"])
            .arg(&directory)
            .arg("--disable-interactivity")
            .status()
            .context("winget validation was requested, but winget is not available on PATH")?;
        if !status.success() {
            bail!("winget validate failed ({status})");
        }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "manifest_directory": directory,
            "version_manifest": version_path,
            "locale_manifest": locale_path,
            "installer_manifest": installer_path,
            "installer_urls": installers.iter().map(|i| i.1.clone()).collect::<Vec<_>>(),
            "package_identifier": args.package_identifier,
        }))?
    );
    Ok(())
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
    fn winget_directory_follows_the_winget_pkgs_layout() {
        let dir = winget_package_dir(Path::new("out"), "ImYourBoyRoy.pyenv-native").unwrap();
        assert_eq!(dir, Path::new("out/manifests/i/ImYourBoyRoy/pyenv-native"));
        assert!(winget_package_dir(Path::new("out"), "NoDots").is_err());
        assert!(winget_package_dir(Path::new("out"), "Bad..Id").is_err());
    }

    fn winget_inputs() -> WingetInputs<'static> {
        WingetInputs {
            identifier: "A.b",
            version: "1.2.3",
            locale: "en-US",
            manifest_version: "1.12.0",
            publisher: "Pub",
            publisher_url: "https://p",
            publisher_support_url: "https://p/issues",
            package_name: "b",
            package_url: "https://p/b",
            moniker: "m",
            short_description: "short",
            description: "long",
            tag: "v1.2.3",
            executable: "pyenv.exe",
            installers: vec![
                ("x64".into(), "https://dl/x64.zip".into(), SHA.into()),
                ("arm64".into(), "https://dl/arm64.zip".into(), SHA.into()),
            ],
        }
    }

    #[test]
    fn winget_manifests_are_complete_and_sorted_by_architecture() {
        let (version, locale, installer) = render_winget(&winget_inputs());
        assert!(
            version.contains("ManifestType: version") && version.contains("DefaultLocale: en-US")
        );
        assert!(locale.contains("LicenseUrl: https://p/b/blob/v1.2.3/LICENSE"));
        assert!(locale.contains("ReleaseNotesUrl: https://p/b/releases/tag/v1.2.3"));
        assert!(installer.contains("RelativeFilePath: pyenv.exe"));
        let arm = installer.find("Architecture: arm64").unwrap();
        let x64 = installer.find("Architecture: x64").unwrap();
        assert!(arm < x64, "arm64 must precede x64");
        assert!(installer.ends_with("ManifestType: installer\nManifestVersion: 1.12.0\n"));
        assert!(version.starts_with(
            "# yaml-language-server: $schema=https://aka.ms/winget-manifest.version.1.12.0"
        ));
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

    #[test]
    fn winget_reads_manifest_from_a_real_bundle_zip() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("pyenv-native-windows-x64.zip");
        let mut writer = zip::ZipWriter::new(File::create(&zip_path).unwrap());
        writer
            .start_file(
                "bundle-manifest.json",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        writer
            .write_all(br#"{"platform":"windows","architecture":"x64","bundle_version":"9.9.9","executable":"pyenv.exe"}"#)
            .unwrap();
        writer.finish().unwrap();
        let sum = dir.path().join("pyenv-native-windows-x64.zip.sha256");
        fs::write(&sum, SHA).unwrap();
        let record = winget_bundle(&zip_path, &sum).unwrap();
        assert_eq!(
            (record.version.as_str(), record.architecture.as_str()),
            ("9.9.9", "x64")
        );
        assert_eq!(record.file_name, "pyenv-native-windows-x64.zip");

        // A non-Windows bundle is rejected.
        let linux = dir.path().join("l.zip");
        let mut writer = zip::ZipWriter::new(File::create(&linux).unwrap());
        writer
            .start_file(
                "bundle-manifest.json",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        writer.write_all(br#"{"platform":"linux","architecture":"x64","bundle_version":"1","executable":"pyenv"}"#).unwrap();
        writer.finish().unwrap();
        assert!(winget_bundle(&linux, &sum).is_err());
    }
}
