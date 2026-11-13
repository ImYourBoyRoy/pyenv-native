// ./xtask/src/installer/macos.rs
//! macOS: a flat `.pkg` installer and a `.dmg` that carries it.
//!
//! The `.pkg` is a system install, like the Linux packages: `pyenv` and `pyenv-mcp` go to
//! `/usr/local/bin` (already on every user's `PATH`) and the GUI is `/Applications/Pyenv Native.app`.
//! `share/pyenv-native/install-source` says `pkg`, so `pyenv self-update` points at the new
//! installer instead of rewriting files it does not own, and `sudo pyenv self-uninstall --system`
//! removes everything the receipt lists. Per-user data under `~/.pyenv` is never part of the package.
//!
//! `pkgbuild`, `productbuild`, `codesign`, and `hdiutil` only exist on macOS, so these two commands
//! run on the macOS release runners. Everything that decides *what* is built (Info.plist, icon,
//! installer UI, file list) is plain Rust and unit-tested on any host.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use super::payload::{self, Layout, Payload, PayloadFile};
use super::{Bundle, Manifest};

/// Receipt identifier; `pyenv self-uninstall --system` reads the same value.
pub(crate) const PKG_IDENTIFIER: &str = "com.pyenv-native.pkg";
const APP_BUNDLE: &str = "Pyenv Native.app";
const APP_ID: &str = "com.pyenv-native.gui";
/// Oldest macOS the release executables run on. `release.yml` builds with the same
/// `MACOSX_DEPLOYMENT_TARGET`, so the installer never admits a system the binaries cannot run on.
const MIN_MACOS: &str = "11.0";

/// Signing choices. Unsigned by default (ad hoc), which is what an arbitrary fork can do without
/// an Apple Developer account; a maintainer supplies identities through the environment.
#[derive(Debug, Clone)]
pub(crate) struct Signing {
    /// `codesign --sign` value; `-` is an ad-hoc signature.
    pub codesign_identity: String,
    /// `productbuild --sign` identity (Developer ID Installer); the package stays unsigned without it.
    pub installer_identity: Option<String>,
}

impl Signing {
    fn is_ad_hoc(&self) -> bool {
        self.codesign_identity == "-"
    }
}

/// `x86_64` for Intel, `arm64` for Apple silicon, as `hostArchitectures` spells them.
fn host_architecture(manifest: &Manifest) -> Result<&'static str> {
    match manifest.architecture.as_str() {
        "arm64" => Ok("arm64"),
        "x64" => Ok("x86_64"),
        other => bail!("unsupported macOS architecture `{other}`"),
    }
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// `CFBundleVersion` must be dotted digits, so a pre-release suffix is dropped.
fn bundle_version(version: &str) -> &str {
    version.split(['-', '+']).next().unwrap_or(version)
}

pub(crate) fn info_plist(version: &str) -> String {
    let short = xml_escape(version);
    let numeric = xml_escape(bundle_version(version));
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleDisplayName</key>
	<string>Pyenv Native</string>
	<key>CFBundleExecutable</key>
	<string>pyenv-gui</string>
	<key>CFBundleIconFile</key>
	<string>icon</string>
	<key>CFBundleIdentifier</key>
	<string>{APP_ID}</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>Pyenv Native</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>{short}</string>
	<key>CFBundleVersion</key>
	<string>{numeric}</string>
	<key>LSApplicationCategoryType</key>
	<string>public.app-category.developer-tools</string>
	<key>LSMinimumSystemVersion</key>
	<string>{MIN_MACOS}</string>
	<key>NSHighResolutionCapable</key>
	<true/>
	<key>NSHumanReadableCopyright</key>
	<string>Copyright pyenv-native contributors. MIT License.</string>
</dict>
</plist>
"#
    )
}

