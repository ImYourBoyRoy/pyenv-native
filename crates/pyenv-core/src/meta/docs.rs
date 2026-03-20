// ./crates/pyenv-core/src/meta/docs.rs
//! Shared command-doc structures and lookup helpers for built-in and compatibility-facing
//! command help.

use super::compat_docs::COMPATIBILITY_COMMAND_DOCS;
use super::public_docs::PUBLIC_COMMAND_DOCS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CommandDoc {
    pub(super) name: &'static str,
    pub(super) summary: &'static str,
    pub(super) usage: &'static str,
    pub(super) help: &'static [&'static str],
    pub(super) completions: &'static [&'static str],
}

pub(super) fn command_doc(name: &str) -> Option<&'static CommandDoc> {
    PUBLIC_COMMAND_DOCS
        .iter()
        .find(|doc| doc.name.eq_ignore_ascii_case(name))
        .or_else(|| {
            COMPATIBILITY_COMMAND_DOCS
                .iter()
                .find(|doc| doc.name.eq_ignore_ascii_case(name))
        })
}
