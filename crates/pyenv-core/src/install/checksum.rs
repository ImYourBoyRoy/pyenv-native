// ./crates/pyenv-core/src/install/checksum.rs
//! Publisher-sourced digest lookup and SHA-256/SHA-512 verification for runtime downloads.
//!
//! Purpose: verify every fetched interpreter archive against the publisher's current
//! checksum metadata instead of baking version-specific hashes into the repo.
//! How to use: `verify_package_digest(ctx, plan, path)` after download or
//! before reusing a cached file.
//! Inputs: install plan URL/provider, optional cached sidecar, live HTTP metadata.
//! Outputs: Ok when the file matches; ChecksumMismatch / MissingChecksum otherwise.
//! Notes: NuGet SHA-512 comes from the registration catalog; CPython source SHA-256
//! from python.org's release-file API; PyPy SHA-256 from pypy.org/checksums.html.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::Deserialize;
use sha2::{Digest, Sha256, Sha512};

use crate::context::AppContext;
use crate::error::PyenvError;
use crate::http::build_blocking_client;

use super::report::io_error;
use super::types::{InstallPlan, PYPY_INDEX_TTL_SECS};

pub(super) const PYPY_CHECKSUMS_URL: &str = "https://www.pypy.org/checksums.html";
const PYTHON_ORG_RELEASE_API: &str = "https://www.python.org/api/v2/downloads/release/";
const PYTHON_ORG_RELEASE_FILE_API: &str = "https://www.python.org/api/v2/downloads/release_file/";
const NUGET_REGISTRATION_BASE: &str = "https://api.nuget.org/v3/registration5-semver1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DigestAlg {
    Sha256,
    Sha512,
}

