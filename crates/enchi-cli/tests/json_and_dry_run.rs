// ./crates/enchi-cli/tests/json_and_dry_run.rs
//! `--json` and `--dry-run` across the command line: the uniform `enchi.result/1` envelope, typed
//! data where a command has it, refusal to prompt in machine mode, and dry runs that change
//! nothing. Runs the real binary against a throwaway root with fake runtimes.

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

fn root_with_runtimes() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    for version in ["3.12.0", "3.13.1"] {
        let bin = dir.path().join("versions").join(version).join("bin");
        fs::create_dir_all(&bin).expect("create runtime");
        fs::write(bin.join("python"), "#!/bin/sh\necho Python\n").expect("fake python");
    }
    fs::write(dir.path().join("version"), "3.13.1\n").expect("global version");
    dir
}

fn enchi(root: &Path, args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_enchi"))
        .args(args)
        .env("ENCHI_ROOT", root)
        .env_remove("PYENV_ROOT")
        .env_remove("PYENV_VERSION")
        .output()
        .expect("run enchi");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn json(root: &Path, args: &[&str]) -> (i32, Value) {
    let (code, stdout, stderr) = enchi(root, args);
    let value = serde_json::from_str(&stdout).unwrap_or_else(|error| {
        panic!("`enchi {args:?}` printed invalid JSON ({error}): {stdout}\n{stderr}")
    });
    (code, value)
}

#[test]
fn versions_json_lists_runtimes_with_the_active_one_marked() {
    let root = root_with_runtimes();
    let (code, value) = json(root.path(), &["versions", "--json"]);
    assert_eq!(code, 0);
    let items = value.as_array().expect("array");
    let active: Vec<&str> = items
        .iter()
        .filter(|item| item["active"] == true)
        .filter_map(|item| item["name"].as_str())
        .collect();
    assert_eq!(active, ["3.13.1"]);
    assert!(items.iter().any(|item| item["name"] == "3.12.0"));
}

#[test]
fn uninstall_dry_run_previews_and_removes_nothing() {
    let root = root_with_runtimes();
    let (code, stdout, _) = enchi(root.path(), &["uninstall", "3.12.0", "--dry-run"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Would uninstall 3.12.0"), "{stdout}");
    assert!(root.path().join("versions/3.12.0").exists());

    let (code, value) = json(root.path(), &["uninstall", "3.12.0", "--dry-run", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(value["schema"], "enchi.result/1");
    assert_eq!(value["dry_run"], true);
    assert_eq!(value["ok"], true);
    assert!(root.path().join("versions/3.12.0").exists());
}

#[test]
fn machine_mode_refuses_to_prompt_and_changes_nothing() {
    let root = root_with_runtimes();
    let (code, value) = json(root.path(), &["uninstall", "3.12.0", "--json"]);
    assert_eq!(code, 1);
    assert_eq!(value["ok"], false);
    let errors = value["errors"].to_string();
    assert!(
        errors.contains("never prompts") && errors.contains("--force"),
        "{errors}"
    );
    assert!(root.path().join("versions/3.12.0").exists());
}

#[test]
fn uninstall_json_with_force_removes_and_reports() {
    let root = root_with_runtimes();
    let (code, value) = json(root.path(), &["uninstall", "3.12.0", "--force", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(value["ok"], true);
    assert_eq!(value["dry_run"], false);
    assert!(!root.path().join("versions/3.12.0").exists());
    assert!(root.path().join("versions/3.13.1").exists());
}

#[test]
fn failures_keep_their_exit_code_inside_the_envelope() {
    let root = root_with_runtimes();
    let (code, value) = json(root.path(), &["uninstall", "9.9", "--dry-run", "--json"]);
    assert_eq!(code, 1);
    assert_eq!(value["exit_code"], 1);
    assert_eq!(value["ok"], false);
    assert!(
        value["errors"][0]
            .as_str()
            .unwrap()
            .contains("not installed")
    );
}

#[test]
fn simple_commands_carry_typed_data() {
    let root = root_with_runtimes();

    let (_, value) = json(root.path(), &["root", "--json"]);
    assert_eq!(
        Path::new(value["data"]["path"].as_str().unwrap()),
        root.path()
    );

    let (_, value) = json(root.path(), &["global", "--json"]);
    assert_eq!(value["data"]["versions"][0], "3.13.1");

    let (_, value) = json(root.path(), &["which", "python", "--json"]);
    assert!(
        value["data"]["path"]
            .as_str()
            .unwrap()
            .ends_with("3.13.1/bin/python"),
        "{value}"
    );
}

#[test]
fn venv_dry_runs_validate_without_creating_anything() {
    let root = root_with_runtimes();

    let (code, stdout, _) = enchi(
        root.path(),
        &["venv", "create", "3.13", "demo", "--dry-run"],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.contains("Would create managed venv 3.13.1/envs/demo"),
        "{stdout}"
    );
    assert!(!root.path().join("venvs").exists());

    let (code, value) = json(
        root.path(),
        &["venv", "create", "3.13", "bad/name", "--dry-run", "--json"],
    );
    assert_eq!(code, 1);
    assert!(
        value["errors"][0]
            .as_str()
            .unwrap()
            .contains("invalid managed venv name")
    );

    let (code, value) = json(
        root.path(),
        &["venv", "delete", "nope", "--dry-run", "--json"],
    );
    assert_eq!(code, 1);
    assert_eq!(value["command"], "venv delete");
}
