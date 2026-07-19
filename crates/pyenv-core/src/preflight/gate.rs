// ./crates/pyenv-core/src/preflight/gate.rs
//! Hard preflight gate used before source compiles so installs fail early with actionable errors.

use crate::error::PyenvError;

use super::android::inspect_android_toolchain;
use super::macos::inspect_macos_toolchain;

pub fn ensure_source_build_ready(platform: &str) -> Result<(), PyenvError> {
    match platform {
        "macos" => ensure_macos_ready(),
        "android" => ensure_android_ready(),
        _ if cfg!(target_os = "android") => ensure_android_ready(),
        _ => Ok(()),
    }
}

fn ensure_macos_ready() -> Result<(), PyenvError> {
    let state = inspect_macos_toolchain();
    let mut problems = Vec::new();
    if !state.clt_ok {
        problems.push(format!(
            "Xcode Command Line Tools: {}. Run `pyenv doctor --fix` or `xcode-select --install`, then `pyenv preflight`.",
            state.clt_detail
        ));
    }
    if state.openssl_prefix.is_none() {
        problems.push(format!(
            "OpenSSL TLS headers: {}. Install with `brew install openssl@3` before building CPython.",
            state.openssl_detail
        ));
    }
    if problems.is_empty() {
        return Ok(());
    }
    Err(PyenvError::Io(format!(
        "pyenv: macOS source-build preflight failed:\n  - {}",
        problems.join("\n  - ")
    )))
}

fn ensure_android_ready() -> Result<(), PyenvError> {
    let state = inspect_android_toolchain();
    if state.ready {
        return Ok(());
    }
    Err(PyenvError::Io(format!(
        "pyenv: Android/Termux source-build preflight failed: {}. Install missing packages, then re-run `pyenv preflight`.",
        state.detail
    )))
}
