// ./xtask/src/bundle.rs
//! `cargo xtask bundle`: build release binaries and assemble the distributable archive.
//!
//! A bundle is a flat directory (`pyenv`, `pyenv-mcp`, optional `pyenv-gui`, docs, and
//! `bundle-manifest.json`) packed as `.tar.gz` (Linux, macOS, Android) or `.zip` (Windows) next to a
//! `<archive>.sha256` sidecar. The installers do not ship inside it: the bundled `pyenv` binary
//! performs the install through `pyenv self-install`.

use std::fs::{self, File};
use std::io::{self, BufWriter, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::Args;
use flate2::Compression;
use flate2::write::GzEncoder;
use sha2::{Digest, Sha256};

use crate::repo_root;

#[derive(Args)]
pub struct BundleArgs {
    /// Rust target triple to build for (default: the host).
    #[arg(long)]
    target: Option<String>,
    /// Where to write the bundle directory, archive, and checksum.
    #[arg(long, default_value = "dist")]
    output_root: PathBuf,
    /// Override the bundle name (default: pyenv-native-<platform>-<arch>).
    #[arg(long)]
    bundle_name: Option<String>,
    /// Override the platform token: linux, macos, windows, or android.
    #[arg(long)]
    platform: Option<String>,
    /// Override the architecture token: x64, arm64, or x86.
    #[arg(long)]
    arch: Option<String>,
    /// Cargo program to build with (use `cross` for musl/arm64 cross builds).
    #[arg(long, env = "XTASK_CARGO", default_value = "cargo")]
    cargo: String,
    /// Android API level for cargo-ndk builds.
    #[arg(long, env = "ANDROID_API_LEVEL", default_value = "21")]
    android_api_level: String,
    /// Package existing release binaries instead of building them.
    #[arg(long)]
    skip_build: bool,
    /// Leave pyenv-gui out even where it would normally be bundled.
    #[arg(long)]
    no_gui: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct Plan {
    platform: String,
    architecture: String,
    include_gui: bool,
}

fn normalize_arch(token: &str) -> String {
    match token.to_ascii_lowercase().as_str() {
        "x86_64" | "amd64" | "x64" => "x64".into(),
        "aarch64" | "arm64" => "arm64".into(),
        "i386" | "i686" | "x86" => "x86".into(),
        other => other.to_string(),
    }
}

fn platform_from_target(target: &str) -> Option<&'static str> {
    if target.contains("linux-android") {
        Some("android")
    } else if target.contains("apple-darwin") {
        Some("macos")
    } else if target.contains("pc-windows") {
        Some("windows")
    } else if target.contains("linux") {
        Some("linux")
    } else {
        None
    }
}

fn host_platform() -> Result<&'static str> {
    match std::env::consts::OS {
        "linux" => Ok("linux"),
        "macos" => Ok("macos"),
        "windows" => Ok("windows"),
        other => bail!("unsupported host operating system `{other}` for bundling"),
    }
}

fn plan(args: &BundleArgs) -> Result<Plan> {
    let target_platform = args.target.as_deref().and_then(platform_from_target);
    let platform = match (&args.platform, target_platform) {
        (Some(explicit), _) => explicit.clone(),
        (None, Some(derived)) => derived.to_string(),
        (None, None) => host_platform()?.to_string(),
    };
    let architecture = match (&args.arch, &args.target) {
        (Some(explicit), _) => normalize_arch(explicit),
        (None, Some(target)) => normalize_arch(target.split('-').next().unwrap_or_default()),
        (None, None) => normalize_arch(std::env::consts::ARCH),
    };
    // Tauri needs native system libraries (GTK/WebKit, WebView2), which cross-compilation
    // environments lack: only native desktop builds, and Windows x64, carry the GUI.
    let include_gui = !args.no_gui
        && match platform.as_str() {
            "android" => false,
            "windows" => architecture == "x64",
            _ => args.target.is_none(),
        };
    Ok(Plan {
        platform,
        architecture,
        include_gui,
    })
}

