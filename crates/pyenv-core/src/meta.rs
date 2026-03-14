// ./crates/pyenv-core/src/meta.rs
//! Command-surface helpers for parity-focused commands like help, commands, shims, and completions.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::catalog::{VersionFamily, installed_version_names, known_version_names};
use crate::command::CommandReport;
use crate::context::AppContext;
use crate::plugin::{
    DEFAULT_HOOK_COMMANDS, complete_plugin_command, discover_plugin_commands, find_plugin_command,
};
use crate::runtime::normalize_shim_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CommandDoc {
    name: &'static str,
    summary: &'static str,
    usage: &'static str,
    help: &'static [&'static str],
    completions: &'static [&'static str],
}

const PUBLIC_COMMAND_DOCS: &[CommandDoc] = &[
    CommandDoc {
        name: "help",
        summary: "Display help for a command",
        usage: "Usage: pyenv help [--usage] [COMMAND]",
        help: &[
            "Shows a short summary for all primary commands, or detailed help for one command.",
            "Use `pyenv help --usage <command>` to print only the usage line.",
        ],
        completions: &["--usage"],
    },
    CommandDoc {
        name: "commands",
        summary: "List all available pyenv commands",
        usage: "Usage: pyenv commands [--sh|--no-sh]",
        help: &[
            "Lists the commands available from the native core and discovered plugins.",
            "Use `--sh` to show shell-helper commands, or `--no-sh` to suppress them.",
        ],
        completions: &["--sh", "--no-sh"],
    },
    CommandDoc {
        name: "local",
        summary: "Set or show the local application-specific Python version",
        usage: "Usage: pyenv local [-f|--force] [--unset] [version ...]",
        help: &[
            "Writes a `.python-version` file in the current directory.",
            "When no version is provided, prints the currently configured local version.",
        ],
        completions: &["-f", "--force", "--unset"],
    },
    CommandDoc {
        name: "global",
        summary: "Set or show the global Python version",
        usage: "Usage: pyenv global [--unset] [version ...]",
        help: &[
            "Writes the global version file under `PYENV_ROOT`.",
            "When no version is provided, prints the currently configured global version.",
        ],
        completions: &["--unset"],
    },
    CommandDoc {
        name: "shell",
        summary: "Set or show the shell-specific Python version",
        usage: "Usage: pyenv shell [--unset|-] [version ...]",
        help: &[
            "Requires shell integration from `pyenv init`.",
            "Use `--unset` to clear the shell-specific selection or `-` to revert to the previous value.",
        ],
        completions: &["--unset", "-"],
    },
    CommandDoc {
        name: "install",
        summary: "Install a Python version",
        usage: "Usage: pyenv install [-l|--list] [--known] [--family <family>] [--dry-run] [--json] [-f|--force] <version> ...",
        help: &[
            "Uses native provider backends where available and compatibility fallbacks where needed.",
            "Prefix requests like `3.12` resolve to the latest matching provider-backed version.",
        ],
        completions: &[
            "-l",
            "--list",
            "--known",
            "--family",
            "--dry-run",
            "--json",
            "-f",
            "--force",
        ],
    },
    CommandDoc {
        name: "uninstall",
        summary: "Uninstall Python versions",
        usage: "Usage: pyenv uninstall [-f|--force] <version> ...",
        help: &[
            "Removes managed runtimes from the versions directory and refreshes shims afterwards.",
        ],
        completions: &["-f", "--force"],
    },
    CommandDoc {
        name: "rehash",
        summary: "Rebuild pyenv shims",
        usage: "Usage: pyenv rehash",
        help: &[
            "Scans managed runtimes and regenerates shim launchers for discovered executables.",
        ],
        completions: &[],
    },
    CommandDoc {
        name: "version",
        summary: "Show the current Python version",
        usage: "Usage: pyenv version [--bare]",
        help: &[
            "Displays the selected versions and how they were chosen.",
            "`--bare` prints just the selected version names.",
        ],
        completions: &["--bare"],
    },
    CommandDoc {
        name: "versions",
        summary: "List all known installed Python versions",
        usage: "Usage: pyenv versions [--bare] [--skip-aliases] [--skip-envs] [--executables]",
        help: &[
            "Marks the currently selected version and can optionally list executable shim names.",
        ],
        completions: &["--bare", "--skip-aliases", "--skip-envs", "--executables"],
    },
    CommandDoc {
        name: "which",
        summary: "Display the full path to an executable",
        usage: "Usage: pyenv which [--nosystem] [--skip-advice] <command>",
        help: &[
            "Searches the selected pyenv runtimes first, then the system PATH unless `--nosystem` is used.",
        ],
        completions: &["--nosystem", "--skip-advice"],
    },
    CommandDoc {
        name: "whence",
        summary: "List Python versions that contain the given executable",
        usage: "Usage: pyenv whence [--path] <command>",
        help: &["Prints matching managed versions or executable paths when `--path` is used."],
        completions: &["--path"],
    },
    CommandDoc {
        name: "exec",
        summary: "Run a command with the selected Python version",
        usage: "Usage: pyenv exec <command> [arg1 arg2...]",
        help: &[
            "Prepares PATH for the active runtime selection and then executes the requested program.",
        ],
        completions: &[],
    },
    CommandDoc {
        name: "root",
        summary: "Display the current pyenv root",
        usage: "Usage: pyenv root",
        help: &["Prints the root directory where managed runtimes, shims, and config are stored."],
        completions: &[],
    },
    CommandDoc {
        name: "prefix",
        summary: "Display the installation prefix for one or more versions",
        usage: "Usage: pyenv prefix [version ...]",
        help: &["When no version is specified, prints the prefixes for the current selection."],
        completions: &[],
    },
    CommandDoc {
        name: "latest",
        summary: "Resolve the latest installed or known matching version",
        usage: "Usage: pyenv latest [-k|--known] [-b|--bypass] [-f|--force] <prefix>",
        help: &[
            "Resolves version prefixes such as `3.12` or `pypy3.11` to the latest matching version.",
        ],
        completions: &["-k", "--known", "-b", "--bypass", "-f", "--force"],
    },
    CommandDoc {
        name: "hooks",
        summary: "List installed hook scripts for a given command",
        usage: "Usage: pyenv hooks <command>",
        help: &["Prints the resolved hook scripts for the requested hook name in execution order."],
        completions: &[],
    },
    CommandDoc {
        name: "shims",
        summary: "List existing pyenv shims",
        usage: "Usage: pyenv shims [--short]",
        help: &[
            "Lists the generated shim launchers under `PYENV_ROOT/shims`.",
            "Use `--short` to print command names only.",
        ],
        completions: &["--short"],
    },
    CommandDoc {
        name: "init",
        summary: "Configure the shell environment for pyenv",
        usage: "Usage: pyenv init [-|--path] [--detect-shell] [--no-push-path] [--no-rehash] [<shell>]",
        help: &["Prints shell initialization code for PowerShell, CMD, and POSIX-style shells."],
        completions: &[
            "-",
            "--path",
            "--detect-shell",
            "--no-push-path",
            "--no-rehash",
            "pwsh",
            "cmd",
            "bash",
            "zsh",
            "fish",
            "sh",
        ],
    },
    CommandDoc {
        name: "completions",
        summary: "List completions for a pyenv command",
        usage: "Usage: pyenv completions <command> [arg1 arg2...]",
        help: &[
            "Prints candidate completions for built-in commands and plugin commands that support `--complete`.",
        ],
        completions: &[],
    },
    CommandDoc {
        name: "doctor",
        summary: "Run diagnostics against the current pyenv environment",
        usage: "Usage: pyenv doctor [--json]",
        help: &["Checks root, shims, PATH, and system-Python visibility."],
        completions: &["--json"],
    },
    CommandDoc {
        name: "config",
        summary: "Inspect or change pyenv configuration",
        usage: "Usage: pyenv config path|show|get|set ...",
        help: &["Reads or updates the persisted `config.toml` under the active pyenv root."],
        completions: &["path", "show", "get", "set"],
    },
];

