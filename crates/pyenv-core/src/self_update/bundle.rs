// ./crates/pyenv-core/src/self_update/bundle.rs
//! Release-bundle acquisition: pick the right asset for this host, download it with its SHA-256
//! sidecar, verify, and extract it into a staging directory.
//!
//! The bundle contains the `pyenv` binary that performs the real install (`pyenv self-install`),
//! so the updater never executes a script it downloaded.

use std::env;
use std::fs;
use std::io::{self, BufReader, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tar::Archive;
use zip::ZipArchive;

use crate::http::{blocking_client_builder, build_blocking_client};
use crate::installer::{PYENV_BINARY, executable_file_name};

/// Environment override for the directory that holds release assets (mirrors, local testing).
/// Must be `https://`, or `http://` on a loopback host.
pub(super) const BASE_URL_ENV: &str = "PYENV_NATIVE_RELEASE_BASE_URL";

const STAGING_PREFIX: &str = "pyenv-native-self-update-";
const STALE_STAGING_AGE: Duration = Duration::from_secs(60 * 60 * 24);

/// Platform/architecture tokens used in release asset names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BundleTarget {
    pub platform: &'static str,
    pub architecture: &'static str,
}

impl BundleTarget {
    pub(super) fn current() -> Result<Self, String> {
        let platform = match env::consts::OS {
            "windows" => "windows",
            "linux" => "linux",
            "macos" => "macos",
            "android" => "android",
            other => return Err(format!("pyenv: no release bundles exist for {other} hosts")),
        };
        let architecture = match env::consts::ARCH {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            "x86" => "x86",
            other => {
                return Err(format!(
                    "pyenv: no release bundles exist for the {other} architecture"
                ));
            }
        };
        Ok(Self {
            platform,
            architecture,
        })
    }

    pub(super) fn asset_name(&self) -> String {
        let extension = if self.platform == "windows" {
            "zip"
        } else {
            "tar.gz"
        };
        format!(
            "pyenv-native-{}-{}.{extension}",
            self.platform, self.architecture
        )
    }
}

/// A bundle extracted and ready to run `pyenv self-install` from.
#[derive(Debug)]
pub(super) struct StagedBundle {
    pub staging_dir: PathBuf,
    pub pyenv_exe: PathBuf,
}

/// Where release assets for `tag` live.
pub(super) fn release_base_url(repo: &str, tag: &str) -> String {
    match env::var(BASE_URL_ENV) {
        Ok(value) if !value.trim().is_empty() => value.trim().trim_end_matches('/').to_string(),
        _ => format!("https://github.com/{repo}/releases/download/{tag}"),
    }
}

fn is_loopback_http(url: &str) -> bool {
    ["http://127.0.0.1", "http://localhost", "http://[::1]"]
        .iter()
        .any(|prefix| url.starts_with(prefix))
}

fn http_client(base_url: &str) -> Result<reqwest::blocking::Client, String> {
    let builder = if is_loopback_http(base_url) {
        blocking_client_builder().https_only(false)
    } else {
        return build_blocking_client()
            .map_err(|error| format!("pyenv: failed to construct HTTP client: {error}"));
    };
    builder
        .build()
        .map_err(|error| format!("pyenv: failed to construct HTTP client: {error}"))
}

pub(super) fn parse_sha256_token(body: &str) -> Option<String> {
    let token = body.split_whitespace().next()?.trim();
    (token.len() == 64 && token.chars().all(|ch| ch.is_ascii_hexdigit()))
        .then(|| token.to_ascii_lowercase())
}

