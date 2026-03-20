// ./crates/pyenv-core/src/version/mod.rs
//! Version file discovery, selection, and related command implementations. This package keeps
//! the public version API stable while splitting file parsing, selection, and command logic into
//! focused modules.

mod commands;
mod files;
mod selection;
#[cfg(test)]
mod tests;
mod types;

pub use self::commands::{
    cmd_global, cmd_local, cmd_root, cmd_version, cmd_version_file, cmd_version_file_read,
    cmd_version_file_write, cmd_version_name, cmd_version_origin,
};
pub use self::files::{find_local_version_file, read_version_file, version_file_path};
pub use self::selection::{installed_version_dir, resolve_selected_versions, version_origin};
pub use self::types::{SelectedVersions, VersionOrigin};