const SHELL_HELPER_COMMANDS: &[&str] = &["cmd", "rehash", "shell"];
pub fn cmd_commands(ctx: &AppContext, shell_only: bool, no_shell: bool) -> CommandReport {
    if shell_only && no_shell {
        return CommandReport::failure(
            vec!["pyenv: choose either `--sh` or `--no-sh`, not both".to_string()],
            1,
        );
    }

    let mut commands = BTreeSet::new();
    if shell_only {
        for name in SHELL_HELPER_COMMANDS {
            commands.insert((*name).to_string());
        }
    } else {
        for doc in PUBLIC_COMMAND_DOCS {
            commands.insert(doc.name.to_string());
        }
    }

    for command in discover_plugin_commands(ctx) {
        let is_shell = command.starts_with("sh-");
        if shell_only && !is_shell {
            continue;
        }
        if no_shell && is_shell {
            continue;
        }

        let visible = command.strip_prefix("sh-").unwrap_or(&command).to_string();
        if !visible.is_empty() {
            commands.insert(visible);
        }
    }

    CommandReport::success(commands.into_iter().collect())
}

pub fn cmd_help(ctx: &AppContext, command: Option<&str>, usage_only: bool) -> CommandReport {
    let target = command
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "pyenv");

    let Some(command_name) = target else {
        if usage_only {
            return CommandReport::success_one("Usage: pyenv <command> [<args>]");
        }

        let mut stdout = vec![
            "Usage: pyenv <command> [<args>]".to_string(),
            String::new(),
            "Some useful pyenv commands are:".to_string(),
        ];
        stdout.extend(
            PUBLIC_COMMAND_DOCS
                .iter()
                .map(|doc| format!("   {:<12} {}", doc.name, doc.summary)),
        );
        stdout.push(String::new());
        stdout
            .push("See `pyenv help <command>` for information on a specific command.".to_string());
        stdout
            .push("For full documentation, see: https://github.com/pyenv/pyenv#readme".to_string());
        return CommandReport::success(stdout);
    };

    if let Some(doc) = command_doc(command_name) {
        if usage_only {
            return CommandReport::success_one(doc.usage);
        }

        let mut stdout = vec![doc.usage.to_string()];
        if !doc.help.is_empty() {
            stdout.push(String::new());
            stdout.extend(doc.help.iter().map(|line| (*line).to_string()));
        }
        return CommandReport::success(stdout);
    }

    if let Some(command_path) = find_plugin_command(ctx, command_name) {
        let external_doc = parse_external_command_doc(&command_path, command_name);
        let usage = external_doc
            .as_ref()
            .map(|doc| doc.usage.clone())
            .unwrap_or_else(|| format!("Usage: pyenv {command_name} [<args>]"));
        if usage_only {
            return CommandReport::success_one(usage);
        }

        if let Some(doc) = external_doc {
            let mut stdout = vec![doc.usage];
            if !doc.help.is_empty() {
                stdout.push(String::new());
                stdout.extend(doc.help);
            }
            return CommandReport::success(stdout);
        }

        return CommandReport::success(vec![
            usage,
            String::new(),
            format!(
                "`{command_name}` is provided by an external pyenv plugin and does not expose built-in help text."
            ),
        ]);
    }

    CommandReport::failure(vec![format!("pyenv: no such command `{command_name}`")], 1)
}