impl DigestAlg {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Sha256 => "sha256",
            Self::Sha512 => "sha512",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "sha256" | "sha-256" => Some(Self::Sha256),
            "sha512" | "sha-512" => Some(Self::Sha512),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExpectedDigest {
    pub alg: DigestAlg,
    pub bytes: Vec<u8>,
    pub source: String,
}

pub(super) fn verify_package_digest(
    ctx: &AppContext,
    plan: &InstallPlan,
    path: &Path,
) -> Result<(), PyenvError> {
    let expected = resolve_expected_digest(ctx, plan)?;
    let actual = hash_file(path, expected.alg)?;
    if actual != expected.bytes {
        return Err(PyenvError::ChecksumMismatch {
            url: plan.download_url.clone(),
            algorithm: expected.alg.as_str().to_string(),
            expected: hex::encode(&expected.bytes),
            actual: hex::encode(&actual),
        });
    }

    write_digest_sidecar(path, &expected)?;
    Ok(())
}

pub(super) fn resolve_expected_digest(
    ctx: &AppContext,
    plan: &InstallPlan,
) -> Result<ExpectedDigest, PyenvError> {
    match resolve_publisher_digest(ctx, plan) {
        Ok(digest) => Ok(digest),
        Err(error) => {
            if let Some(cached) = read_digest_sidecar(&plan.cache_path) {
                return Ok(cached);
            }
            Err(error)
        }
    }
}

fn resolve_publisher_digest(
    ctx: &AppContext,
    plan: &InstallPlan,
) -> Result<ExpectedDigest, PyenvError> {
    if plan.download_url.starts_with("python-build://") {
        return Err(PyenvError::MissingChecksum(format!(
            "{} (python-build CPython sources are prefetched and verified in PYTHON_BUILD_CACHE_PATH)",
            plan.download_url
        )));
    }

    if plan.provider == "windows-cpython-nuget" {
        return fetch_nuget_digest(plan);
    }
    if plan.provider.ends_with("-cpython-source") {
        return fetch_python_org_source_digest(plan);
    }
    if plan.provider.contains("pypy") {
        return fetch_pypy_digest(ctx, plan);
    }

    fetch_generic_sidecar_digest(&plan.download_url)
}

pub(super) fn verify_python_build_cache(
    ctx: &AppContext,
    plan: &InstallPlan,
) -> Result<Option<ExpectedDigest>, PyenvError> {
    if !python_build_cache_is_cpython(plan) {
        return Ok(None);
    }
    let cache_dir = ctx.cache_dir().join("python-build");
    let version = plan.resolved_version.trim();
    let names = [
        format!("Python-{version}.tgz"),
        format!("Python-{version}.tar.xz"),
        format!("Python-{version}.tar.bz2"),
    ];
    for name in names {
        let path = cache_dir.join(&name);
        if !path.is_file() {
            continue;
        }
        let mut probe = plan.clone();
        probe.provider = format!("{}-cpython-source", plan.architecture);
        probe.package_name = name.clone();
        probe.package_version = version.to_string();
        probe.cache_path = path.clone();
        probe.download_url = format!("https://www.python.org/ftp/python/{version}/{name}");
        verify_package_digest(ctx, &probe, &path)?;
        return Ok(read_digest_sidecar(&path));
    }
    Ok(None)
}

pub(super) fn python_build_cache_is_cpython(plan: &InstallPlan) -> bool {
    let family = plan.family.trim().to_ascii_lowercase();
    if !family.is_empty() && family != "cpython" && family != "python" {
        return false;
    }
    let version = plan.resolved_version.trim().to_ascii_lowercase();
    if version.starts_with("pypy")
        || version.starts_with("miniconda")
        || version.starts_with("anaconda")
        || version.starts_with("stackless")
        || version.starts_with("graalpy")
        || version.starts_with("micropython")
        || version.starts_with("ironpython")
        || version.starts_with("jython")
    {
        return false;
    }
    true
}

pub(super) fn hash_file(path: &Path, alg: DigestAlg) -> Result<Vec<u8>, PyenvError> {
    let mut file = fs::File::open(path).map_err(io_error)?;
    let mut buffer = [0u8; 64 * 1024];
    match alg {
        DigestAlg::Sha256 => {
            let mut hasher = Sha256::new();
            loop {
                let read = file.read(&mut buffer).map_err(io_error)?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            }
            Ok(hasher.finalize().to_vec())
        }
        DigestAlg::Sha512 => {
            let mut hasher = Sha512::new();
            loop {
                let read = file.read(&mut buffer).map_err(io_error)?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            }
            Ok(hasher.finalize().to_vec())
        }
    }
}

fn digest_sidecar_path(path: &Path) -> PathBuf {
    let mut sidecar = path.as_os_str().to_os_string();
    sidecar.push(".digest");
    PathBuf::from(sidecar)
}

pub(super) fn write_digest_sidecar(
    path: &Path,
    expected: &ExpectedDigest,
) -> Result<(), PyenvError> {
    let body = format!(
        "{} {}\n# source {}\n",
        expected.alg.as_str(),
        hex::encode(&expected.bytes),
        expected.source
    );
    fs::write(digest_sidecar_path(path), body).map_err(io_error)
}

pub(super) fn read_digest_sidecar(path: &Path) -> Option<ExpectedDigest> {
    let text = fs::read_to_string(digest_sidecar_path(path)).ok()?;
    let mut alg = None;
    let mut hex_digest = None;
    let mut source = String::from("local digest sidecar");
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("# source ") {
            source = rest.trim().to_string();
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        alg = parts.next().and_then(DigestAlg::parse);
        hex_digest = parts.next().map(ToOwned::to_owned);
    }
    let bytes = hex::decode(hex_digest?).ok()?;
    Some(ExpectedDigest {
        alg: alg?,
        bytes,
        source,
    })
}

fn fetch_nuget_digest(plan: &InstallPlan) -> Result<ExpectedDigest, PyenvError> {
    match fetch_nuget_catalog_digest(plan) {
        Ok(digest) => Ok(digest),
        Err(catalog_error) => {
            // Sidecars next to the nupkg are only trusted for nuget.org hosts.
            // A hostile install.source_base_url must not supply both the package
            // and its checksum.
            if nuget_download_host_is_official(&plan.download_url)
                && let Ok(digest) = fetch_generic_sidecar_digest(&plan.download_url)
            {
                return Ok(digest);
            }
            Err(catalog_error)
        }
    }
}

fn nuget_download_host_is_official(download_url: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(download_url) else {
        return false;
    };
    if url.scheme() != "https" {
        return false;
    }
    let host = url.host_str().unwrap_or("").to_ascii_lowercase();
    host == "nuget.org" || host.ends_with(".nuget.org")
}

fn fetch_nuget_catalog_digest(plan: &InstallPlan) -> Result<ExpectedDigest, PyenvError> {
    let package = plan.package_name.to_ascii_lowercase();
    let version = plan.package_version.to_ascii_lowercase();
    let url = format!("{NUGET_REGISTRATION_BASE}/{package}/{version}.json");
    let body = http_get_text(&url)?;
    let registration = serde_json::from_str::<NugetRegistration>(&body).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to parse NuGet registration {url}: {error}"
        ))
    })?;

    let catalog_url = match registration.catalog_entry {
        NugetCatalogRef::Url(value) => value,
        NugetCatalogRef::Object(entry) => {
            return nuget_entry_to_digest(&entry, &url);
        }
    };
    let catalog_body = http_get_text(&catalog_url)?;
    let entry = serde_json::from_str::<NugetCatalogEntry>(&catalog_body).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to parse NuGet catalog {catalog_url}: {error}"
        ))
    })?;
    nuget_entry_to_digest(&entry, &catalog_url)
}

