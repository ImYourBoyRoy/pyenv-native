// ./crates/pyenv-core/src/shell/emit/mod.rs
//! Shell-code emitters for init wrappers, shell-scoped version changes, and managed-venv
//! activation/deactivation helpers.

mod activation;
mod environment;
mod functions;
mod quotes;

pub(crate) use activation::{shell_emit_activate, shell_emit_deactivate};
pub(crate) use environment::{
    shell_emit_rehash, shell_emit_revert, shell_emit_set, shell_emit_set_shell,
    shell_emit_show_current, shell_emit_unset,
};
pub(crate) use functions::{render_cmd_exec_line, render_shell_function};
pub(crate) use quotes::ps_single_quote;