pub fn cmd_shims(ctx: &AppContext, short: bool) -> CommandReport {
    let entries = match list_shim_entries(ctx) {
        Ok(entries) => entries,
        Err(error) => return CommandReport::failure(vec![error], 1),
    };

    if short {
        return CommandReport::success(entries.into_keys().collect());
    }

    CommandReport::success(
        entries
            .into_values()
            .map(|path| path.display().to_string())
            .collect(),
    )
}

pub fn cmd_completions(ctx: &AppContext, command: &str, args: &[String]) -> CommandReport {
    let requested = command.trim();
    if requested.is_empty() {
        return CommandReport::failure(
            vec!["Usage: pyenv completions <command> [arg1 arg2...]".to_string()],
            1,
        );
    }

    if requested == "--complete" {
        return cmd_commands(ctx, false, false);
    }

    let mut values = BTreeSet::new();
    values.insert("--help".to_string());

    if let Some(doc) = command_doc(requested) {
        values.extend(doc.completions.iter().map(|value| (*value).to_string()));
        values.extend(dynamic_builtin_completions(ctx, requested, args));
    } else if let Some(plugin_values) = match complete_plugin_command(ctx, requested, args) {
        Ok(values) => values,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    } {
        values.extend(plugin_values);
    } else {
        return CommandReport::failure(vec![format!("pyenv: no such command `{requested}`")], 1);
    }

    CommandReport::success(values.into_iter().collect())
}

fn command_doc(name: &str) -> Option<&'static CommandDoc> {
    PUBLIC_COMMAND_DOCS
        .iter()
        .find(|doc| doc.name.eq_ignore_ascii_case(name))
}