fn nuget_entry_to_digest(
    entry: &NugetCatalogEntry,
    source: &str,
) -> Result<ExpectedDigest, PyenvError> {
    let alg = DigestAlg::parse(entry.package_hash_algorithm.as_deref().unwrap_or("SHA512"))
        .ok_or_else(|| {
            PyenvError::MissingChecksum(format!("{} (unknown NuGet hash algorithm)", source))
        })?;
    let hash = entry.package_hash.as_deref().ok_or_else(|| {
        PyenvError::MissingChecksum(format!("{source} (NuGet catalog missing packageHash)"))
    })?;
    let bytes = BASE64.decode(hash.trim()).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: invalid NuGet packageHash at {source}: {error}"
        ))
    })?;
    Ok(ExpectedDigest {
        alg,
        bytes,
        source: source.to_string(),
    })
}

fn fetch_python_org_source_digest(plan: &InstallPlan) -> Result<ExpectedDigest, PyenvError> {
    let compact = plan.package_version.replace('.', "");
    let slug = format!("python-{compact}");
    let release_url = format!("{PYTHON_ORG_RELEASE_API}?slug={slug}");
    let release_body = http_get_text(&release_url)?;
    let releases = serde_json::from_str::<Vec<PythonRelease>>(&release_body).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to parse python.org release metadata {release_url}: {error}"
        ))
    })?;
    let release = releases.first().ok_or_else(|| {
        PyenvError::MissingChecksum(format!(
            "{} (python.org has no release slug {slug})",
            plan.download_url
        ))
    })?;
    let release_id = release
        .resource_uri
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .ok_or_else(|| {
            PyenvError::Io(format!(
                "pyenv: python.org release URI missing id: {}",
                release.resource_uri
            ))
        })?;
    let files_url = format!("{PYTHON_ORG_RELEASE_FILE_API}?release={release_id}");
    let files_body = http_get_text(&files_url)?;
    let files = serde_json::from_str::<Vec<PythonReleaseFile>>(&files_body).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to parse python.org release files {files_url}: {error}"
        ))
    })?;
    let wanted = plan.package_name.as_str();
    let file = files
        .iter()
        .find(|item| {
            item.url.ends_with(wanted)
                || item
                    .url
                    .rsplit('/')
                    .next()
                    .is_some_and(|name| name == wanted)
        })
        .ok_or_else(|| {
            PyenvError::MissingChecksum(format!(
                "{} (python.org release files did not include {wanted})",
                plan.download_url
            ))
        })?;
    let sha = file.sha256_sum.trim();
    if sha.len() != 64 || !sha.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(PyenvError::MissingChecksum(format!(
            "{} (python.org sha256_sum missing)",
            plan.download_url
        )));
    }
    Ok(ExpectedDigest {
        alg: DigestAlg::Sha256,
        bytes: hex::decode(sha).map_err(io_error_from_hex)?,
        source: files_url,
    })
}

