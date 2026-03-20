// ./crates/pyenv-core/src/install/providers/mod.rs
//! Provider catalogs, version normalization, and python-build backend discovery.

mod catalog;
mod python_build;
mod versioning;

pub(crate) use catalog::{cmd_provider_install_list, cpython_source_provider_versions};
#[cfg(test)]
pub(crate) use catalog::{cpython_source_provider_entries, provider_catalog_entries_for_platform};
pub(crate) use python_build::{load_python_build_definitions, resolve_python_build_path};
pub(crate) use versioning::{
    ensure_supported_cpython_version, is_free_threaded, is_pypy_request,
    normalize_requested_version, nuget_package_name, resolve_provider_version,
};
