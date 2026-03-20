// ./crates/pyenv-core/src/catalog/types.rs
//! Shared catalog entry/group models and install-list options.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CatalogSourceKind {
    Installed,
    Known,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CatalogEntry {
    pub name: String,
    pub family: String,
    pub family_slug: String,
    pub source: CatalogSourceKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CatalogGroup {
    pub family: String,
    pub family_slug: String,
    pub source: CatalogSourceKind,
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InstallListOptions {
    pub family: Option<String>,
    pub json: bool,
    pub pattern: Option<String>,
}
