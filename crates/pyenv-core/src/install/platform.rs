// ./crates/pyenv-core/src/install/platform.rs
//! Shared platform/provider naming and path-shape helpers for install operations.

use std::env;
use std::path::{Path, PathBuf};

use crate::config::RuntimeArch;

pub(super) fn current_platform() -> &'static str {
    env::consts::OS
}

pub(super) fn is_windows_platform(platform: &str) -> bool {
    platform.eq_ignore_ascii_case("windows")
}

pub(super) fn python_build_provider_name(platform: &str) -> String {
    format!("{platform}-python-build")
}

pub(super) fn cpython_source_provider_name(platform: &str) -> Option<&'static str> {
    match platform {
        "linux" => Some("linux-cpython-source"),
        "macos" => Some("macos-cpython-source"),
        "android" => Some("android-cpython-source"),
        _ => None,
    }
}

pub(super) fn pypy_provider_name(platform: &str) -> Option<&'static str> {
    match platform {
        "windows" => Some("windows-pypy-downloads"),
        "linux" => Some("linux-pypy-downloads"),
        "macos" => Some("macos-pypy-downloads"),
        _ => None,
    }
}

pub(super) fn family_filter_matches_provider(
    filter: &str,
    family_slug: &str,
    family_label: &str,
    provider: Option<&str>,
) -> bool {
    filter == family_slug
        || filter == family_label.to_ascii_lowercase()
        || provider.is_some_and(|provider_name| provider_name.eq_ignore_ascii_case(filter))
}

pub(super) fn pypy_manifest_platform(platform: &str) -> Option<&'static str> {
    match platform {
        "windows" => Some("win64"),
        "linux" => Some("linux"),
        "macos" => Some("darwin"),
        _ => None,
    }
}

pub(super) fn pypy_manifest_arches(arch: RuntimeArch, platform: &str) -> &'static [&'static str] {
    match (platform, arch) {
        ("windows", RuntimeArch::X64 | RuntimeArch::Auto) => &["x64"],
        ("linux", RuntimeArch::X64 | RuntimeArch::Auto) => &["x64"],
        ("linux", RuntimeArch::X86) => &["i686", "x86"],
        ("linux", RuntimeArch::Arm64) => &["aarch64", "arm64"],
        ("macos", RuntimeArch::X64 | RuntimeArch::Auto) => &["x64"],
        ("macos", RuntimeArch::Arm64) => &["arm64", "aarch64"],
        _ => &[],
    }
}

pub(super) fn pypy_python_executable_path(install_dir: &Path, platform: &str) -> PathBuf {
    if is_windows_platform(platform) {
        install_dir.join("python.exe")
    } else {
        install_dir.join("bin").join("pypy3")
    }
}

pub(super) fn cpython_source_python_executable_path(install_dir: &Path) -> PathBuf {
    install_dir.join("bin").join("python")
}