fn dynamic_builtin_completions(ctx: &AppContext, command: &str, args: &[String]) -> Vec<String> {
    match command {
        "help" | "commands" => PUBLIC_COMMAND_DOCS
            .iter()
            .map(|doc| doc.name.to_string())
            .chain(
                discover_plugin_commands(ctx)
                    .into_iter()
                    .map(|name| name.strip_prefix("sh-").unwrap_or(&name).to_string()),
            )
            .collect(),
        "global" | "local" | "shell" | "prefix" | "uninstall" => {
            let mut values = installed_version_names(ctx).unwrap_or_default();
            if matches!(command, "global" | "shell" | "prefix") {
                values.push("system".to_string());
            }
            values
        }
        "latest" => known_version_names().iter().cloned().collect(),
        "hooks" => DEFAULT_HOOK_COMMANDS
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        "install" => {
            if args
                .iter()
                .rev()
                .find(|value| !value.trim().is_empty())
                .is_some_and(|value| value == "--family")
            {
                return known_family_slugs();
            }

            let mut values = known_family_slugs();
            values.extend(known_version_names().iter().cloned());
            values
        }
        "which" | "whence" | "exec" => cmd_shims(ctx, true).stdout,
        _ => Vec::new(),
    }
}

fn known_family_slugs() -> Vec<String> {
    let mut families = BTreeSet::new();
    for version in known_version_names() {
        families.insert(VersionFamily::classify(version).slug());
    }
    families.into_iter().collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExternalCommandDoc {
    usage: String,
    help: Vec<String>,
}

fn parse_external_command_doc(path: &Path, command_name: &str) -> Option<ExternalCommandDoc> {
    let content = fs::read(path).ok()?;
    let text = String::from_utf8_lossy(&content);
    let header = extract_external_doc_header(&text);
    if header.is_empty() {
        return None;
    }

    let usage = parse_header_usage(&header)
        .unwrap_or_else(|| format!("Usage: pyenv {command_name} [<args>]"));
    let summary = header
        .iter()
        .find_map(|line| {
            line.strip_prefix("Summary:")
                .map(str::trim)
                .map(ToOwned::to_owned)
        })
        .filter(|value| !value.is_empty());
    let help = parse_header_help(&header, summary.as_deref());

    Some(ExternalCommandDoc { usage, help })
}

fn extract_external_doc_header(text: &str) -> Vec<String> {
    let mut header = Vec::new();
    let mut collecting = false;

    for line in text.lines() {
        if !collecting && line.starts_with("#!") {
            continue;
        }

        if let Some(comment) = strip_doc_comment_prefix(line) {
            collecting = true;
            header.push(comment);
            continue;
        }

        if collecting && line.trim().is_empty() {
            header.push(String::new());
            continue;
        }

        if collecting {
            break;
        }
    }

    while header.first().is_some_and(|line| line.trim().is_empty()) {
        header.remove(0);
    }
    while header.last().is_some_and(|line| line.trim().is_empty()) {
        header.pop();
    }

    header
}

fn strip_doc_comment_prefix(line: &str) -> Option<String> {
    let trimmed_start = line.trim_start();
    if trimmed_start.starts_with('#') && !trimmed_start.starts_with("#!") {
        return Some(
            trimmed_start[1..]
                .strip_prefix(' ')
                .unwrap_or(&trimmed_start[1..])
                .to_string(),
        );
    }
    if let Some(rest) = trimmed_start.strip_prefix("::") {
        return Some(rest.strip_prefix(' ').unwrap_or(rest).to_string());
    }
    if trimmed_start
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("rem"))
    {
        let rest = &trimmed_start[3..];
        return Some(rest.strip_prefix(' ').unwrap_or(rest).to_string());
    }
    None
}

