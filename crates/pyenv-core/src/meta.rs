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
            "",
            "Examples:",
            "  pyenv help            Show all commands with summaries",
            "  pyenv help install    Show detailed help for the install command",
            "  pyenv help --usage global   Show only the usage line for global",
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
            "Sets or displays the Python version for the current directory by writing",
            "a `.python-version` file. When no version argument is given, prints the",
            "currently configured local version.",
            "",
            "The `-f|--force` flag allows writing a version even if it is not installed.",
            "`--unset` removes the `.python-version` file from the current directory.",
            "",
            "Version prefixes like `3.12` are resolved to the latest matching installed version.",
            "",
            "Examples:",
            "  pyenv local 3.13.12     Set the local version to 3.13.12",
            "  pyenv local 3.12        Set local to the latest installed 3.12.x",
            "  pyenv local --unset     Remove local version setting",
            "  pyenv local             Show the currently configured local version",
            "",
            "See also: pyenv global, pyenv shell, pyenv versions",
        ],
        completions: &["-f", "--force", "--unset"],
    },
    CommandDoc {
        name: "global",
        summary: "Set or show the global Python version",
        usage: "Usage: pyenv global [--unset] [version ...]",
        help: &[
            "Sets or displays the default Python version used system-wide by writing",
            "the global version file under PYENV_ROOT. When no version argument is",
            "given, prints the currently configured global version.",
            "",
            "`--unset` removes the global version file entirely, reverting to `system`.",
            "",
            "Version prefixes like `3.12` are resolved to the latest matching installed version.",
            "Multiple versions can be specified to set a priority list.",
            "",
            "Examples:",
            "  pyenv global 3.13.12       Set the global version to 3.13.12",
            "  pyenv global 3.12          Set global to the latest installed 3.12.x",
            "  pyenv global 3.12 3.11     Set a fallback chain: prefer 3.12, then 3.11",
            "  pyenv global --unset       Remove global version setting",
            "  pyenv global               Show the currently configured global version",
            "",
            "See also: pyenv local, pyenv shell, pyenv install",
        ],
        completions: &["--unset"],
    },
    CommandDoc {
        name: "shell",
        summary: "Set or show the shell-specific Python version",
        usage: "Usage: pyenv shell [--unset|-] [version ...]",
        help: &[
            "Sets the PYENV_VERSION environment variable for the current shell session.",
            "This takes the highest priority and overrides both local and global settings.",
            "",
            "Requires shell integration from `pyenv init`. Run `pyenv init` for setup instructions.",
            "",
            "Use `--unset` to clear the shell-specific selection.",
            "Use `-` to revert to the previously selected shell version.",
            "",
            "Examples:",
            "  pyenv shell 3.13.12       Use 3.13.12 in this shell session only",
            "  pyenv shell --unset       Clear the shell-specific version",
            "  pyenv shell -             Revert to the previous shell version",
            "",
            "See also: pyenv init, pyenv global, pyenv local",
        ],
        completions: &["--unset", "-"],
    },
    CommandDoc {
        name: "install",
        summary: "Install a Python version",
        usage: "Usage: pyenv install [-l|--list] [--known] [--family <family>] [--dry-run] [--json] [-f|--force] <version> ...",
        help: &[
            "Downloads and installs Python runtimes into the managed versions directory.",
            "Uses native provider backends (NuGet on Windows, source builds on Linux/macOS)",
            "for maximum reliability and speed.",
            "",
            "Version prefixes like `3.12` resolve to the latest available provider-backed version.",
            "",
            "Flags:",
            "  -l, --list         List all installable versions from native providers",
            "  --known            List from the embedded known-versions catalog instead",
            "  --family <name>    Filter --list by runtime family (cpython, pypy)",
            "  --dry-run          Show what would be installed without downloading anything",
            "  --json             Output results as JSON",
            "  -f, --force        Reinstall even if the version already exists",
            "",
            "Examples:",
            "  pyenv install --list              Show all installable versions",
            "  pyenv install --list --family pypy Show only PyPy versions",
            "  pyenv install 3.13.12             Install CPython 3.13.12",
            "  pyenv install 3.12                Install the latest available 3.12.x",
            "  pyenv install pypy3.11            Install the latest PyPy 3.11",
            "  pyenv install --dry-run 3.12      Preview what would be installed",
            "  pyenv install -f 3.13.12          Force reinstall 3.13.12",
            "",
            "See also: pyenv uninstall, pyenv versions, pyenv global",
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
            "Removes managed runtimes from the versions directory and refreshes shims.",
            "Use `-f|--force` to suppress the confirmation prompt.",
            "",
            "Examples:",
            "  pyenv uninstall 3.12.10      Remove CPython 3.12.10",
            "  pyenv uninstall -f 3.11.9    Remove without confirmation",
            "",
            "See also: pyenv install, pyenv versions",
        ],
        completions: &["-f", "--force"],
    },
    CommandDoc {
        name: "rehash",
        summary: "Rebuild pyenv shims",
        usage: "Usage: pyenv rehash",
        help: &[
            "Scans all managed runtimes and regenerates shim launchers for discovered",
            "executables (python, pip, etc.). Run this after installing packages that",
            "provide new command-line tools.",
            "",
            "This is run automatically after `pyenv install` and during `pyenv init`.",
        ],
        completions: &[],
    },
    CommandDoc {
        name: "version",
        summary: "Show the current Python version and how it was selected",
        usage: "Usage: pyenv version [--bare]",
        help: &[
            "Displays the currently selected Python version(s) and how they were chosen",
            "(environment variable, .python-version file, or global version file).",
            "",
            "`--bare` prints just the version names without the origin information.",
            "",
            "Examples:",
            "  pyenv version         3.13.12 (set by C:\\Users\\Roy\\.pyenv\\version)",
            "  pyenv version --bare  3.13.12",
            "",
            "See also: pyenv versions, pyenv version-name, pyenv version-origin",
        ],
        completions: &["--bare"],
    },
    CommandDoc {
        name: "versions",
        summary: "List all installed Python versions",
        usage: "Usage: pyenv versions [--bare] [--skip-aliases] [--skip-envs] [--executables]",
        help: &[
            "Lists all Python versions installed under the managed versions directory,",
            "marking the currently selected version with an asterisk (*).",
            "",
            "Flags:",
            "  --bare           Print version names only, without the selection marker",
            "  --skip-aliases   Exclude symlink/alias entries",
            "  --skip-envs      Exclude virtualenvs",
            "  --executables    Show the executable names found in each version",
            "",
            "Examples:",
            "  pyenv versions         Show all installed versions with the active one marked",
            "  pyenv versions --bare  Print just the version names, one per line",
            "",
            "See also: pyenv version, pyenv install --list",
        ],
        completions: &["--bare", "--skip-aliases", "--skip-envs", "--executables"],
    },
    CommandDoc {
        name: "which",
        summary: "Display the full path to an executable",
        usage: "Usage: pyenv which [--nosystem] [--skip-advice] <command>",
        help: &[
            "Searches the currently selected pyenv runtimes first, then the system PATH,",
            "and prints the full path to the matching executable.",
            "",
            "`--nosystem` restricts the search to managed runtimes only.",
            "`--skip-advice` suppresses the helpful hint when the command is not found.",
            "",
            "Examples:",
            "  pyenv which python    /home/user/.pyenv/versions/3.13.12/bin/python",
            "  pyenv which pip       Show where pip resolves to",
            "",
            "See also: pyenv whence, pyenv prefix",
        ],
        completions: &["--nosystem", "--skip-advice"],
    },
    CommandDoc {
        name: "whence",
        summary: "List Python versions that contain the given executable",
        usage: "Usage: pyenv whence [--path] <command>",
        help: &[
            "Prints all managed versions that contain a matching executable.",
            "Use `--path` to show the full executable paths instead of version names.",
            "",
            "Examples:",
            "  pyenv whence python        List versions containing `python`",
            "  pyenv whence --path pip    Show full paths to `pip` in each version",
        ],
        completions: &["--path"],
    },
    CommandDoc {
        name: "exec",
        summary: "Run a command with the selected Python version",
        usage: "Usage: pyenv exec <command> [arg1 arg2...]",
        help: &[
            "Prepares PATH for the active runtime selection and then executes the",
            "requested program. Useful for running commands through a specific Python",
            "version without changing the global or local setting.",
            "",
            "Examples:",
            "  pyenv exec python -c \"import sys; print(sys.version)\"",
            "  pyenv exec pip install requests",
        ],
        completions: &[],
    },
    CommandDoc {
        name: "root",
        summary: "Display the current pyenv root directory",
        usage: "Usage: pyenv root",
        help: &[
            "Prints the root directory where managed runtimes, shims, cache, and config",
            "are stored. Defaults to `~/.pyenv` unless PYENV_ROOT is set or the binary",
            "is launched from a portable layout.",
        ],
        completions: &[],
    },
    CommandDoc {
        name: "prefix",
        summary: "Display the installation prefix for one or more versions",
        usage: "Usage: pyenv prefix [version ...]",
        help: &[
            "Prints the directory path for the specified version(s). When no version",
            "is specified, prints the prefix for the currently selected version.",
            "",
            "Examples:",
            "  pyenv prefix                Show prefix for the active version",
            "  pyenv prefix 3.13.12        Show prefix for a specific version",
        ],
        completions: &[],
    },
    CommandDoc {
        name: "latest",
        summary: "Resolve the latest installed or known matching version",
        usage: "Usage: pyenv latest [-k|--known] [-b|--bypass] [-f|--force] <prefix>",
        help: &[
            "Resolves version prefixes such as `3.12` or `pypy3.11` to the latest",
            "matching version. By default, searches installed versions; use `--known`",
            "to search the full catalog instead.",
            "",
            "Examples:",
            "  pyenv latest 3.12             Latest installed 3.12.x",
            "  pyenv latest --known 3.14     Latest known 3.14.x (may not be installed)",
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
            "Use `--short` to print command names only instead of full paths.",
            "",
            "Examples:",
            "  pyenv shims            Show full paths to all shim files",
            "  pyenv shims --short    Show just the command names (python, pip, etc.)",
        ],
        completions: &["--short"],
    },
    CommandDoc {
        name: "init",
        summary: "Configure the shell environment for pyenv",
        usage: "Usage: pyenv init [-|--path] [--detect-shell] [--no-push-path] [--no-rehash] [<shell>]",
        help: &[
            "Prints shell initialization code that adds shims to PATH, sets the PYENV_SHELL",
            "variable, and installs the `pyenv` shell function for shell-level features like",
            "`pyenv shell`.",
            "",
            "Supported shells: pwsh (PowerShell), cmd, bash, zsh, fish, sh",
            "",
            "Setup (add to your shell profile):",
            "  PowerShell:   iex ((pyenv init - pwsh) -join \"`n\")",
            "  Bash:         eval \"$(pyenv init - bash)\"",
            "  Zsh:          eval \"$(pyenv init - zsh)\"",
            "  Fish:         pyenv init - fish | source",
            "",
            "Flags:",
            "  -               Print full init code (PATH + shell function)",
            "  --path          Print only the PATH setup code",
            "  --no-push-path  Guard against duplicate shims entries in PATH",
            "  --no-rehash     Skip automatic rehash during init",
        ],
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
        help: &[
            "Performs a series of health checks and reports potential problems:",
            "  - Verifies PYENV_ROOT exists and is accessible",
            "  - Checks that pyenv bin and shims directories are on PATH",
            "  - Detects pyenv-win conflicts (stale PYENV_ROOT, PATH shadowing)",
            "  - Warns about Windows Store Python aliases",
            "  - On Linux/macOS, verifies source-build prerequisites (make, gcc, etc.)",
            "",
            "Use `--json` for structured output suitable for scripts and automation.",
            "",
            "Examples:",
            "  pyenv doctor          Human-readable diagnostic output",
            "  pyenv doctor --json   JSON-formatted diagnostic output",
        ],
        completions: &["--json"],
    },
    CommandDoc {
        name: "self-update",
        summary: "Check for or install the latest published pyenv-native release",
        usage: "Usage: pyenv self-update [--check] [--yes] [--force] [--github-repo <owner/repo>] [--tag <vX.Y.Z>]",
        help: &[
            "Checks GitHub releases for pyenv-native and upgrades the current portable install",
            "in place. By default it targets the latest published release for the canonical repo.",
            "",
            "Use `--check` to see whether an update is available without installing anything.",
            "Use `--tag` to reinstall or roll to a specific published release tag.",
            "Use `--yes` for unattended automation.",
            "",
            "Examples:",
            "  pyenv self-update              Update to the latest published release",
            "  pyenv self-update --check      Check whether a newer release exists",
            "  pyenv self-update --tag v0.1.8 Reinstall or pin to a specific release",
            "",
            "This command is intended for portable installs launched from PYENV_ROOT/bin.",
        ],
        completions: &["--check", "--yes", "--force", "--github-repo", "--tag"],
    },
    CommandDoc {
        name: "config",
        summary: "Inspect or change pyenv configuration",
        usage: "Usage: pyenv config path|show|get|set ...",
        help: &[
            "Reads or updates the persisted `config.toml` under the active pyenv root.",
            "",
            "Subcommands:",
            "  path              Show the path to the config file",
            "  show              Print all current configuration",
            "  get <key>         Print the value of a specific config key",
            "  set <key> <value> Update a config key",
            "",
            "Examples:",
            "  pyenv config show              Show all settings",
            "  pyenv config get install.arch   Show the configured architecture",
            "  pyenv config set install.bootstrap_pip true",
        ],
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
        stdout.push("CORE CONCEPTS:".to_string());
        stdout.push("  Shims:       Lightweight executables (like `python` or `pip`) that intercept your commands".to_string());
        stdout.push("               and route them to the correct Python version based on your current environment.".to_string());
        stdout.push(
            "               Run `pyenv rehash` to refresh these after installing new pip packages."
                .to_string(),
        );
        stdout.push("  Versions:    Python environments installed via `pyenv install`. Located in `~/.pyenv/versions`.".to_string());
        stdout.push("  Selection:   Pyenv decides which Python version to use in this order (highest priority first):".to_string());
        stdout.push(
            "                 1. PYENV_VERSION environment variable (set via `pyenv shell`)"
                .to_string(),
        );
        stdout.push("                 2. .python-version file in the current directory (set via `pyenv local`)".to_string());
        stdout.push(
            "                 3. The global version file (set via `pyenv global`)".to_string(),
        );
        stdout.push(String::new());
        stdout
            .push("See `pyenv help <command>` for information on a specific command.".to_string());
        stdout.push(
            "For full documentation, see: https://github.com/imyourboyroy/pyenv-native".to_string(),
        );
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
        "latest" => known_version_names().to_vec(),
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