/// An `.icns` file: a container of PNG-encoded icons keyed by four-letter type.
pub(crate) fn icns(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let body: usize = entries.iter().map(|(_, png)| 8 + png.len()).sum();
    let mut out = Vec::with_capacity(8 + body);
    out.extend_from_slice(b"icns");
    out.extend_from_slice(&((8 + body) as u32).to_be_bytes());
    for (kind, png) in entries {
        debug_assert_eq!(kind.len(), 4);
        out.extend_from_slice(kind.as_bytes());
        out.extend_from_slice(&((8 + png.len()) as u32).to_be_bytes());
        out.extend_from_slice(png);
    }
    out
}

/// The GUI icons the repository already carries, packed for macOS.
fn app_icon() -> Result<Vec<u8>> {
    let dir = crate::repo_root()
        .join("crates")
        .join("pyenv-gui")
        .join("icons");
    let read = |name: &str| -> Result<Vec<u8>> {
        fs::read(dir.join(name))
            .with_context(|| format!("cannot read {}", dir.join(name).display()))
    };
    let small = read("128x128.png")?;
    let retina = read("128x128@2x.png")?;
    let large = read("icon.png")?;
    // ic07 = 128px, ic08 = 256px, ic13 = 128px@2x, ic09 = 512px.
    Ok(icns(&[
        ("ic07", &small),
        ("ic08", &retina),
        ("ic13", &retina),
        ("ic09", &large),
    ]))
}

/// Tell `pkgbuild` the app is not relocatable. Otherwise Installer "upgrades" any copy of the app
/// it finds elsewhere on disk (a build folder, a Trash item) instead of `/Applications`.
pub(crate) fn component_plist() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array>
	<dict>
		<key>BundleHasStrictIdentifier</key>
		<true/>
		<key>BundleIsRelocatable</key>
		<false/>
		<key>BundleIsVersionChecked</key>
		<false/>
		<key>BundleOverwriteAction</key>
		<string>upgrade</string>
		<key>RootRelativeBundlePath</key>
		<string>Applications/Pyenv Native.app</string>
	</dict>
</array>
</plist>
"#
    .to_string()
}

pub(crate) fn distribution_xml(version: &str, host: &str) -> String {
    let version = xml_escape(version);
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="2">
	<title>Pyenv Native</title>
	<organization>com.pyenv-native</organization>
	<options customize="never" require-scripts="false" hostArchitectures="{host}"/>
	<domains enable_localSystem="true" enable_currentUserHome="false" enable_anywhere="false"/>
	<allowed-os-versions>
		<os-version min="{MIN_MACOS}"/>
	</allowed-os-versions>
	<license file="LICENSE.txt" mime-type="text/plain"/>
	<conclusion file="conclusion.html" mime-type="text/html"/>
	<choices-outline>
		<line choice="default">
			<line choice="{PKG_IDENTIFIER}"/>
		</line>
	</choices-outline>
	<choice id="default"/>
	<choice id="{PKG_IDENTIFIER}" visible="false">
		<pkg-ref id="{PKG_IDENTIFIER}"/>
	</choice>
	<pkg-ref id="{PKG_IDENTIFIER}" version="{version}" onConclusion="none">component.pkg</pkg-ref>
</installer-gui-script>
"#
    )
}

pub(crate) fn conclusion_html() -> String {
    r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><style>body{font:13px -apple-system,sans-serif}code{font:12px ui-monospace,Menlo,monospace}</style></head>
<body>
<h2>Pyenv Native is installed</h2>
<p><code>pyenv</code> and <code>pyenv-mcp</code> are in <code>/usr/local/bin</code>, and the desktop app is in Applications.</p>
<p>Open a new Terminal window and add this line to your shell profile (<code>~/.zshrc</code>) to put your Python versions on <code>PATH</code>:</p>
<p><code>eval "$(pyenv init - zsh)"</code></p>
<p>Python versions and virtual environments live in <code>~/.pyenv</code>. To upgrade, install a newer package over this one. To remove the program files later, run <code>sudo pyenv self-uninstall --system</code>.</p>
</body></html>
"#
    .to_string()
}

