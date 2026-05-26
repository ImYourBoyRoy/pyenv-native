// ./crates/pyenv-core/src/pip/tests.rs
//! Unit tests verifying Pip package manager serializations and model configurations.

use super::types::{DependencyConflict, OutdatedPackage, PipPackage, PrecheckResult};

#[test]
fn test_pip_package_serialization() {
    let pkg = PipPackage {
        name: "requests".to_string(),
        version: "2.31.0".to_string(),
    };
    let json = serde_json::to_string(&pkg).unwrap();
    assert!(json.contains("\"name\":\"requests\""));
    assert!(json.contains("\"version\":\"2.31.0\""));
}

#[test]
fn test_outdated_package_serialization() {
    let pkg = OutdatedPackage {
        name: "urllib3".to_string(),
        version: "1.26.15".to_string(),
        latest_version: "2.2.1".to_string(),
    };
    let json = serde_json::to_string(&pkg).unwrap();
    assert!(json.contains("\"latest_version\":\"2.2.1\""));
}

#[test]
fn test_precheck_result_serialization() {
    let result = PrecheckResult {
        is_safe: false,
        resolved_packages: vec![PipPackage {
            name: "requests".to_string(),
            version: "2.31.0".to_string(),
        }],
        potential_conflicts: vec![DependencyConflict {
            package: "urllib3".to_string(),
            requirement: "urllib3<2".to_string(),
            installed: "2.2.1".to_string(),
            message: "Installed version 2.2.1 violates requirement urllib3<2".to_string(),
        }],
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"is_safe\":false"));
    assert!(json.contains("\"package\":\"urllib3\""));
}

#[test]
fn test_github_url_translation() {
    let url = "https://github.com/imyourboyroy/pyenv-native/blob/main/requirements.txt";
    let raw = url
        .replace("github.com", "raw.githubusercontent.com")
        .replace("/blob/", "/");
    assert_eq!(
        raw,
        "https://raw.githubusercontent.com/imyourboyroy/pyenv-native/main/requirements.txt"
    );
}