fn parse_header_usage(header: &[String]) -> Option<String> {
    let start = header
        .iter()
        .position(|line| line.trim_start().starts_with("Usage:"))?;
    let mut lines = Vec::new();
    let first = header[start].trim_start();
    lines.push(first.to_string());

    for line in &header[start + 1..] {
        if line.starts_with("Summary:")
            || (!line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t'))
        {
            break;
        }
        lines.push(line.to_string());
    }

    Some(lines.join("\n"))
}

fn parse_header_help(header: &[String], summary: Option<&str>) -> Vec<String> {
    let Some(start) = header
        .iter()
        .position(|line| line.trim_start().starts_with("Summary:"))
    else {
        return summary
            .map(|value| vec![value.to_string()])
            .unwrap_or_default();
    };

    let mut help = header[start + 1..].to_vec();
    while help.first().is_some_and(|line| line.trim().is_empty()) {
        help.remove(0);
    }
    while help.last().is_some_and(|line| line.trim().is_empty()) {
        help.pop();
    }

    if help.iter().any(|line| !line.trim().is_empty()) {
        help
    } else {
        summary
            .map(|value| vec![value.to_string()])
            .unwrap_or_default()
    }
}

fn list_shim_entries(ctx: &AppContext) -> Result<BTreeMap<String, PathBuf>, String> {
    let shims_dir = ctx.shims_dir();
    if !shims_dir.is_dir() {
        return Ok(BTreeMap::new());
    }

    let mut entries = fs::read_dir(&shims_dir)
        .map_err(|error| format!("pyenv: {error}"))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter_map(|path| {
            let normalized = normalize_shim_name(&path, ctx.path_ext.as_deref())?;
            Some((normalized, path))
        })
        .collect::<Vec<_>>();

    entries.sort_by(|(lhs_name, lhs_path), (rhs_name, rhs_path)| {
        lhs_name
            .to_ascii_lowercase()
            .cmp(&rhs_name.to_ascii_lowercase())
            .then_with(|| {
                preferred_shim_rank(lhs_path)
                    .cmp(&preferred_shim_rank(rhs_path))
                    .then_with(|| {
                        lhs_path
                            .display()
                            .to_string()
                            .to_ascii_lowercase()
                            .cmp(&rhs_path.display().to_string().to_ascii_lowercase())
                    })
            })
    });

    let mut selected = BTreeMap::new();
    for (name, path) in entries {
        selected.entry(name).or_insert(path);
    }
    Ok(selected)
}

fn preferred_shim_rank(path: &Path) -> usize {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("exe") => 0,
        Some("cmd") => 1,
        Some("bat") => 2,
        Some("ps1") => 3,
        None => 0,
        _ => 10,
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::ffi::OsString;
    use std::fs;

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::{cmd_commands, cmd_completions, cmd_help, cmd_shims};

    fn test_context() -> (TempDir, AppContext) {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        let dir = temp.path().join("work");
        fs::create_dir_all(root.join("plugins")).expect("plugins");
        fs::create_dir_all(root.join("versions")).expect("versions");
        fs::create_dir_all(root.join("shims")).expect("shims");
        fs::create_dir_all(&dir).expect("work");

        let ctx = AppContext {
            root,
            dir,
            exe_path: std::path::PathBuf::from("pyenv"),
            env_version: None,
            env_shell: None,
            path_env: Some(OsString::from("C:\\Windows\\System32")),
            path_ext: Some(OsString::from(".EXE;.CMD;.BAT;.PS1")),
            config: AppConfig::default(),
        };

        (temp, ctx)
    }

    #[test]
    fn commands_lists_core_and_plugin_commands() {
        let (_temp, ctx) = test_context();
        let plugin_bin = ctx.root.join("plugins").join("demo").join("bin");
        fs::create_dir_all(&plugin_bin).expect("plugin bin");
        fs::write(plugin_bin.join("pyenv-hello.cmd"), "@echo off").expect("plugin");

        let report = cmd_commands(&ctx, false, false);
        assert_eq!(report.exit_code, 0);
        assert!(report.stdout.iter().any(|line| line == "help"));
        assert!(report.stdout.iter().any(|line| line == "hello"));
        assert!(report.stdout.iter().all(|line| !line.starts_with("sh-")));
    }

    #[test]
    fn commands_detect_path_plugins_in_directories_with_spaces() {
        let (_temp, mut ctx) = test_context();
        let path_dir = ctx.root.join("path plugins");
        fs::create_dir_all(&path_dir).expect("path dir");
        fs::write(path_dir.join("pyenv-sh-hello.cmd"), "@echo off").expect("plugin");
        let existing_path = ctx.path_env.clone().expect("path env");
        let mut joined = env::split_paths(&existing_path).collect::<Vec<_>>();
        joined.insert(0, path_dir);
        ctx.path_env = Some(env::join_paths(joined).expect("join path"));

        let report = cmd_commands(&ctx, true, false);
        assert_eq!(report.exit_code, 0);
        assert!(report.stdout.iter().any(|line| line == "hello"));
    }

    #[test]
    fn help_prints_usage_and_summary() {
        let (_temp, ctx) = test_context();
        let report = cmd_help(&ctx, Some("install"), false);
        assert_eq!(report.exit_code, 0);
        assert_eq!(
            report.stdout.first().expect("usage"),
            "Usage: pyenv install [-l|--list] [--known] [--family <family>] [--dry-run] [--json] [-f|--force] <version> ..."
        );

        let usage_report = cmd_help(&ctx, None, true);
        assert_eq!(usage_report.stdout, vec!["Usage: pyenv <command> [<args>]"]);
    }

    #[test]
    fn help_parses_external_plugin_doc_headers() {
        let (_temp, ctx) = test_context();
        let plugin_bin = ctx.root.join("plugins").join("demo").join("bin");
        fs::create_dir_all(&plugin_bin).expect("plugin bin");
        fs::write(
            plugin_bin.join("pyenv-hello"),
            "#!/usr/bin/env sh\n# Usage: pyenv hello <world>\n#        pyenv hi [everybody]\n# Summary: Says hello to you.\n# This is extended help.\n#\n# And paragraphs.\nexit 0\n",
        )
        .expect("plugin");

        let report = cmd_help(&ctx, Some("hello"), false);
        assert_eq!(report.exit_code, 0);
        assert_eq!(
            report.stdout,
            vec![
                "Usage: pyenv hello <world>\n       pyenv hi [everybody]".to_string(),
                String::new(),
                "This is extended help.".to_string(),
                String::new(),
                "And paragraphs.".to_string(),
            ]
        );
    }

    #[test]
    fn help_falls_back_to_plugin_summary_without_extended_text() {
        let (_temp, ctx) = test_context();
        let plugin_bin = ctx.root.join("plugins").join("demo").join("bin");
        fs::create_dir_all(&plugin_bin).expect("plugin bin");
        fs::write(
            plugin_bin.join("pyenv-hello"),
            "#!/usr/bin/env sh\n# Usage: pyenv hello <world>\n# Summary: Says hello to you.\nexit 0\n",
        )
        .expect("plugin");

        let report = cmd_help(&ctx, Some("hello"), false);
        assert_eq!(
            report.stdout,
            vec![
                "Usage: pyenv hello <world>".to_string(),
                String::new(),
                "Says hello to you.".to_string(),
            ]
        );

        let usage_only = cmd_help(&ctx, Some("hello"), true);
        assert_eq!(
            usage_only.stdout,
            vec!["Usage: pyenv hello <world>".to_string()]
        );
    }

    #[test]
    fn shims_prefers_primary_launcher_per_command() {
        let (_temp, ctx) = test_context();
        fs::write(ctx.shims_dir().join("python.exe"), "").expect("python exe");
        fs::write(ctx.shims_dir().join("python.cmd"), "").expect("python cmd");
        fs::write(ctx.shims_dir().join("pip.cmd"), "").expect("pip cmd");
        fs::write(ctx.shims_dir().join(".pyenv-shims.json"), "{}").expect("manifest");

        let short = cmd_shims(&ctx, true);
        assert_eq!(short.exit_code, 0);
        assert_eq!(short.stdout, vec!["pip".to_string(), "python".to_string()]);

        let full = cmd_shims(&ctx, false);
        assert!(full.stdout.iter().any(|line| line.ends_with("python.exe")));
        assert!(full.stdout.iter().any(|line| line.ends_with("pip.cmd")));
    }

    #[test]
    fn completions_include_help_and_dynamic_values() {
        let (_temp, ctx) = test_context();
        fs::create_dir_all(ctx.versions_dir().join("3.12.6")).expect("version");

        let report = cmd_completions(&ctx, "global", &[]);
        assert_eq!(report.exit_code, 0);
        assert!(report.stdout.iter().any(|line| line == "--help"));
        assert!(report.stdout.iter().any(|line| line == "3.12.6"));
        assert!(report.stdout.iter().any(|line| line == "system"));

        let hooks = cmd_completions(&ctx, "hooks", &[]);
        assert_eq!(hooks.exit_code, 0);
        assert!(hooks.stdout.iter().any(|line| line == "install"));
        assert!(hooks.stdout.iter().any(|line| line == "rehash"));
    }
}
