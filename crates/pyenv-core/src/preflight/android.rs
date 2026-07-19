// ./crates/pyenv-core/src/preflight/android.rs
//! Termux/Android source-build readiness helpers for doctor, preflight, and CPython builds.

use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AndroidToolchainState {
    pub is_termux: bool,
    pub prefix: Option<PathBuf>,
    pub api_level: Option<u32>,
    pub ready: bool,
    pub missing: Vec<String>,
    pub detail: String,
}

pub(crate) fn is_termux_environment() -> bool {
    env::var_os("TERMUX_VERSION").is_some()
        || env::var_os("PREFIX")
            .map(|value| value.to_string_lossy().contains("/data/data/com.termux"))
            .unwrap_or(false)
        || Path::new("/data/data/com.termux/files/usr").is_dir()
}

pub(crate) fn resolve_termux_prefix() -> Option<PathBuf> {
    if let Some(prefix) = env::var_os("PREFIX").map(PathBuf::from)
        && prefix.is_dir()
    {
        return Some(prefix);
    }
    let fallback = PathBuf::from("/data/data/com.termux/files/usr");
    fallback.is_dir().then_some(fallback)
}

pub(crate) fn detect_android_api_level() -> Option<u32> {
    for key in [
        "TERMUX_PKG_API_LEVEL",
        "ANDROID_API_LEVEL",
        "TERMUX_API_LEVEL",
    ] {
        if let Ok(value) = env::var(key)
            && let Ok(parsed) = value.trim().parse::<u32>()
        {
            return Some(parsed);
        }
    }

    let output = std::process::Command::new("getprop")
        .arg("ro.build.version.sdk")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

pub(crate) fn inspect_android_toolchain() -> AndroidToolchainState {
    let is_termux = is_termux_environment();
    if !is_termux && cfg!(not(target_os = "android")) {
        return AndroidToolchainState {
            is_termux: false,
            prefix: None,
            api_level: None,
            ready: true,
            missing: Vec::new(),
            detail: "not running under Termux/Android".to_string(),
        };
    }

    let prefix = resolve_termux_prefix();
    let api_level = detect_android_api_level();
    let Some(prefix) = prefix else {
        return AndroidToolchainState {
            is_termux,
            prefix: None,
            api_level,
            ready: false,
            missing: vec!["termux-prefix".to_string()],
            detail: "Termux PREFIX was not found; open Termux and ensure $PREFIX points at /data/data/com.termux/files/usr".to_string(),
        };
    };

    let required = [
        ("clang", prefix.join("bin/clang")),
        ("make", prefix.join("bin/make")),
        ("pkg-config", prefix.join("bin/pkg-config")),
        ("libffi", prefix.join("include/ffi.h")),
        ("openssl", prefix.join("include/openssl/ssl.h")),
        ("readline", prefix.join("include/readline/readline.h")),
        ("ncurses", prefix.join("include/ncurses.h")),
        ("sqlite", prefix.join("include/sqlite3.h")),
        ("zlib", prefix.join("include/zlib.h")),
        ("bzip2", prefix.join("include/bzlib.h")),
        ("xz", prefix.join("include/lzma.h")),
    ];

    let mut missing = Vec::new();
    for (name, path) in required {
        let alt_ok = match name {
            "libffi" => prefix.join("include/ffi/ffi.h").is_file(),
            "ncurses" => prefix.join("include/ncursesw/ncurses.h").is_file(),
            _ => false,
        };
        if !(path.is_file() || alt_ok) {
            missing.push(name.to_string());
        }
    }

    let ready = missing.is_empty();
    let detail = if ready {
        format!(
            "Termux source-build prefix ready at {}{}",
            prefix.display(),
            api_level
                .map(|level| format!(" (API {level})"))
                .unwrap_or_default()
        )
    } else {
        format!(
            "Termux is missing packages/headers for: {}. Run `pkg install clang make pkg-config libffi openssl readline ncurses sqlite zlib bzip2 xz -y`",
            missing.join(", ")
        )
    };

    AndroidToolchainState {
        is_termux: true,
        prefix: Some(prefix),
        api_level,
        ready,
        missing,
        detail,
    }
}

pub(crate) fn termux_required_pkg_packages() -> &'static [&'static str] {
    &[
        "clang",
        "make",
        "pkg-config",
        "libffi",
        "openssl",
        "readline",
        "ncurses",
        "sqlite",
        "zlib",
        "bzip2",
        "xz",
    ]
}

pub(crate) fn android_source_build_env(
    prefix: Option<&Path>,
    api_level: Option<u32>,
) -> Vec<(String, String)> {
    let mut env_pairs = Vec::new();

    if let Some(prefix) = prefix {
        let include_dir = prefix.join("include");
        let lib_dir = prefix.join("lib");
        env_pairs.push((
            "CPPFLAGS".to_string(),
            append_shell_flag(
                env::var("CPPFLAGS").ok().as_deref(),
                &format!("-I{}", include_dir.display()),
            ),
        ));
        env_pairs.push((
            "LDFLAGS".to_string(),
            append_shell_flag(
                env::var("LDFLAGS").ok().as_deref(),
                &format!("-L{} -Wl,-rpath,{}", lib_dir.display(), lib_dir.display()),
            ),
        ));
        env_pairs.push(("LIBCRYPT_LIBS".to_string(), "-lcrypt".to_string()));
        let pkg = lib_dir.join("pkgconfig");
        if pkg.is_dir() {
            env_pairs.push((
                "PKG_CONFIG_PATH".to_string(),
                append_path_entry(
                    env::var("PKG_CONFIG_PATH").ok().as_deref(),
                    &pkg.display().to_string(),
                ),
            ));
        }
    }

    if let Some(api_level) = api_level {
        if api_level < 28 {
            env_pairs.push(("ac_cv_func_fexecve".to_string(), "no".to_string()));
            env_pairs.push(("ac_cv_func_getlogin_r".to_string(), "no".to_string()));
        }
        if api_level < 29 {
            env_pairs.push(("ac_cv_func_getloadavg".to_string(), "no".to_string()));
        }
        if api_level < 30 {
            env_pairs.push(("ac_cv_func_sem_clockwait".to_string(), "no".to_string()));
        }
        if api_level < 33 {
            env_pairs.push(("ac_cv_func_preadv2".to_string(), "no".to_string()));
            env_pairs.push(("ac_cv_func_pwritev2".to_string(), "no".to_string()));
        }
        if api_level < 34 {
            env_pairs.push(("ac_cv_func_close_range".to_string(), "no".to_string()));
            env_pairs.push(("ac_cv_func_copy_file_range".to_string(), "no".to_string()));
        }
    }

    env_pairs
}

fn append_shell_flag(existing: Option<&str>, addition: &str) -> String {
    match existing.map(str::trim).filter(|value| !value.is_empty()) {
        Some(existing) => format!("{existing} {addition}"),
        None => addition.to_string(),
    }
}

fn append_path_entry(existing: Option<&str>, addition: &str) -> String {
    match existing.map(str::trim).filter(|value| !value.is_empty()) {
        Some(existing) if existing.split(':').any(|part| part == addition) => existing.to_string(),
        Some(existing) => format!("{addition}:{existing}"),
        None => addition.to_string(),
    }
}