fn fetch_pypy_digest(ctx: &AppContext, plan: &InstallPlan) -> Result<ExpectedDigest, PyenvError> {
    let cache_path = ctx.cache_dir().join("indexes").join("pypy-checksums.html");
    let html = load_or_fetch_text(&cache_path, PYPY_CHECKSUMS_URL)?;
    parse_pypy_checksums(&html, &plan.package_name).ok_or_else(|| {
        PyenvError::MissingChecksum(format!(
            "{} (no SHA-256 for {} on pypy.org/checksums.html)",
            plan.download_url, plan.package_name
        ))
    })
}

pub(super) fn parse_pypy_checksums(html: &str, filename: &str) -> Option<ExpectedDigest> {
    for line in html.lines() {
        let line = html_to_text(line);
        let mut parts = line.split_whitespace();
        let Some(hex_digest) = parts.next() else {
            continue;
        };
        let Some(name) = parts.next() else {
            continue;
        };
        if name == filename
            && hex_digest.len() == 64
            && hex_digest.chars().all(|ch| ch.is_ascii_hexdigit())
        {
            return Some(ExpectedDigest {
                alg: DigestAlg::Sha256,
                bytes: hex::decode(hex_digest).ok()?,
                source: PYPY_CHECKSUMS_URL.to_string(),
            });
        }
    }
    None
}

fn html_to_text(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_tag = false;
    for ch in line.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    html_unescape(&out)
}

