// ./xtask/src/installer/mod.rs
//! `cargo xtask installer <kind>`: native installers built from the release bundles.
//!
//! Every format reuses the binaries already produced by `cargo xtask bundle`, so a release builds
//! each executable exactly once. Package formats differ in where they put files, never in what the
//! files are:
//!
//! * `deb`, `rpm`: system packages (`/usr`) that the distro package manager owns and updates.
//! * `termux`: a `.deb` rooted at the Termux prefix for Android.
//! * `nsis`: a per-user Windows setup that runs `pyenv self-install`, so it shares one install
//!   layout (and the in-place updater) with the scripts and `pyenv self-update`.
//!
//! System packages carry `share/pyenv-native/install-source`; `pyenv self-update` reads it and
//! defers to the package manager instead of replacing files it does not own.

mod deb;
mod nsis;
mod payload;
mod rpm_pkg;

use std::fs::{self, File};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use flate2::read::GzDecoder;

use crate::bundle::sha256_hex;

#[derive(Subcommand)]
pub enum InstallerCommand {
    /// Debian/Ubuntu package (`pyenv-native_<version>_<arch>.deb`) from a Linux bundle.
    Deb(PackageArgs),
    /// Fedora/RHEL/openSUSE package (`pyenv-native-<version>-1.<arch>.rpm`) from a Linux bundle.
    Rpm(PackageArgs),
    /// Termux package (`.deb` rooted at the Termux prefix) from an Android bundle.
    Termux(PackageArgs),
    /// Windows setup executable built with NSIS from a Windows bundle (needs `makensis`).
    Nsis(NsisArgs),
}

#[derive(Args)]
pub struct PackageArgs {
    /// Release bundle archive produced by `cargo xtask bundle`.
    #[arg(long)]
    bundle: PathBuf,
    /// Directory that receives the package and its `.sha256` sidecar.
    #[arg(long, default_value = "dist")]
    output_root: PathBuf,
    /// Fail if the executables need a glibc newer than 2.<N> (guards the supported-distro floor,
    /// e.g. 35 for Ubuntu 22.04).
    #[arg(long, value_name = "MINOR")]
    max_glibc: Option<u32>,
}

#[derive(Args)]
pub struct NsisArgs {
    #[command(flatten)]
    package: PackageArgs,
    /// Path to `makensis` (default: found on PATH).
    #[arg(long, env = "MAKENSIS", default_value = "makensis")]
    makensis: String,
    /// Write the `.nsi` script next to the output and stop, without compiling it.
    #[arg(long)]
    script_only: bool,
}

pub fn run(command: &InstallerCommand) -> Result<()> {
    match command {
        InstallerCommand::Deb(args) => {
            let bundle = Bundle::open(&args.bundle, "linux")?;
            let payload = payload::system_payload(&bundle, &payload::Layout::debian())?;
            enforce_glibc_floor(&payload, args.max_glibc)?;
            let output = deb::write(
                &payload,
                &bundle.manifest,
                deb::Flavor::Debian,
                &args.output_root,
            )?;
            finish(&output)
        }
        InstallerCommand::Termux(args) => {
            let bundle = Bundle::open(&args.bundle, "android")?;
            let payload = payload::system_payload(&bundle, &payload::Layout::termux())?;
            let output = deb::write(
                &payload,
                &bundle.manifest,
                deb::Flavor::Termux,
                &args.output_root,
            )?;
            finish(&output)
        }
        InstallerCommand::Rpm(args) => {
            let bundle = Bundle::open(&args.bundle, "linux")?;
            let payload = payload::system_payload(&bundle, &payload::Layout::rpm())?;
            enforce_glibc_floor(&payload, args.max_glibc)?;
            let output = rpm_pkg::write(&payload, &bundle.manifest, &args.output_root)?;
            finish(&output)
        }
        InstallerCommand::Nsis(args) => {
            let bundle = Bundle::open(&args.package.bundle, "windows")?;
            let output = nsis::write(
                &bundle,
                &args.package.output_root,
                &args.makensis,
                args.script_only,
            )?;
            if args.script_only {
                println!("script_path: {}", output.display());
                return Ok(());
            }
            finish(&output)
        }
    }
}