fn exe_name(base: &str, platform: &str) -> String {
    if platform == "windows" {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn android_abi(architecture: &str) -> Result<&'static str> {
    match architecture {
        "arm64" => Ok("arm64-v8a"),
        "x64" => Ok("x86_64"),
        "x86" => Ok("x86"),
        other => bail!("unsupported Android architecture `{other}`"),
    }
}

fn build(args: &BundleArgs, plan: &Plan, root: &Path) -> Result<()> {
    let mut packages = vec!["-p", "pyenv-cli", "-p", "pyenv-mcp"];
    if plan.include_gui {
        packages.extend(["-p", "pyenv-gui"]);
    }

    let mut command = if plan.platform == "android" && args.target.is_some() {
        let ndk_check = Command::new("cargo")
            .args(["ndk", "--help"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        if !ndk_check {
            bail!("cargo-ndk is required for Android bundles: cargo install cargo-ndk --locked");
        }
        let mut command = Command::new("cargo");
        command
            .args(["ndk", "-t", android_abi(&plan.architecture)?, "--"])
            .env("CARGO_NDK_PLATFORM", &args.android_api_level);
        command
    } else {
        Command::new(&args.cargo)
    };
    command.args(["build", "--release"]);
    if let Some(target) = &args.target {
        command.args(["--target", target]);
    }
    command.args(&packages).current_dir(root);

    println!("running: {command:?}");
    let status = command
        .status()
        .with_context(|| format!("failed to launch `{}`", args.cargo))?;
    if !status.success() {
        bail!("release build failed ({status})");
    }
    Ok(())
}

fn release_dir(args: &BundleArgs, root: &Path) -> PathBuf {
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    match &args.target {
        Some(target) => target_dir.join(target).join("release"),
        None => target_dir.join("release"),
    }
}

pub(crate) fn workspace_version(root: &Path) -> Result<String> {
    let text = fs::read_to_string(root.join("Cargo.toml")).context("cannot read Cargo.toml")?;
    text.lines()
        .find_map(|line| {
            let rest = line.strip_prefix("version = \"")?;
            Some(rest.split('"').next()?.to_string())
        })
        .context("could not determine the workspace version from Cargo.toml")
}

fn copy_file(from: &Path, to: &Path) -> Result<()> {
    fs::copy(from, to)
        .with_context(|| format!("cannot copy {} to {}", from.display(), to.display()))?;
    Ok(())
}

#[cfg(unix)]
fn mark_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(not(unix))]
fn mark_executable(_path: &Path) -> Result<()> {
    Ok(())
}

/// Icon sizes staged for the Linux desktop launcher (`pyenv self-install` copies them into
/// `$XDG_DATA_HOME/icons/hicolor`).
const LINUX_ICONS: [(&str, &str); 4] = [
    ("32x32.png", "32x32"),
    ("128x128.png", "128x128"),
    ("128x128@2x.png", "256x256"),
    ("icon.png", "512x512"),
];

fn assemble(args: &BundleArgs, plan: &Plan, root: &Path, bundle_dir: &Path) -> Result<()> {
    let release = release_dir(args, root);
    let platform = plan.platform.as_str();
    let required = [
        ("pyenv", true),
        ("pyenv-mcp", true),
        ("pyenv-gui", plan.include_gui),
    ];
    for (base, needed) in required {
        if !needed {
            continue;
        }
        let name = exe_name(base, platform);
        let source = release.join(&name);
        if !source.is_file() {
            bail!("release binary not found at {}", source.display());
        }
        let destination = bundle_dir.join(&name);
        copy_file(&source, &destination)?;
        mark_executable(&destination)?;
    }

    for (source, name) in [
        ("README.md", "README.md"),
        ("docs/INSTRUCTIONS.md", "INSTRUCTIONS.md"),
        ("docs/MCP.md", "MCP.md"),
        ("LICENSE", "LICENSE"),
    ] {
        let source = root.join(source);
        if source.is_file() {
            copy_file(&source, &bundle_dir.join(name))?;
        }
    }

    if plan.include_gui && platform == "linux" {
        let icons = root.join("crates").join("pyenv-gui").join("icons");
        for (file, size) in LINUX_ICONS {
            let dest_dir = bundle_dir
                .join("share")
                .join("icons")
                .join("hicolor")
                .join(size)
                .join("apps");
            fs::create_dir_all(&dest_dir)?;
            copy_file(
                &icons.join(file),
                &dest_dir.join("com.pyenv-native.gui.png"),
            )?;
        }
    }
    Ok(())
}

fn write_manifest(
    args: &BundleArgs,
    plan: &Plan,
    name: &str,
    version: &str,
    dir: &Path,
) -> Result<()> {
    let platform = plan.platform.as_str();
    let manifest = serde_json::json!({
        "bundle_name": name,
        "bundle_version": version,
        "platform": platform,
        "architecture": plan.architecture,
        "target_triple": args.target.clone().unwrap_or_default(),
        "executable": exe_name("pyenv", platform),
        "mcp_executable": exe_name("pyenv-mcp", platform),
        "gui_executable": plan.include_gui.then(|| exe_name("pyenv-gui", platform)),
        "installer": "self-install",
    });
    let mut text = serde_json::to_string_pretty(&manifest)?;
    text.push('\n');
    fs::write(dir.join("bundle-manifest.json"), text)?;
    Ok(())
}

fn write_tar_gz(dir: &Path, archive: &Path) -> Result<()> {
    let file = BufWriter::new(File::create(archive)?);
    let mut builder = tar::Builder::new(GzEncoder::new(file, Compression::best()));
    // Deterministic headers: normalised owners, times, and 0644/0755 modes from the exec bit.
    builder.mode(tar::HeaderMode::Deterministic);
    builder.follow_symlinks(false);
    builder.append_dir_all(".", dir)?;
    builder.into_inner()?.finish()?;
    Ok(())
}

fn collect_files(dir: &Path, base: &Path, out: &mut Vec<(PathBuf, PathBuf)>) -> Result<()> {
    let mut entries = fs::read_dir(dir)?.collect::<io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_files(&path, base, out)?;
        } else {
            out.push((path.clone(), path.strip_prefix(base)?.to_path_buf()));
        }
    }
    Ok(())
}