fn html_unescape(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

fn fetch_generic_sidecar_digest(download_url: &str) -> Result<ExpectedDigest, PyenvError> {
    for (suffix, alg) in [
        (".sha256", DigestAlg::Sha256),
        (".sha512", DigestAlg::Sha512),
    ] {
        let url = format!("{download_url}{suffix}");
        if let Ok(body) = http_get_text(&url)
            && let Some(digest) = parse_sum_file(&body, alg, &url)
        {
            return Ok(digest);
        }
    }
    Err(PyenvError::MissingChecksum(download_url.to_string()))
}

fn parse_sum_file(body: &str, alg: DigestAlg, source: &str) -> Option<ExpectedDigest> {
    let token = body
        .split_whitespace()
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let bytes = if token.chars().all(|ch| ch.is_ascii_hexdigit())
        && (token.len() == 64 || token.len() == 128)
    {
        hex::decode(token).ok()?
    } else {
        BASE64.decode(token).ok()?
    };
    Some(ExpectedDigest {
        alg,
        bytes,
        source: source.to_string(),
    })
}

fn load_or_fetch_text(cache_path: &Path, url: &str) -> Result<String, PyenvError> {
    if cache_path.is_file()
        && super::fetch::cache_is_fresh_with_ttl(cache_path, PYPY_INDEX_TTL_SECS)
    {
        return fs::read_to_string(cache_path).map_err(io_error);
    }
    let body = http_get_text(url)?;
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    fs::write(cache_path, &body).map_err(io_error)?;
    Ok(body)
}

fn http_get_text(url: &str) -> Result<String, PyenvError> {
    let client = build_blocking_client()
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to build HTTP client: {error}")))?;
    client
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to query {url}: {error}")))?
        .text()
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to read {url}: {error}")))
}

fn io_error_from_hex(error: hex::FromHexError) -> PyenvError {
    PyenvError::Io(format!("pyenv: invalid hex digest: {error}"))
}

#[derive(Debug, Deserialize)]
struct NugetRegistration {
    #[serde(rename = "catalogEntry")]
    catalog_entry: NugetCatalogRef,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NugetCatalogRef {
    Url(String),
    Object(NugetCatalogEntry),
}

#[derive(Debug, Deserialize)]
struct NugetCatalogEntry {
    #[serde(rename = "packageHash")]
    package_hash: Option<String>,
    #[serde(rename = "packageHashAlgorithm")]
    package_hash_algorithm: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PythonRelease {
    resource_uri: String,
}

#[derive(Debug, Deserialize)]
struct PythonReleaseFile {
    url: String,
    #[serde(default)]
    sha256_sum: String,
}

#[cfg(test)]
mod tests {
    use super::{DigestAlg, hash_file, parse_pypy_checksums, parse_sum_file};
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn hashes_known_sha256_vector() {
        let mut file = NamedTempFile::new().expect("temp");
        file.write_all(b"abc").expect("write");
        let digest = hash_file(file.path(), DigestAlg::Sha256).expect("hash");
        assert_eq!(
            hex::encode(digest),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn parses_pypy_checksum_listing() {
        let html = r#"
<p>pypy3.11-v7.3.23 sha256:</p>
<pre>
2bcab031cef7a37fe1930b51f7091e78a191ae63f80eca00a265d3378c3a645b  pypy3.11-v7.3.23-linux64.tar.bz2
</pre>
"#;
        let digest = parse_pypy_checksums(html, "pypy3.11-v7.3.23-linux64.tar.bz2").expect("row");
        assert_eq!(digest.alg, DigestAlg::Sha256);
        assert_eq!(
            hex::encode(digest.bytes),
            "2bcab031cef7a37fe1930b51f7091e78a191ae63f80eca00a265d3378c3a645b"
        );
    }

    #[test]
    fn parses_sha256sum_sidecar() {
        let digest = parse_sum_file(
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  file.bin\n",
            DigestAlg::Sha256,
            "https://example.invalid/file.bin.sha256",
        )
        .expect("sidecar");
        assert_eq!(
            hex::encode(digest.bytes),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    fn sample_plan(cache_path: &std::path::Path, url: &str) -> super::super::types::InstallPlan {
        super::super::types::InstallPlan {
            requested_version: "3.14.0".to_string(),
            resolved_version: "3.14.0".to_string(),
            family: "CPython".to_string(),
            provider: "linux-cpython-python-build".to_string(),
            architecture: "x64".to_string(),
            runtime_version: "3.14.0".to_string(),
            free_threaded: false,
            package_name: "Python-3.14.0.tgz".to_string(),
            package_version: "3.14.0".to_string(),
            download_url: url.to_string(),
            cache_path: cache_path.to_path_buf(),
            install_dir: cache_path.parent().unwrap().join("versions"),
            python_executable: cache_path.parent().unwrap().join("python"),
            bootstrap_pip: false,
            create_base_venv: false,
            base_venv_path: None,
        }
    }

    #[test]
    fn python_build_url_uses_sidecar_when_publisher_is_unavailable() {
        use super::{DigestAlg, ExpectedDigest, verify_package_digest, write_digest_sidecar};
        use crate::config::AppConfig;
        use crate::context::AppContext;

        let temp = tempfile::TempDir::new().expect("tempdir");
        let cache = temp.path().join("pkg.tgz");
        std::fs::write(&cache, b"abc").expect("cache");
        write_digest_sidecar(
            &cache,
            &ExpectedDigest {
                alg: DigestAlg::Sha256,
                bytes: hex::decode(
                    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
                )
                .expect("hex"),
                source: "test sidecar".to_string(),
            },
        )
        .expect("sidecar");

        let ctx = AppContext {
            root: temp.path().join(".pyenv"),
            dir: temp.path().join("work"),
            exe_path: std::path::PathBuf::from("pyenv"),
            env_version: None,
            env_shell: None,
            path_env: None,
            path_ext: None,
            config: AppConfig::default(),
        };
        let plan = sample_plan(&cache, "python-build://3.14.0");
        verify_package_digest(&ctx, &plan, &cache).expect("sidecar fallback");
    }

    #[test]
    fn python_build_url_without_sidecar_is_missing_checksum() {
        use super::verify_package_digest;
        use crate::config::AppConfig;
        use crate::context::AppContext;
        use crate::error::PyenvError;

        let temp = tempfile::TempDir::new().expect("tempdir");
        let cache = temp.path().join("pkg.tgz");
        std::fs::write(&cache, b"abc").expect("cache");
        let ctx = AppContext {
            root: temp.path().join(".pyenv"),
            dir: temp.path().join("work"),
            exe_path: std::path::PathBuf::from("pyenv"),
            env_version: None,
            env_shell: None,
            path_env: None,
            path_ext: None,
            config: AppConfig::default(),
        };
        let plan = sample_plan(&cache, "python-build://3.14.0");
        let error = verify_package_digest(&ctx, &plan, &cache).expect_err("missing");
        assert!(matches!(error, PyenvError::MissingChecksum(_)));
    }

    #[test]
    fn sidecar_mismatch_fails_closed() {
        use super::{DigestAlg, ExpectedDigest, verify_package_digest, write_digest_sidecar};
        use crate::config::AppConfig;
        use crate::context::AppContext;
        use crate::error::PyenvError;

        let temp = tempfile::TempDir::new().expect("tempdir");
        let cache = temp.path().join("pkg.tgz");
        std::fs::write(&cache, b"abc").expect("cache");
        write_digest_sidecar(
            &cache,
            &ExpectedDigest {
                alg: DigestAlg::Sha256,
                bytes: vec![0u8; 32],
                source: "stale sidecar".to_string(),
            },
        )
        .expect("sidecar");
        let ctx = AppContext {
            root: temp.path().join(".pyenv"),
            dir: temp.path().join("work"),
            exe_path: std::path::PathBuf::from("pyenv"),
            env_version: None,
            env_shell: None,
            path_env: None,
            path_ext: None,
            config: AppConfig::default(),
        };
        let plan = sample_plan(&cache, "python-build://3.14.0");
        let error = verify_package_digest(&ctx, &plan, &cache).expect_err("mismatch");
        assert!(matches!(error, PyenvError::ChecksumMismatch { .. }));
    }

    #[test]
    fn python_build_cache_returns_none_when_tarball_missing() {
        use super::verify_python_build_cache;
        use crate::config::AppConfig;
        use crate::context::AppContext;

        let temp = tempfile::TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        std::fs::create_dir_all(root.join("cache")).expect("cache");
        let ctx = AppContext {
            root,
            dir: temp.path().join("work"),
            exe_path: std::path::PathBuf::from("pyenv"),
            env_version: None,
            env_shell: None,
            path_env: None,
            path_ext: None,
            config: AppConfig::default(),
        };
        let plan = sample_plan(&temp.path().join("missing.tgz"), "python-build://3.14.0");
        assert!(
            verify_python_build_cache(&ctx, &plan)
                .expect("ok")
                .is_none()
        );
    }

    #[test]
    fn nuget_sidecar_is_trusted_only_for_official_https_hosts() {
        use super::nuget_download_host_is_official;

        assert!(nuget_download_host_is_official(
            "https://globalcdn.nuget.org/packages/python.3.14.0.nupkg"
        ));
        assert!(nuget_download_host_is_official(
            "https://www.nuget.org/api/v2/package/python/3.14.0"
        ));
        assert!(!nuget_download_host_is_official(
            "https://evil.example/python.3.14.0.nupkg"
        ));
        assert!(!nuget_download_host_is_official(
            "http://globalcdn.nuget.org/packages/python.3.14.0.nupkg"
        ));
        assert!(!nuget_download_host_is_official("not-a-url"));
    }
}
