// ./crates/pyenv-cli/tests/agent_skills.rs
//! End-to-end checks of `pyenv agent-skills` against a throwaway home and project directory.
//!
//! Only agents that write files or print text are exercised, so the real `gemini` and `agy`
//! tools are never started.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn run(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pyenv"))
        .args(args)
        .current_dir(cwd)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("PYENV_ROOT", home.join(".pyenv"))
        .env_remove("PYENV_VERSION")
        .output()
        .expect("run pyenv")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn installs_the_built_in_skill_for_user_scope_without_a_checkout() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    let cwd = temp.path().join("elsewhere");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&cwd).unwrap();

    for agent in ["cursor", "kiro"] {
        let output = run(&home, &cwd, &["agent-skills", "--agent", agent]);
        assert!(output.status.success(), "{agent}: {}", text(&output.stderr));
    }

    for dir in [".cursor", ".kiro"] {
        let skill = fs::read_to_string(home.join(dir).join("skills/pyenv-native/SKILL.md"))
            .unwrap_or_else(|error| panic!("{dir}: {error}"));
        assert!(
            skill.starts_with("---\nname: pyenv-native"),
            "{dir}: {skill}"
        );
    }
    // Nothing is written next to the working directory for user scope.
    assert_eq!(fs::read_dir(&cwd).unwrap().count(), 0);
}

#[test]
fn project_scope_writes_into_the_current_directory() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    let cwd = temp.path().join("project");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&cwd).unwrap();

    let output = run(
        &home,
        &cwd,
        &["agent-skills", "--agent", "copilot", "--scope", "project"],
    );
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(cwd.join(".github/skills/pyenv-native/SKILL.md").is_file());
}

#[test]
fn claude_prints_the_marketplace_commands_for_a_fork_url() {
    let temp = TempDir::new().unwrap();
    let output = run(
        temp.path(),
        temp.path(),
        &[
            "agent-skills",
            "--agent",
            "claude",
            "--repo-url",
            "https://github.com/someone/fork",
        ],
    );
    let stdout = text(&output.stdout);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(
        stdout.contains("/plugin marketplace add https://github.com/someone/fork"),
        "{stdout}"
    );
    assert!(
        stdout.contains("/plugin install pyenv-native@pyenv-native-skills"),
        "{stdout}"
    );
}

#[test]
fn an_unknown_agent_is_rejected_before_anything_is_written() {
    let temp = TempDir::new().unwrap();
    let output = run(
        temp.path(),
        temp.path(),
        &["agent-skills", "--agent", "emacs"],
    );
    assert!(!output.status.success());
    assert!(text(&output.stderr).contains("possible values"));
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
}
