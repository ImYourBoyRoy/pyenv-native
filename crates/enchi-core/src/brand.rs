// ./crates/enchi-core/src/brand.rs
//! Product naming in one place: display name, command names, environment variables, install
//! directories and release-asset prefixes. Everything that used to be spelled `pyenv`/`pyenv-native`
//! in code goes through these constants, with the previous names kept as `LEGACY_*` so existing
//! installs keep working.
//!
//! The GUI keeps its own copy of the display name in `crates/enchi-gui/ui/brand.js`.

/// Display name used in messages and UI.
pub const NAME: &str = "Enchi";

/// Base names of the executables (without `.exe`).
pub const CLI: &str = "enchi";
pub const MCP: &str = "enchi-mcp";
pub const GUI: &str = "enchi-gui";

/// Names earlier releases (pyenv-native) installed. Kept as aliases when upgrading an install and
/// accepted wherever a binary name is recognised.
pub const LEGACY_CLI: &str = "pyenv";
pub const LEGACY_MCP: &str = "pyenv-mcp";
pub const LEGACY_GUI: &str = "pyenv-gui";

/// Install-root environment variable, and the classic-pyenv variable still honoured as a fallback.
pub const ROOT_ENV: &str = "ENCHI_ROOT";
pub const LEGACY_ROOT_ENV: &str = "PYENV_ROOT";

/// Default root directory name under the home directory for fresh installs, and the one existing
/// pyenv-native installs already use (they keep it; nothing is moved automatically).
pub const ROOT_DIR: &str = ".enchi";
pub const LEGACY_ROOT_DIR: &str = ".pyenv";

/// GitHub repository that publishes releases. The repository keeps its old name until the rename
/// is done on GitHub (old URLs redirect); changing this is then a one-line edit.
pub const RELEASE_REPO: &str = "imyourboyroy/pyenv-native";

/// Release-asset name prefixes: `<prefix>-<platform>-<arch>.<ext>`.
pub const ASSET_PREFIX: &str = "enchi";
pub const LEGACY_ASSET_PREFIX: &str = "pyenv-native";

/// Environment variable that redirects release-asset downloads (mirrors, local testing), and the
/// previous name, still accepted.
pub const RELEASE_BASE_URL_ENV: &str = "ENCHI_RELEASE_BASE_URL";
pub const LEGACY_RELEASE_BASE_URL_ENV: &str = "PYENV_NATIVE_RELEASE_BASE_URL";

/// Marker fenced around the block this tool writes into shell profiles, and the earlier one.
pub const PROFILE_BEGIN: &str = "# >>> enchi init >>>";
pub const PROFILE_END: &str = "# <<< enchi init <<<";
pub const LEGACY_PROFILE_BEGIN: &str = "# >>> pyenv-native init >>>";
pub const LEGACY_PROFILE_END: &str = "# <<< pyenv-native init <<<";

/// Values for `{ $brand }` / `{ $command }` in the message catalogs.
pub const CATALOG_ARGS: [(&str, &str); 2] = [("brand", NAME), ("command", CLI)];

/// `base` with the platform executable suffix.
pub fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

/// Whether `stem` (a file name without extension) is one of this tool's command-line binaries,
/// current or legacy. Companion binaries (`*-gui`, `*-mcp`) are not.
pub fn is_cli_name(stem: &str) -> bool {
    let lowered = stem.to_ascii_lowercase();
    matches!(
        lowered.as_str(),
        CLI | LEGACY_CLI | "enchi-cli" | "pyenv-cli"
    )
}

/// Whether `stem` is a companion (GUI or MCP) binary, current or legacy.
pub fn is_companion_name(stem: &str) -> bool {
    let lowered = stem.to_ascii_lowercase();
    matches!(lowered.as_str(), GUI | MCP | LEGACY_GUI | LEGACY_MCP)
}

/// The install-root variable as set in the environment: `ENCHI_ROOT`, else `PYENV_ROOT`.
pub fn root_from_env() -> Option<std::ffi::OsString> {
    [ROOT_ENV, LEGACY_ROOT_ENV]
        .iter()
        .filter_map(std::env::var_os)
        .find(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_and_legacy_cli_names_are_recognised() {
        for name in ["enchi", "pyenv", "ENCHI", "enchi-cli", "pyenv-cli"] {
            assert!(is_cli_name(name), "{name}");
        }
        for name in ["enchi-gui", "enchi-mcp", "python", "pyenvx"] {
            assert!(!is_cli_name(name), "{name}");
        }
    }

    #[test]
    fn companions_are_not_cli() {
        for name in ["enchi-gui", "enchi-mcp", "pyenv-gui", "pyenv-mcp"] {
            assert!(is_companion_name(name), "{name}");
            assert!(!is_cli_name(name), "{name}");
        }
    }

    #[test]
    fn exe_name_adds_the_platform_suffix() {
        assert_eq!(exe_name("enchi").ends_with(".exe"), cfg!(windows));
    }
}
