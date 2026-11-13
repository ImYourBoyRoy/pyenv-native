// ./crates/pyenv-cli/tests/help.rs
//! `pyenv help <command>` must work for every command the top-level help lists.

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn pyenv(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pyenv"))
        .args(args)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("PYENV_ROOT", home.join(".pyenv"))
        // An empty PATH keeps `pyenv-gui`, `pyenv-mcp` and any other `pyenv-*` program on the
        // developer's machine from being picked up as plugin commands that have help.
        .env("PATH", home)
        .env_remove("PYENV_VERSION")
        .env_remove("PYENV_LANG")
        .output()
        .expect("run pyenv")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Command names from the rows of `pyenv --help` (`  name   description`).
fn listed_commands(home: &Path) -> Vec<String> {
    let help = pyenv(home, &["--help"]);
    assert!(help.status.success(), "{}", stderr(&help));
    stdout(&help)
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("  ")?;
            let name = rest.split_whitespace().next()?;
            let described = rest.len() > name.len() && rest[name.len()..].starts_with(' ');
            (described
                && name.chars().next()?.is_ascii_lowercase()
                && name.chars().all(|c| c.is_ascii_lowercase() || c == '-'))
            .then(|| name.to_string())
        })
        .collect()
}

#[test]
fn help_shows_the_usage_and_description_of_a_command() {
    let home = TempDir::new().unwrap();
    let output = pyenv(home.path(), &["help", "install"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).starts_with("Usage: pyenv install"),
        "{}",
        stdout(&output)
    );

    let usage = pyenv(home.path(), &["help", "--usage", "install"]);
    assert!(usage.status.success(), "{}", stderr(&usage));
    assert_eq!(stdout(&usage).lines().count(), 1);
}

#[test]
fn bare_help_lists_the_commands() {
    let home = TempDir::new().unwrap();
    let output = pyenv(home.path(), &["help"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).starts_with("Usage: pyenv <command>"));
}

#[test]
fn help_for_an_unknown_command_fails_by_naming_it() {
    let home = TempDir::new().unwrap();
    let output = pyenv(home.path(), &["help", "no-such-command"]);
    assert!(!output.status.success());
    let message = format!("{}{}", stdout(&output), stderr(&output));
    assert!(
        message.contains("no such command `no-such-command`"),
        "{message}"
    );
}

#[test]
fn the_older_manual_help_spelling_still_works() {
    let home = TempDir::new().unwrap();
    let output = pyenv(home.path(), &["manual-help", "install"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).starts_with("Usage: pyenv install"));
}

#[test]
fn every_command_listed_by_the_top_level_help_has_help_of_its_own() {
    let home = TempDir::new().unwrap();
    let commands = listed_commands(home.path());
    assert!(
        commands.len() > 30,
        "the command list looks too short to be real: {commands:?}"
    );
    let mut problems = Vec::new();
    for command in &commands {
        let output = pyenv(home.path(), &["help", command]);
        if !output.status.success() {
            problems.push(format!(
                "`pyenv help {command}` failed: {}{}",
                stdout(&output),
                stderr(&output)
            ));
        } else if !stdout(&output).starts_with(&format!("Usage: pyenv {command}")) {
            problems.push(format!(
                "`pyenv help {command}` printed: {}",
                stdout(&output)
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    // The commands that used to be missing are among them.
    for command in [
        "self-install",
        "self-uninstall",
        "status",
        "prompt",
        "agent-skills",
    ] {
        assert!(commands.iter().any(|c| c == command), "{command}");
    }
}