/// Download `<base>/<asset>` and its `.sha256`, verify, and return the verified bytes' path.
fn download_verified(
    client: &reqwest::blocking::Client,
    base_url: &str,
    asset: &str,
    destination: &Path,
) -> Result<(), String> {
    let asset_url = format!("{base_url}/{asset}");
    let checksum_url = format!("{asset_url}.sha256");

    let checksum_text = client
        .get(&checksum_url)
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.text())
        .map_err(|error| {
            format!(
                "pyenv: missing checksum at {checksum_url} (releases must publish a .sha256 sidecar): {error}"
            )
        })?;
    let expected = parse_sha256_token(&checksum_text)
        .ok_or_else(|| format!("pyenv: invalid SHA-256 checksum at {checksum_url}"))?;

    let mut response = client
        .get(&asset_url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("pyenv: failed to download {asset_url}: {error}"))?;

    let mut file = fs::File::create(destination)
        .map_err(|error| format!("pyenv: cannot write {}: {error}", destination.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|error| format!("pyenv: download of {asset_url} failed: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read])
            .map_err(|error| format!("pyenv: cannot write {}: {error}", destination.display()))?;
    }
    file.flush()
        .map_err(|error| format!("pyenv: cannot write {}: {error}", destination.display()))?;

    let actual = hex::encode(hasher.finalize());
    if actual != expected {
        let _ = fs::remove_file(destination);
        return Err(format!(
            "pyenv: SHA-256 verification failed for {asset_url}: expected {expected}, got {actual}"
        ));
    }
    Ok(())
}

/// Extract `.tar.gz` or `.zip` into `destination`, refusing paths that escape it.
pub(super) fn extract_bundle(archive: &Path, destination: &Path) -> Result<(), String> {
    let name = archive
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let file = fs::File::open(archive)
        .map_err(|error| format!("pyenv: cannot open {}: {error}", archive.display()))?;
    fs::create_dir_all(destination)
        .map_err(|error| format!("pyenv: cannot create {}: {error}", destination.display()))?;

    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let mut tar = Archive::new(GzDecoder::new(BufReader::new(file)));
        tar.set_preserve_permissions(true);
        for entry in tar
            .entries()
            .map_err(|error| format!("pyenv: cannot read {}: {error}", archive.display()))?
        {
            let mut entry = entry.map_err(|error| {
                format!("pyenv: corrupt entry in {}: {error}", archive.display())
            })?;
            entry.unpack_in(destination).map_err(|error| {
                format!(
                    "pyenv: refused to extract from {}: {error}",
                    archive.display()
                )
            })?;
        }
        Ok(())
    } else if name.ends_with(".zip") {
        let mut zip = ZipArchive::new(file)
            .map_err(|error| format!("pyenv: cannot open {}: {error}", archive.display()))?;
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).map_err(|error| {
                format!("pyenv: corrupt entry in {}: {error}", archive.display())
            })?;
            let Some(relative) = entry.enclosed_name() else {
                continue;
            };
            let relative: PathBuf = relative
                .components()
                .filter(|component| matches!(component, Component::Normal(_)))
                .collect();
            if relative.as_os_str().is_empty() {
                continue;
            }
            let output = destination.join(relative);
            if entry.is_dir() {
                fs::create_dir_all(&output).map_err(io_message)?;
                continue;
            }
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent).map_err(io_message)?;
            }
            let mut out = fs::File::create(&output).map_err(io_message)?;
            io::copy(&mut entry, &mut out).map_err(io_message)?;
        }
        Ok(())
    } else {
        Err(format!(
            "pyenv: unsupported bundle format: {}",
            archive.display()
        ))
    }
}

fn io_message(error: io::Error) -> String {
    format!("pyenv: {error}")
}

/// Directory inside `extracted` that holds the `pyenv` binary (the root, or a single wrapper dir).
fn find_bundle_root(extracted: &Path) -> Option<PathBuf> {
    let binary = executable_file_name(PYENV_BINARY);
    if extracted.join(&binary).is_file() {
        return Some(extracted.to_path_buf());
    }
    let mut children = fs::read_dir(extracted)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir());
    let only = children.next()?;
    (children.next().is_none() && only.join(&binary).is_file()).then_some(only)
}

