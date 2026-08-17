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

#[test]
fn helper_exits_nonzero_on_error_payload() {
    use std::fs;
    use std::process::Command;

    let python = ["python3", "python"].into_iter().find(|bin| {
        Command::new(bin)
            .arg("-c")
            .arg("import sys; sys.exit(0)")
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    });
    let Some(python) = python else {
        eprintln!("skipping helper exit test: no python interpreter on PATH");
        return;
    };

    let temp = tempfile::NamedTempFile::new().expect("helper temp");
    fs::write(temp.path(), include_str!("helper.py")).expect("write helper");
    let output = Command::new(python)
        .arg(temp.path())
        .output()
        .expect("run helper");
    assert!(
        !output.status.success(),
        "helper must fail closed when invoked without arguments"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"error\""), "stdout={stdout}");
}

fn python_bin() -> Option<&'static str> {
    use std::process::Command;

    ["python3", "python"].into_iter().find(|bin| {
        Command::new(bin)
            .arg("-c")
            .arg("import sys; sys.exit(0)")
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    })
}

#[test]
fn helper_rejects_pip_option_and_unparsed_lines() {
    use std::fs;
    use std::process::Command;

    let Some(python) = python_bin() else {
        eprintln!("skipping helper option-line test: no python interpreter on PATH");
        return;
    };

    let dir = tempfile::TempDir::new().expect("tempdir");
    let helper = dir.path().join("helper.py");
    let reqs = dir.path().join("requirements.txt");
    fs::write(&helper, include_str!("helper.py")).expect("write helper");
    fs::write(&reqs, "requests>=2.0\n-r other.txt\n").expect("write reqs");

    let output = Command::new(python)
        .arg(&helper)
        .arg(&reqs)
        .output()
        .expect("run helper");
    assert!(
        !output.status.success(),
        "precheck must fail closed on pip option lines, stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"error\""), "stdout={stdout}");
    assert!(stdout.contains("-r other.txt"), "stdout={stdout}");
}

#[test]
fn helper_compare_versions_fails_closed_on_unparseable() {
    use std::fs;
    use std::process::Command;

    let Some(python) = python_bin() else {
        eprintln!("skipping helper compare_versions test: no python interpreter on PATH");
        return;
    };

    let dir = tempfile::TempDir::new().expect("tempdir");
    let helper = dir.path().join("helper.py");
    fs::write(&helper, include_str!("helper.py")).expect("write helper");
    let script = format!(
        r#"
import importlib.util
spec = importlib.util.spec_from_file_location("helper", r"{path}")
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)
assert mod.compare_versions("1.0.0", "==", "1.0.0") is True
assert mod.compare_versions("2.0", ">=", "1.0") is True
assert mod.compare_versions("not-a-version", ">=", "1.0") is False
assert mod.compare_versions("1.0", "??", "2.0") is False
print("ok")
"#,
        path = helper.display().to_string().replace('\\', "\\\\")
    );

    let output = Command::new(python)
        .arg("-c")
        .arg(script)
        .output()
        .expect("run compare_versions assertions");
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