fn write_zip(dir: &Path, archive: &Path) -> Result<()> {
    let mut writer = zip::ZipWriter::new(BufWriter::new(File::create(archive)?));
    let mut files = Vec::new();
    collect_files(dir, dir, &mut files)?;
    for (path, relative) in files {
        let name = relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);
        writer.start_file(name, options)?;
        io::copy(&mut File::open(&path)?, &mut writer)?;
    }
    writer.finish()?;
    Ok(())
}

pub(crate) fn sha256_hex(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn run(args: &BundleArgs) -> Result<()> {
    let root = repo_root();
    let plan = plan(args)?;
    let version = workspace_version(&root)?;
    let name = args
        .bundle_name
        .clone()
        .unwrap_or_else(|| format!("pyenv-native-{}-{}", plan.platform, plan.architecture));

    if !args.skip_build {
        build(args, &plan, &root)?;
    }

    let output_root = if args.output_root.is_absolute() {
        args.output_root.clone()
    } else {
        std::env::current_dir()?.join(&args.output_root)
    };
    fs::create_dir_all(&output_root)?;
    let bundle_dir = output_root.join(&name);
    if bundle_dir.exists() {
        fs::remove_dir_all(&bundle_dir)?;
    }
    fs::create_dir_all(&bundle_dir)?;

    assemble(args, &plan, &root, &bundle_dir)?;
    write_manifest(args, &plan, &name, &version, &bundle_dir)?;

    let extension = if plan.platform == "windows" {
        "zip"
    } else {
        "tar.gz"
    };
    let archive = output_root.join(format!("{name}.{extension}"));
    let checksum = output_root.join(format!("{name}.{extension}.sha256"));
    for stale in [&archive, &checksum] {
        let _ = fs::remove_file(stale);
    }
    if extension == "zip" {
        write_zip(&bundle_dir, &archive)?;
    } else {
        write_tar_gz(&bundle_dir, &archive)?;
    }
    let digest = sha256_hex(&archive)?;
    let archive_name = archive.file_name().unwrap().to_string_lossy();
    fs::write(&checksum, format!("{digest}  {archive_name}\n"))?;

    println!("bundle_dir: {}", bundle_dir.display());
    println!("archive_path: {}", archive.display());
    println!("checksum_path: {}", checksum.display());
    println!("platform: {}", plan.platform);
    println!("architecture: {}", plan.architecture);
    println!("gui_included: {}", plan.include_gui);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use tempfile::TempDir;

    #[derive(Parser)]
    struct Wrapper {
        #[command(flatten)]
        args: BundleArgs,
    }

    fn parse(extra: &[&str]) -> BundleArgs {
        let mut argv = vec!["bundle"];
        argv.extend_from_slice(extra);
        Wrapper::parse_from(argv).args
    }

    #[test]
    fn derives_platform_and_arch_from_target_triples() {
        let cases = [
            ("aarch64-unknown-linux-musl", "linux", "arm64", false),
            ("aarch64-linux-android", "android", "arm64", false),
            ("x86_64-apple-darwin", "macos", "x64", false),
            ("x86_64-pc-windows-msvc", "windows", "x64", true),
            ("aarch64-pc-windows-msvc", "windows", "arm64", false),
        ];
        for (target, platform, arch, gui) in cases {
            let got = plan(&parse(&["--target", target])).unwrap();
            assert_eq!(
                got,
                Plan {
                    platform: platform.into(),
                    architecture: arch.into(),
                    include_gui: gui
                },
                "{target}"
            );
        }
    }

    #[test]
    fn native_desktop_builds_include_the_gui_and_no_gui_overrides() {
        let native = plan(&parse(&[])).unwrap();
        assert_eq!(native.include_gui, native.platform != "android");
        assert!(!plan(&parse(&["--no-gui"])).unwrap().include_gui);
    }

    #[test]
    fn explicit_overrides_win() {
        let got = plan(&parse(&[
            "--target",
            "aarch64-linux-android",
            "--platform",
            "android",
            "--arch",
            "arm64",
        ]))
        .unwrap();
        assert_eq!(
            (got.platform.as_str(), got.architecture.as_str()),
            ("android", "arm64")
        );
    }

    #[test]
    fn android_abis_map_from_architectures() {
        assert_eq!(android_abi("arm64").unwrap(), "arm64-v8a");
        assert!(android_abi("mips").is_err());
    }

    #[test]
    fn archives_round_trip_with_flat_layout_and_checksum() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("bundle");
        fs::create_dir_all(dir.join("share")).unwrap();
        fs::write(dir.join("pyenv"), "binary").unwrap();
        fs::write(dir.join("share").join("a.png"), "png").unwrap();

        let tarball = temp.path().join("b.tar.gz");
        write_tar_gz(&dir, &tarball).unwrap();
        let mut archive =
            tar::Archive::new(flate2::read::GzDecoder::new(File::open(&tarball).unwrap()));
        let names: Vec<String> = archive
            .entries()
            .unwrap()
            .map(|entry| {
                entry
                    .unwrap()
                    .path()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert!(
            names
                .iter()
                .any(|name| name.trim_start_matches("./") == "pyenv"),
            "{names:?}"
        );

        let zipped = temp.path().join("b.zip");
        write_zip(&dir, &zipped).unwrap();
        let mut zip = zip::ZipArchive::new(File::open(&zipped).unwrap()).unwrap();
        let mut zip_names: Vec<String> = (0..zip.len())
            .map(|index| zip.by_index(index).unwrap().name().unwrap().to_string())
            .collect();
        zip_names.sort();
        assert_eq!(
            zip_names,
            vec!["pyenv".to_string(), "share/a.png".to_string()]
        );

        assert_eq!(sha256_hex(&dir.join("pyenv")).unwrap().len(), 64);
    }

    #[test]
    fn manifest_names_the_self_install_flow() {
        let temp = TempDir::new().unwrap();
        let args = parse(&["--target", "x86_64-pc-windows-msvc"]);
        let plan = plan(&args).unwrap();
        write_manifest(
            &args,
            &plan,
            "pyenv-native-windows-x64",
            "9.9.9",
            temp.path(),
        )
        .unwrap();
        let manifest: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(temp.path().join("bundle-manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["platform"], "windows");
        assert_eq!(manifest["executable"], "pyenv.exe");
        assert_eq!(manifest["gui_executable"], "pyenv-gui.exe");
        assert_eq!(manifest["installer"], "self-install");
        assert!(manifest.get("install_script").is_none());
    }
}
