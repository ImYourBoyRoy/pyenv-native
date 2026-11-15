// ./xtask/src/brand.rs
//! Product naming for the release automation, in one place.
//!
//! Mirrors `crates/enchi-core/src/brand.rs`: the current names are `enchi*`, and the names earlier
//! releases (pyenv-native) shipped are kept as `LEGACY_*` so the bridge release can still publish
//! what already-installed binaries download.

/// Display name used in installers, desktop entries, and formulas.
pub const NAME: &str = "Enchi";

/// Executable base names (without `.exe`).
pub const CLI: &str = "enchi";
pub const MCP: &str = "enchi-mcp";
pub const GUI: &str = "enchi-gui";

/// Names earlier releases installed.
pub const LEGACY_CLI: &str = "pyenv";
pub const LEGACY_MCP: &str = "pyenv-mcp";

/// Release-asset prefixes: `<prefix>-<platform>-<arch>.<ext>`.
pub const ASSET_PREFIX: &str = "enchi";
pub const LEGACY_ASSET_PREFIX: &str = "pyenv-native";

/// System package name, and the name it replaces.
pub const PACKAGE: &str = CLI;
pub const LEGACY_PACKAGE: &str = "pyenv-native";

/// Linux desktop id, macOS app bundle id, and Tauri identifier.
pub const APP_ID: &str = "io.github.imyourboyroy.enchi";
/// macOS installer receipt identifier; `enchi self-uninstall --system` reads the same value.
pub const PKG_IDENTIFIER: &str = "io.github.imyourboyroy.enchi.pkg";
/// Reverse-DNS organisation for the macOS installer.
pub const ORGANIZATION: &str = "io.github.imyourboyroy";

/// The GitHub repository that publishes releases. The repository keeps its old name until it is
/// renamed on GitHub (old URLs redirect); changing this one constant is then the whole edit here.
pub const GITHUB_REPO: &str = "imyourboyroy/pyenv-native";

/// `https://github.com/<owner>/<repo>`.
pub fn repo_url() -> String {
    format!("https://github.com/{GITHUB_REPO}")
}

/// `enchi-<platform>-<arch>`: the current bundle (and archive) stem.
pub fn bundle_stem(platform: &str, architecture: &str) -> String {
    format!("{ASSET_PREFIX}-{platform}-{architecture}")
}

/// The legacy bundle name that matches a current one: `enchi-linux-x64` -> `pyenv-native-linux-x64`.
/// Names that do not follow the `enchi-` pattern (custom `--bundle-name`) get a `-legacy` suffix.
pub fn legacy_bundle_name(name: &str) -> String {
    match name.strip_prefix(&format!("{ASSET_PREFIX}-")) {
        Some(rest) => format!("{LEGACY_ASSET_PREFIX}-{rest}"),
        None => format!("{name}-legacy"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stems_and_legacy_names_line_up() {
        assert_eq!(bundle_stem("linux", "x64"), "enchi-linux-x64");
        assert_eq!(
            legacy_bundle_name("enchi-windows-arm64"),
            "pyenv-native-windows-arm64"
        );
        assert_eq!(legacy_bundle_name("custom"), "custom-legacy");
    }

    #[test]
    fn repo_url_follows_the_repository_constant() {
        assert_eq!(repo_url(), format!("https://github.com/{GITHUB_REPO}"));
    }
}
