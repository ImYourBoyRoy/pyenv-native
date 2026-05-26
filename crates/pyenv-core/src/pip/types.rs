// ./crates/pyenv-core/src/pip/types.rs
//! Pip Package Manager data structures and serialization types.
//!
//! Exposes structure definitions for installed packages, outdated packages,
//! environment-wide requirement conflicts, and precheck validation reports.

use serde::{Deserialize, Serialize};

/// Represents an installed Pip package in a target environment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipPackage {
    pub name: String,
    pub version: String,
}

/// Represents an outdated Pip package with a pending upgrade.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutdatedPackage {
    pub name: String,
    pub version: String,
    pub latest_version: String,
}

/// Represents a dependency requirement conflict in a target environment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyConflict {
    pub package: String,
    pub requirement: String,
    pub installed: String,
    pub message: String,
}

/// Represents the report of a requirements.txt import verification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrecheckResult {
    pub is_safe: bool,
    pub resolved_packages: Vec<PipPackage>,
    pub potential_conflicts: Vec<DependencyConflict>,
}