fn read_me(version: &str) -> String {
    format!(
        "Pyenv Native {version}\n\nInstall:   open \"Install Pyenv Native.pkg\"\nUpgrade:   install a newer package over this one\nUninstall: sudo pyenv self-uninstall --system   (your Python versions in ~/.pyenv are kept)\n\nThis package is not signed with an Apple Developer ID. If macOS says it cannot be opened, Control-click the\npackage, choose Open, and confirm. Downloading with curl does not trigger that prompt.\nVerify the download against SHA256SUMS on the release page before installing.\n\nhttps://github.com/imyourboyroy/pyenv-native\n"
    )
}

/// System files plus the app bundle.
pub(crate) fn pkg_payload(bundle: &Bundle) -> Result<Payload> {
    let mut payload = payload::system_payload(bundle, &Layout::macos())?;
    if bundle.manifest.gui {
        let gui = bundle
            .read("pyenv-gui")
            .context("the manifest lists pyenv-gui but the bundle lacks it")?;
        let app = format!("/Applications/{APP_BUNDLE}/Contents");
        let mut push = |path: String, mode: u32, data: Vec<u8>| {
            payload.files.push(PayloadFile {
                path,
                mode,
                data,
                config: false,
            });
        };
        push(format!("{app}/MacOS/pyenv-gui"), 0o755, gui);
        push(
            format!("{app}/Info.plist"),
            0o644,
            info_plist(&bundle.manifest.version).into_bytes(),
        );
        push(format!("{app}/PkgInfo"), 0o644, b"APPL????".to_vec());
        push(format!("{app}/Resources/icon.icns"), 0o644, app_icon()?);
        push(
            format!("{app}/Resources/install-source"),
            0o644,
            b"pkg\n".to_vec(),
        );
        payload.files.sort_by(|a, b| a.path.cmp(&b.path));
    }
    Ok(payload)
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}

/// Lay the payload out under `root` exactly as it will be installed under `/`.
fn stage(payload: &Payload, root: &Path) -> Result<()> {
    for file in &payload.files {
        let target = root.join(file.path.trim_start_matches('/'));
        fs::create_dir_all(target.parent().context("payload path has no parent")?)?;
        fs::write(&target, &file.data)?;
        set_mode(&target, file.mode)?;
    }
    Ok(())
}

fn run(command: &mut Command) -> Result<()> {
    let shown = format!("{command:?}");
    let status = command
        .status()
        .with_context(|| format!("cannot run {shown}"))?;
    if !status.success() {
        bail!("{shown} failed with {status}");
    }
    Ok(())
}

fn require_macos() -> Result<()> {
    if cfg!(target_os = "macos") {
        Ok(())
    } else {
        bail!("pkgbuild, codesign, and hdiutil only exist on macOS; run this on a macOS machine")
    }
}

fn codesign(path: &Path, signing: &Signing) -> Result<()> {
    let mut command = Command::new("codesign");
    command.args(["--force", "--sign", &signing.codesign_identity]);
    if !signing.is_ad_hoc() {
        command.args(["--options", "runtime", "--timestamp"]);
    }
    command.arg(path);
    run(&mut command)
}