/// Check the bundle manifest (when present) names this host's platform and architecture.
fn verify_manifest(bundle_root: &Path, target: &BundleTarget) -> Result<(), String> {
    let manifest_path = bundle_root.join("bundle-manifest.json");
    let Ok(text) = fs::read_to_string(&manifest_path) else {
        return Ok(());
    };
    let manifest: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("pyenv: invalid bundle manifest: {error}"))?;
    let field = |name: &str| manifest.get(name).and_then(|value| value.as_str());
    if let Some(platform) = field("platform")
        && platform != target.platform
    {
        return Err(format!(
            "pyenv: bundle platform `{platform}` does not match this host (`{}`)",
            target.platform
        ));
    }
    if let Some(architecture) = field("architecture")
        && architecture != target.architecture
    {
        return Err(format!(
            "pyenv: bundle architecture `{architecture}` does not match this host (`{}`)",
            target.architecture
        ));
    }
    Ok(())
}

fn timestamp_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or(0)
}

/// Remove staging directories left behind by earlier updates (best effort).
pub(super) fn purge_stale_staging_dirs() {
    let Ok(entries) = fs::read_dir(env::temp_dir()) else {
        return;
    };
    for entry in entries.flatten() {
        let is_staging = entry
            .file_name()
            .to_string_lossy()
            .starts_with(STAGING_PREFIX);
        let is_old = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > STALE_STAGING_AGE);
        if is_staging && is_old {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

/// Download, verify, and extract the release bundle for `tag` into a fresh staging directory.
pub(super) fn stage_release_bundle(repo: &str, tag: &str) -> Result<StagedBundle, String> {
    let target = BundleTarget::current()?;
    let base_url = release_base_url(repo, tag);
    let client = http_client(&base_url)?;

    purge_stale_staging_dirs();
    let staging_dir = env::temp_dir().join(format!(
        "{STAGING_PREFIX}{}-{}",
        std::process::id(),
        timestamp_suffix()
    ));
    fs::create_dir_all(&staging_dir).map_err(|error| {
        format!("pyenv: failed to create the update staging directory: {error}")
    })?;

    let result = (|| {
        let asset = target.asset_name();
        let archive = staging_dir.join(&asset);
        download_verified(&client, &base_url, &asset, &archive)?;
        let extracted = staging_dir.join("bundle");
        extract_bundle(&archive, &extracted)?;
        let _ = fs::remove_file(&archive);
        let root = find_bundle_root(&extracted).ok_or_else(|| {
            format!(
                "pyenv: the downloaded bundle did not contain `{}`",
                executable_file_name(PYENV_BINARY)
            )
        })?;
        verify_manifest(&root, &target)?;
        Ok(root.join(executable_file_name(PYENV_BINARY)))
    })();

    match result {
        Ok(pyenv_exe) => Ok(StagedBundle {
            staging_dir,
            pyenv_exe,
        }),
        Err(error) => {
            let _ = fs::remove_dir_all(&staging_dir);
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use mockito::Server;
    use tempfile::TempDir;

    #[test]
    fn asset_names_follow_the_release_convention() {
        let linux = BundleTarget {
            platform: "linux",
            architecture: "x64",
        };
        assert_eq!(linux.asset_name(), "pyenv-native-linux-x64.tar.gz");
        let windows = BundleTarget {
            platform: "windows",
            architecture: "arm64",
        };
        assert_eq!(windows.asset_name(), "pyenv-native-windows-arm64.zip");
        assert!(BundleTarget::current().is_ok());
    }

    #[test]
    fn parses_checksum_sidecars() {
        let hash = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(
            parse_sha256_token(&format!("{hash}  pyenv-native-linux-x64.tar.gz\n")),
            Some(hash.to_string())
        );
        assert_eq!(parse_sha256_token(hash), Some(hash.to_string()));
        assert_eq!(parse_sha256_token("deadbeef"), None);
        assert_eq!(parse_sha256_token(""), None);
    }

    #[test]
    fn loopback_http_is_the_only_plain_http_allowed() {
        assert!(is_loopback_http("http://127.0.0.1:8000/x"));
        assert!(is_loopback_http("http://localhost/x"));
        assert!(!is_loopback_http("http://example.com/x"));
        assert!(!is_loopback_http("https://localhost/x"));
    }

    fn tar_gz(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::fast()));
        for (name, data) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder
                .append_data(&mut header, format!("./{name}"), *data)
                .unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn extracts_tarballs_and_finds_the_binary() {
        let temp = TempDir::new().unwrap();
        let archive = temp.path().join("bundle.tar.gz");
        let name = executable_file_name(PYENV_BINARY);
        fs::write(
            &archive,
            tar_gz(&[(name.as_str(), b"bin"), ("bundle-manifest.json", b"{}")]),
        )
        .unwrap();
        let out = temp.path().join("out");
        extract_bundle(&archive, &out).unwrap();
        let root = find_bundle_root(&out).expect("root");
        assert_eq!(fs::read(root.join(&name)).unwrap(), b"bin");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(root.join(&name)).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0o111);
        }
    }

    #[test]
    fn finds_a_binary_inside_a_single_wrapper_directory() {
        let temp = TempDir::new().unwrap();
        let wrapped = temp.path().join("pyenv-native-linux-x64");
        fs::create_dir_all(&wrapped).unwrap();
        fs::write(wrapped.join(executable_file_name(PYENV_BINARY)), "x").unwrap();
        assert_eq!(find_bundle_root(temp.path()), Some(wrapped));
    }

    #[test]
    fn manifest_must_match_the_host() {
        let temp = TempDir::new().unwrap();
        let target = BundleTarget {
            platform: "linux",
            architecture: "x64",
        };
        assert!(
            verify_manifest(temp.path(), &target).is_ok(),
            "no manifest is tolerated"
        );
        fs::write(
            temp.path().join("bundle-manifest.json"),
            r#"{"platform":"macos","architecture":"x64"}"#,
        )
        .unwrap();
        assert!(verify_manifest(temp.path(), &target).is_err());
        fs::write(
            temp.path().join("bundle-manifest.json"),
            r#"{"platform":"linux","architecture":"x64"}"#,
        )
        .unwrap();
        assert!(verify_manifest(temp.path(), &target).is_ok());
    }

    #[test]
    fn download_is_verified_against_the_sidecar() {
        let body = b"payload bytes".to_vec();
        let good = hex::encode(Sha256::digest(&body));
        let mut server = Server::new();
        let _asset = server
            .mock("GET", "/asset.tar.gz")
            .with_body(body.clone())
            .create();
        let _sum = server
            .mock("GET", "/asset.tar.gz.sha256")
            .with_body(format!("{good}  asset.tar.gz\n"))
            .create();
        let client = build_blocking_client().unwrap();
        let temp = TempDir::new().unwrap();
        let dest = temp.path().join("asset.tar.gz");
        download_verified(&client, &server.url(), "asset.tar.gz", &dest).expect("verified");
        assert_eq!(fs::read(&dest).unwrap(), body);
    }

    #[test]
    fn a_checksum_mismatch_deletes_the_download() {
        let mut server = Server::new();
        let _asset = server
            .mock("GET", "/asset.zip")
            .with_body("tampered")
            .create();
        let _sum = server
            .mock("GET", "/asset.zip.sha256")
            .with_body("0".repeat(64))
            .create();
        let client = build_blocking_client().unwrap();
        let temp = TempDir::new().unwrap();
        let dest = temp.path().join("asset.zip");
        let error = download_verified(&client, &server.url(), "asset.zip", &dest).unwrap_err();
        assert!(error.contains("SHA-256 verification failed"), "{error}");
        assert!(!dest.exists());
    }

    #[test]
    fn a_missing_checksum_sidecar_is_an_error() {
        let mut server = Server::new();
        let _asset = server.mock("GET", "/asset.zip").with_body("x").create();
        let _sum = server
            .mock("GET", "/asset.zip.sha256")
            .with_status(404)
            .create();
        let client = build_blocking_client().unwrap();
        let temp = TempDir::new().unwrap();
        let error = download_verified(&client, &server.url(), "asset.zip", &temp.path().join("a"))
            .unwrap_err();
        assert!(error.contains("missing checksum"), "{error}");
    }
}
