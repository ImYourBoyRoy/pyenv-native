// ./crates/pyenv-core/src/preflight/mod.rs
//! Platform intelligence and install preflight for CLI, MCP, GUI, and local agents.

mod android;
mod gate;
mod intel;
mod macos;
mod report;
mod types;

#[cfg(test)]
mod tests;

pub use intel::build_platform_intelligence;
pub use report::{cmd_environment, cmd_preflight};
pub use types::{PlatformFact, PlatformIntelligence, PreflightVerdict};

pub(crate) use android::{
    android_source_build_env, detect_android_api_level, inspect_android_toolchain,
    is_termux_environment, resolve_termux_prefix, termux_required_pkg_packages,
};
pub(crate) use gate::ensure_source_build_ready;
pub(crate) use macos::{
    inspect_macos_toolchain, macos_source_build_env, try_install_or_update_macos_clt,
};
