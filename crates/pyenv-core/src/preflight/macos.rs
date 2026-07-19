// ./crates/pyenv-core/src/preflight/macos.rs
//! macOS Xcode/CLT and Homebrew OpenSSL discovery used by doctor, preflight, and source builds.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::process::PyenvCommandExt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacosToolchainState {
    pub clt_path: Option<PathBuf>,
    pub clt_ok: bool,
    pub clt_detail: String,
    pub xcodebuild_version: Option<String>,
    pub openssl_prefix: Option<PathBuf>,
    pub openssl_detail: String,
    pub brew_available: bool,
}

pub(crate) fn inspect_macos_toolchain() -> MacosToolchainState {
    let clt = detect_xcode_clt();
    let xcodebuild_version = detect_xcodebuild_version();
    let brew_available = command_exists("brew");
    let openssl = detect_openssl_prefix(brew_available);

    MacosToolchainState {
        clt_path: clt.path.clone(),
        clt_ok: clt.ok,
        clt_detail: clt.detail,
        xcodebuild_version,
        openssl_prefix: openssl.prefix,
        openssl_detail: openssl.detail,
        brew_available,
    }
}

struct CltProbe {
    path: Option<PathBuf>,
    ok: bool,
    detail: String,
}

fn detect_xcode_clt() -> CltProbe {
    let output = Command::new("xcode-select").headless().arg("-p").output();

    match output {
        Ok(out) if out.status.success() => {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if path.is_empty() {
                return CltProbe {
                    path: None,
                    ok: false,
                    detail: "xcode-select -p returned an empty developer directory".to_string(),
                };
            }
            let path_buf = PathBuf::from(&path);
            let sdk_ok = path_buf.join("SDKs").is_dir()
                || path_buf
                    .join("Platforms/MacOSX.platform/Developer/SDKs")
                    .is_dir()
                || Path::new("/Library/Developer/CommandLineTools/SDKs").is_dir();
            if path_buf.exists() {
                CltProbe {
                    path: Some(path_buf),
                    ok: true,
                    detail: if sdk_ok {
                        format!("Xcode/CLT developer directory available at {path}")
                    } else {
                        format!(
                            "Xcode/CLT developer directory found at {path}; SDK layout looks incomplete"
                        )
                    },
                }
            } else {
                CltProbe {
                    path: Some(path_buf),
                    ok: false,
                    detail: format!(
                        "xcode-select points to {path}, but that path is missing; reinstall Command Line Tools"
                    ),
                }
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            CltProbe {
                path: None,
                ok: false,
                detail: if stderr.is_empty() {
                    "Xcode Command Line Tools are not installed (`xcode-select -p` failed)"
                        .to_string()
                } else {
                    format!("Xcode Command Line Tools are not ready: {stderr}")
                },
            }
        }
        Err(error) => CltProbe {
            path: None,
            ok: false,
            detail: format!("unable to run xcode-select: {error}"),
        },
    }
}

fn detect_xcodebuild_version() -> Option<String> {
    let output = Command::new("xcodebuild")
        .headless()
        .arg("-version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().next().map(|line| line.trim().to_string())
}

struct OpensslProbe {
    prefix: Option<PathBuf>,
    detail: String,
}

fn detect_openssl_prefix(brew_available: bool) -> OpensslProbe {
    let mut candidates = Vec::new();
    if brew_available {
        for formula in ["openssl@3", "openssl"] {
            if let Some(prefix) = brew_prefix(formula) {
                candidates.push(prefix);
            }
        }
    }
    candidates.extend([
        PathBuf::from("/opt/homebrew/opt/openssl@3"),
        PathBuf::from("/opt/homebrew/opt/openssl"),
        PathBuf::from("/usr/local/opt/openssl@3"),
        PathBuf::from("/usr/local/opt/openssl"),
    ]);

    for prefix in candidates {
        let include = prefix.join("include/openssl/ssl.h");
        if include.is_file() {
            return OpensslProbe {
                prefix: Some(prefix.clone()),
                detail: format!(
                    "OpenSSL headers available at {} (required for TLS-capable CPython builds)",
                    prefix.display()
                ),
            };
        }
    }

    OpensslProbe {
        prefix: None,
        detail: if brew_available {
            "OpenSSL development headers were not found; install with `brew install openssl@3`"
                .to_string()
        } else {
            "OpenSSL development headers were not found and Homebrew is unavailable; install Homebrew then `brew install openssl@3`".to_string()
        },
    }
}

pub(crate) fn macos_source_build_env() -> Vec<(String, String)> {
    let state = inspect_macos_toolchain();
    let mut pairs = Vec::new();

    let mut cppflags = Vec::new();
    let mut ldflags = Vec::new();
    let mut pkg_paths = Vec::new();

    if let Some(prefix) = state.openssl_prefix {
        cppflags.push(format!("-I{}/include", prefix.display()));
        ldflags.push(format!("-L{}/lib", prefix.display()));
        let pkg = prefix.join("lib/pkgconfig");
        if pkg.is_dir() {
            pkg_paths.push(pkg.display().to_string());
        }
    }

    for formula in ["readline", "sqlite", "xz", "zlib", "bzip2"] {
        if let Some(prefix) = brew_prefix(formula).or_else(|| homebrew_opt_fallback(formula)) {
            let include = prefix.join("include");
            let lib = prefix.join("lib");
            if include.is_dir() {
                cppflags.push(format!("-I{}", include.display()));
            }
            if lib.is_dir() {
                ldflags.push(format!("-L{}", lib.display()));
            }
            let pkg = lib.join("pkgconfig");
            if pkg.is_dir() {
                pkg_paths.push(pkg.display().to_string());
            }
        }
    }

    if !cppflags.is_empty() {
        pairs.push((
            "CPPFLAGS".to_string(),
            merge_env_flags(std::env::var("CPPFLAGS").ok().as_deref(), &cppflags),
        ));
    }
    if !ldflags.is_empty() {
        pairs.push((
            "LDFLAGS".to_string(),
            merge_env_flags(std::env::var("LDFLAGS").ok().as_deref(), &ldflags),
        ));
    }
    if !pkg_paths.is_empty() {
        pairs.push((
            "PKG_CONFIG_PATH".to_string(),
            merge_path_list(std::env::var("PKG_CONFIG_PATH").ok().as_deref(), &pkg_paths),
        ));
    }

    pairs
}

fn brew_prefix(formula: &str) -> Option<PathBuf> {
    let output = Command::new("brew")
        .headless()
        .args(["--prefix", formula])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return None;
    }
    let path = PathBuf::from(path);
    path.exists().then_some(path)
}

fn homebrew_opt_fallback(formula: &str) -> Option<PathBuf> {
    for root in ["/opt/homebrew/opt", "/usr/local/opt"] {
        let candidate = PathBuf::from(root).join(formula);
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

fn merge_env_flags(existing: Option<&str>, additions: &[String]) -> String {
    let mut parts = Vec::new();
    if let Some(existing) = existing.map(str::trim).filter(|value| !value.is_empty()) {
        parts.push(existing.to_string());
    }
    for addition in additions {
        if !parts.iter().any(|part| part == addition) {
            parts.push(addition.clone());
        }
    }
    parts.join(" ")
}

fn merge_path_list(existing: Option<&str>, additions: &[String]) -> String {
    let mut parts = Vec::new();
    if let Some(existing) = existing.map(str::trim).filter(|value| !value.is_empty()) {
        for part in existing.split(':') {
            if !part.is_empty() {
                parts.push(part.to_string());
            }
        }
    }
    for addition in additions {
        if !parts.iter().any(|part| part == addition) {
            parts.push(addition.clone());
        }
    }
    parts.join(":")
}

fn command_exists(name: &str) -> bool {
    Command::new(name)
        .headless()
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// Best-effort automated Command Line Tools install/update for macOS.
/// Full Xcode.app still requires the App Store; CLT can often be refreshed via softwareupdate.
pub(crate) fn try_install_or_update_macos_clt() -> Result<String, String> {
    let before = inspect_macos_toolchain();
    if before.clt_ok {
        if let Some(update_name) = find_clt_softwareupdate_label() {
            let output = Command::new("softwareupdate")
                .headless()
                .args(["-i", &update_name, "--agree-to-license"])
                .output()
                .map_err(|error| format!("failed to run softwareupdate: {error}"))?;
            if output.status.success() {
                return Ok(format!(
                    "Installed/updated macOS Command Line Tools via softwareupdate ({update_name})"
                ));
            }
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(format!(
                "softwareupdate could not install `{update_name}`: {stderr}"
            ));
        }
        return Ok(
            "Xcode Command Line Tools already look installed; no CLT softwareupdate was pending"
                .to_string(),
        );
    }

    if let Some(update_name) = find_clt_softwareupdate_label() {
        let output = Command::new("softwareupdate")
            .headless()
            .args(["-i", &update_name, "--agree-to-license"])
            .output()
            .map_err(|error| format!("failed to run softwareupdate: {error}"))?;
        if output.status.success() {
            return Ok(format!(
                "Installed macOS Command Line Tools via softwareupdate ({update_name})"
            ));
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "softwareupdate failed for `{update_name}`: {stderr}. Fallback: run `xcode-select --install` (may show a system dialog)."
        ));
    }

    let output = Command::new("xcode-select")
        .headless()
        .arg("--install")
        .output()
        .map_err(|error| format!("failed to run xcode-select --install: {error}"))?;
    if output.status.success() {
        return Ok(
            "Triggered `xcode-select --install`. macOS may show a system dialog to finish Command Line Tools setup."
                .to_string(),
        );
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(format!(
        "unable to auto-install Command Line Tools ({stderr}). Install the latest Xcode-compatible Command Line Tools for this macOS release, then re-run `pyenv preflight`."
    ))
}

fn find_clt_softwareupdate_label() -> Option<String> {
    let output = Command::new("softwareupdate")
        .headless()
        .arg("--list")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    for line in combined.lines() {
        let trimmed = line.trim().trim_start_matches('*').trim();
        let label = trimmed
            .strip_prefix("Label:")
            .map(str::trim)
            .unwrap_or(trimmed);
        if label.to_ascii_lowercase().contains("command line tools") {
            return Some(label.to_string());
        }
    }
    None
}