/// Build `pyenv-native-macos-<arch>.pkg` from a macOS release bundle.
pub(crate) fn write_pkg(bundle: &Bundle, output_root: &Path, signing: &Signing) -> Result<PathBuf> {
    require_macos()?;
    let manifest = &bundle.manifest;
    let host = host_architecture(manifest)?;
    let payload = pkg_payload(bundle)?;

    let work = tempfile::tempdir().context("cannot create a staging directory")?;
    let root = work.path().join("root");
    stage(&payload, &root)?;

    for binary in ["usr/local/bin/pyenv", "usr/local/bin/pyenv-mcp"] {
        codesign(&root.join(binary), signing)?;
    }
    let app = root.join("Applications").join(APP_BUNDLE);
    if app.is_dir() {
        codesign(&app, signing)?;
        run(Command::new("codesign")
            .args(["--verify", "--strict", "--deep"])
            .arg(&app))?;
    }

    let component_plist_path = work.path().join("components.plist");
    fs::write(&component_plist_path, component_plist())?;
    let component = work.path().join("component.pkg");
    run(Command::new("pkgbuild")
        .arg("--root")
        .arg(&root)
        .args([
            "--identifier",
            PKG_IDENTIFIER,
            "--version",
            &manifest.version,
        ])
        .args(["--install-location", "/"])
        .arg("--component-plist")
        .arg(&component_plist_path)
        .arg(&component))?;

    let resources = work.path().join("resources");
    fs::create_dir_all(&resources)?;
    if let Some(license) = bundle.read("LICENSE") {
        fs::write(resources.join("LICENSE.txt"), license)?;
    }
    fs::write(resources.join("conclusion.html"), conclusion_html())?;
    let distribution = work.path().join("distribution.xml");
    fs::write(&distribution, distribution_xml(&manifest.version, host))?;

    fs::create_dir_all(output_root)?;
    let output = output_root.join(format!("pyenv-native-macos-{}.pkg", manifest.architecture));
    let _ = fs::remove_file(&output);
    let mut product = Command::new("productbuild");
    product
        .arg("--distribution")
        .arg(&distribution)
        .arg("--resources")
        .arg(&resources)
        .arg("--package-path")
        .arg(work.path());
    if let Some(identity) = &signing.installer_identity {
        product.args(["--sign", identity]);
    }
    product.arg(&output);
    run(&mut product)?;
    Ok(output)
}

/// Wrap a `.pkg` in `pyenv-native-macos-<arch>.dmg` with a short read-me.
pub(crate) fn write_dmg(pkg: &Path, output_root: &Path) -> Result<PathBuf> {
    require_macos()?;
    let name = pkg
        .file_name()
        .and_then(|name| name.to_str())
        .context("the package path has no file name")?;
    let stem = name
        .strip_suffix(".pkg")
        .with_context(|| format!("{name} is not a .pkg"))?;
    let version = pkg_version(pkg)?;

    let work = tempfile::tempdir().context("cannot create a staging directory")?;
    let folder = work.path().join("Pyenv Native");
    fs::create_dir_all(&folder)?;
    fs::copy(pkg, folder.join("Install Pyenv Native.pkg"))?;
    fs::write(folder.join("Read Me.txt"), read_me(&version))?;

    fs::create_dir_all(output_root)?;
    let output = output_root.join(format!("{stem}.dmg"));
    let _ = fs::remove_file(&output);
    run(Command::new("hdiutil")
        .args(["create", "-quiet", "-ov", "-fs", "HFS+", "-format", "UDZO"])
        .args(["-volname", "Pyenv Native"])
        .arg("-srcfolder")
        .arg(&folder)
        .arg(&output))?;
    Ok(output)
}

