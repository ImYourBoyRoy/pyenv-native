// ./crates/pyenv-core/src/installer/mod.rs
//! Native install, update, and uninstall for portable pyenv-native roots.
//!
//! One implementation serves every entry point: the thin `install.sh`/`install.ps1` bootstraps
//! download and verify a release bundle and then run `pyenv self-install`; `pyenv self-update`
//! does the same from inside a running install; the GUI and Python bootstrap package call the
//! same code. Shell-profile blocks, user PATH edits, binary replacement, and desktop integration
//! all live here instead of being re-implemented per script.

mod arp;
mod binaries;
mod commands;
#[cfg(target_os = "linux")]
mod desktop;
mod install;
mod layout;
mod profile;
mod report;
mod source;
mod uninstall;
mod user_path;

pub use commands::{cmd_self_install, cmd_self_uninstall, cmd_self_uninstall_with};
pub use install::{PowerShell7Mode, SelfInstallOptions, SelfInstallOutcome, run_self_install};
pub use layout::{GUI_BINARY, InstallLayout, MCP_BINARY, PYENV_BINARY, executable_file_name};
pub use profile::{
    BEGIN_MARKER, END_MARKER, ProfileUpdate, ShellKind, apply_profile_block, has_pyenv_init,
    home_dir, known_profile_paths, powershell_profile_path, profile_path, remove_profile_block,
};
pub use report::LineSink;
pub use source::{package_source_for, upgrade_command};
pub use uninstall::{SelfUninstallOptions, run_self_uninstall};
