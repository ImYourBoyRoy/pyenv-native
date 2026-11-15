// ./crates/enchi-cli/tests/help_coverage.rs
//! Every command listed in `enchi --help` must also be documented by `enchi help <command>` and
//! appear in `enchi commands`, so a new command cannot ship without its help text.

use std::process::Command;

fn enchi(root: &std::path::Path, args: &[&str]) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_enchi"))
        .args(args)
        .env("ENCHI_ROOT", root)
        .env_remove("PYENV_ROOT")
        .output()
        .expect("run enchi");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), text)
}

/// Command names printed in the grouped sections of the top-level help.
fn listed_commands(root: &std::path::Path) -> Vec<String> {
    let (_, help) = enchi(root, &["--help"]);
    help.lines()
        .take_while(|line| !line.trim_start().starts_with("-h, --help"))
        .filter_map(|line| {
            let rest = line.strip_prefix("  ")?;
            if rest.starts_with(' ') || rest.starts_with('-') {
                return None;
            }
            let name = rest.split_whitespace().next()?;
            (rest.len() > name.len()).then(|| name.to_string())
        })
        .collect()
}

#[test]
fn every_listed_command_has_help_text_and_is_inventoried() {
    let dir = tempfile::tempdir().expect("temp dir");
    let names = listed_commands(dir.path());
    assert!(
        names.len() > 30,
        "expected the grouped command list, got {names:?}"
    );
    assert!(
        names.iter().any(|name| name == "upgrade"),
        "`upgrade` is missing from `enchi --help`: {names:?}"
    );

    let (_, inventory) = enchi(dir.path(), &["commands"]);
    let inventory: Vec<&str> = inventory.lines().map(str::trim).collect();

    for name in &names {
        let (ok, text) = enchi(dir.path(), &["help", name]);
        assert!(
            ok && !text.contains("no such command") && !text.contains("does not expose"),
            "`enchi help {name}` has no help text: {text}"
        );
        assert!(
            inventory.contains(&name.as_str()),
            "`enchi commands` does not list `{name}`"
        );
    }
}