/// The version recorded in a `.pkg`'s receipt metadata, via `pkgutil --expand`.
fn pkg_version(pkg: &Path) -> Result<String> {
    let work = tempfile::tempdir()?;
    let expanded = work.path().join("expanded");
    run(Command::new("pkgutil")
        .arg("--expand")
        .arg(pkg)
        .arg(&expanded))?;
    let text = fs::read_to_string(expanded.join("Distribution"))
        .context("the package has no Distribution file")?;
    text.split("version=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .map(str::to_string)
        .context("cannot read the version from the package")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_names_the_gui_executable_and_the_floor() {
        let plist = info_plist("0.8.0-rc1");
        assert!(plist.contains("<string>pyenv-gui</string>"));
        assert!(plist.contains("<string>com.pyenv-native.gui</string>"));
        assert!(plist.contains(&format!("<string>{MIN_MACOS}</string>")));
        // The marketing version keeps the suffix; the build version must be plain digits.
        assert!(plist.contains("<string>0.8.0-rc1</string>"));
        assert!(plist.contains("<key>CFBundleVersion</key>\n\t<string>0.8.0</string>"));
    }

    #[test]
    fn icns_is_a_well_formed_container() {
        let bytes = icns(&[("ic07", b"AAAA"), ("ic09", b"BBBBBB")]);
        assert_eq!(&bytes[..4], b"icns");
        assert_eq!(
            u32::from_be_bytes(bytes[4..8].try_into().unwrap()) as usize,
            bytes.len()
        );
        assert_eq!(&bytes[8..12], b"ic07");
        assert_eq!(u32::from_be_bytes(bytes[12..16].try_into().unwrap()), 12);
        assert_eq!(&bytes[16..20], b"AAAA");
        assert_eq!(&bytes[20..24], b"ic09");
        assert_eq!(u32::from_be_bytes(bytes[24..28].try_into().unwrap()), 14);
        assert_eq!(bytes.len(), 8 + 12 + 14);
    }

    #[test]
    fn the_repository_icons_pack_into_a_real_icns() {
        let bytes = app_icon().unwrap();
        assert_eq!(&bytes[..4], b"icns");
        // Each entry embeds a PNG.
        let pngs = bytes
            .windows(8)
            .filter(|w| *w == b"\x89PNG\r\n\x1a\n")
            .count();
        assert_eq!(pngs, 4);
    }

    #[test]
    fn distribution_pins_architecture_and_system_domain() {
        let xml = distribution_xml("0.8.0", "arm64");
        assert!(xml.contains(r#"hostArchitectures="arm64""#));
        assert!(xml.contains(r#"enable_localSystem="true""#));
        assert!(xml.contains(r#"enable_anywhere="false""#));
        assert!(xml.contains(&format!(r#"<os-version min="{MIN_MACOS}"/>"#)));
        assert!(xml.contains(&format!(
            r#"<pkg-ref id="{PKG_IDENTIFIER}" version="0.8.0""#
        )));
    }

    #[test]
    fn intel_and_apple_silicon_map_to_installer_spellings() {
        let manifest = |arch: &str| Manifest {
            version: "1.0.0".into(),
            platform: "macos".into(),
            architecture: arch.into(),
            gui: true,
        };
        assert_eq!(host_architecture(&manifest("arm64")).unwrap(), "arm64");
        assert_eq!(host_architecture(&manifest("x64")).unwrap(), "x86_64");
        assert!(host_architecture(&manifest("riscv")).is_err());
    }

    #[test]
    fn the_app_is_pinned_in_place_not_relocatable() {
        let plist = component_plist();
        assert!(plist.contains("<key>BundleIsRelocatable</key>\n\t\t<false/>"));
        assert!(plist.contains("Applications/Pyenv Native.app"));
    }

    #[test]
    fn xml_values_are_escaped() {
        assert_eq!(xml_escape("a&b<c>\"d\""), "a&amp;b&lt;c&gt;&quot;d&quot;");
    }

    #[test]
    fn the_installer_floor_matches_the_build_deployment_target() {
        let config =
            fs::read_to_string(crate::repo_root().join(".cargo").join("config.toml")).unwrap();
        assert!(
            config.contains(&format!("MACOSX_DEPLOYMENT_TARGET = \"{MIN_MACOS}\"")),
            ".cargo/config.toml must build for macOS {MIN_MACOS}, the version the installer admits"
        );
    }

    #[test]
    fn non_macos_hosts_get_a_clear_error() {
        if !cfg!(target_os = "macos") {
            let error = require_macos().unwrap_err().to_string();
            assert!(error.contains("only exist on macOS"));
        }
    }
}
