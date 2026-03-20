// ./crates/pyenv-core/src/meta/public_docs/mod.rs
//! Built-in public command documentation grouped by command families.

mod core;
mod environment;
mod system;

use std::sync::LazyLock;

use super::docs::CommandDoc;

pub(super) static PUBLIC_COMMAND_DOCS: LazyLock<Vec<CommandDoc>> = LazyLock::new(|| {
    core::CORE_COMMAND_DOCS
        .iter()
        .chain(environment::ENVIRONMENT_COMMAND_DOCS.iter())
        .chain(system::SYSTEM_COMMAND_DOCS.iter())
        .copied()
        .collect()
});