fn enforce_glibc_floor(payload: &payload::Payload, max: Option<u32>) -> Result<()> {
    let floor = payload.glibc_floor();
    match (floor, max) {
        (Some(actual), Some(limit)) if actual > limit => bail!(
            "the executables need glibc 2.{actual} but the policy allows at most 2.{limit}; build the Linux bundle on an older runner (ubuntu-22.04)"
        ),
        (Some(actual), _) => println!("glibc_floor: 2.{actual}"),
        (None, _) => println!("glibc_floor: none (static)"),
    }
    Ok(())
}

/// Write the `<file>.sha256` sidecar (`<hex>  <name>`, the format `bundle` and the updater use).
fn finish(output: &Path) -> Result<()> {
    let digest = sha256_hex(output)?;
    let name = output.file_name().context("output has no file name")?;
    let sidecar = PathBuf::from(format!("{}.sha256", output.display()));
    fs::write(&sidecar, format!("{digest}  {}\n", name.to_string_lossy()))?;
    println!("package_path: {}", output.display());
    println!("checksum_path: {}", sidecar.display());
    Ok(())
}

/// What `bundle-manifest.json` says about a release bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Manifest {
    pub version: String,
    pub platform: String,
    pub architecture: String,
    pub gui: bool,
}

impl Manifest {
    pub(crate) fn parse(text: &str) -> Result<Self> {
        let json: serde_json::Value = serde_json::from_str(text).context("bad bundle manifest")?;
        let field = |name: &str| -> Result<String> {
            json.get(name)
                .and_then(|value| value.as_str())
                .map(str::to_string)
                .with_context(|| format!("bundle manifest is missing `{name}`"))
        };
        Ok(Self {
            version: field("bundle_version")?,
            platform: field("platform")?,
            architecture: field("architecture")?,
            gui: json
                .get("gui_executable")
                .is_some_and(|value| value.is_string()),
        })
    }
}

/// A release bundle unpacked into a temporary directory.
pub(crate) struct Bundle {
    dir: tempfile::TempDir,
    pub manifest: Manifest,
}

impl Bundle {
    pub(crate) fn open(archive: &Path, expected_platform: &str) -> Result<Self> {
        let dir = tempfile::tempdir().context("cannot create a staging directory")?;
        extract(archive, dir.path())
            .with_context(|| format!("cannot unpack {}", archive.display()))?;
        let manifest_text = fs::read_to_string(dir.path().join("bundle-manifest.json"))
            .context("the archive has no bundle-manifest.json; is it a release bundle?")?;
        let manifest = Manifest::parse(&manifest_text)?;
        if manifest.platform != expected_platform {
            bail!(
                "expected a {expected_platform} bundle but {} is a {} bundle",
                archive.display(),
                manifest.platform
            );
        }
        Ok(Self { dir, manifest })
    }

    pub(crate) fn path(&self) -> &Path {
        self.dir.path()
    }

    pub(crate) fn read(&self, relative: &str) -> Option<Vec<u8>> {
        fs::read(self.path().join(relative)).ok()
    }
}

fn extract(archive: &Path, destination: &Path) -> Result<()> {
    let name = archive.to_string_lossy();
    if name.ends_with(".zip") {
        let mut zip = zip::ZipArchive::new(File::open(archive)?)?;
        zip.extract(destination)?;
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let decoder = GzDecoder::new(File::open(archive)?);
        let mut tar = tar::Archive::new(decoder);
        tar.set_preserve_permissions(false);
        tar.unpack(destination)?;
    } else {
        bail!("unsupported bundle archive `{name}` (expected .zip or .tar.gz)");
    }
    Ok(())
}

/// Debian and RPM both reject `-` inside a version; pre-release suffixes become `~` so that
/// `1.0.0~rc1` sorts before `1.0.0`.
pub(crate) fn package_version(version: &str) -> String {
    version.replacen('-', "~", 1)
}

/// Timestamp stamped on package contents: `SOURCE_DATE_EPOCH`, else the last commit, so rebuilding
/// the same commit yields the same bytes.
pub(crate) fn source_date_epoch() -> u64 {
    if let Some(value) = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|value| value.trim().parse().ok())
    {
        return value;
    }
    std::process::Command::new("git")
        .args(["log", "-1", "--format=%ct"])
        .current_dir(crate::repo_root())
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8_lossy(&output.stdout).trim().parse().ok())
        .unwrap_or(1_700_000_000)
}
