// ./crates/pyenv-cli/src/cli.rs
//! Clap command definitions for the `pyenv` binary. This module centralizes the public CLI
//! surface, subcommand nesting, and argument metadata while leaving execution logic to the
//! dispatch module.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "pyenv",
    version,
    about = "Native-first, cross-platform Python version manager",
    long_about = "Native-first, cross-platform Python version manager.\n\nManage multiple Python versions with local, global, and shell-scoped selection.\nRun `pyenv help` for detailed command information and examples.",
    after_help = "CORE CONCEPTS:\n  Shims:       Lightweight executables (like `python` or `pip`) that intercept your commands\n               and route them to the correct Python version based on your current environment.\n               Run `pyenv rehash` to refresh these after installing new pip packages.\n  Versions:    Python environments installed via `pyenv install`. Located in `~/.pyenv/versions`.\n  Managed envs: Named virtual environments can live under `~/.pyenv/venvs/<runtime>/<name>`.\n               Use `pyenv venv create 3.13 api` and bind a folder with `pyenv venv use api`.\n               Compatibility aliases like `pyenv virtualenv` and `pyenv activate` are supported,\n               but `pyenv venv ...` plus `.python-version` remains the preferred workflow.\n  Discovery:   Search installable runtimes with `pyenv install --list 3.13` or `pyenv available 3.13`.\n  Selection:   Pyenv decides which Python version to use in this order (highest priority first):\n                 1. PYENV_VERSION environment variable (set via `pyenv shell`/`pyenv activate`)\n                 2. .python-version file in the current directory (set via `pyenv local` or `pyenv venv use`)\n                 3. The global version file (set via `pyenv global`)\n\nRun `pyenv help <command>` for detailed help on any command.\nFull documentation: https://github.com/imyourboyroy/pyenv-native",
    disable_help_subcommand = true
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    #[command(about = "Display help for a command")]
    Help {
        #[arg(long = "usage")]
        usage: bool,
        command: Option<String>,
    },
    #[command(about = "List all available pyenv commands")]
    #[allow(clippy::enum_variant_names)]
    Commands {
        #[arg(long = "sh")]
        sh: bool,
        #[arg(long = "no-sh")]
        no_sh: bool,
    },
    #[command(about = "Display the root directory where versions and shims are kept")]
    Root,
    #[command(about = "Launch the beautiful Pyenv Native GUI dashboard")]
    Gui,
    #[command(about = "List executable hooks for a given command")]
    Hooks { hook: String },
    #[command(about = "Verify pyenv installation and environment health")]
    Doctor {
        #[arg(long = "json")]
        json: bool,
        #[arg(
            long = "fix",
            help = "Apply safe automated fixes after showing diagnostics"
        )]
        fix: bool,
        #[arg(
            short = 'f',
            long = "force",
            help = "Skip the confirmation prompt when used with --fix"
        )]
        force: bool,
    },
    #[command(about = "Check for or install the latest published pyenv-native release")]
    SelfUpdate {
        #[arg(long = "check", help = "Check for updates without installing")]
        check: bool,
        #[arg(short = 'y', long = "yes", help = "Skip the confirmation prompt")]
        yes: bool,
        #[arg(
            short = 'f',
            long = "force",
            help = "Reinstall even when the current version already matches the target release"
        )]
        force: bool,
        #[arg(
            long = "github-repo",
            help = "GitHub owner/repo that publishes pyenv-native release bundles"
        )]
        github_repo: Option<String>,
        #[arg(long = "tag", help = "Specific release tag to install, such as v0.1.8")]
        tag: Option<String>,
    },
    #[command(about = "Uninstall pyenv-native from your system")]
    SelfUninstall {
        #[arg(short = 'y', long = "yes", help = "Skip the confirmation prompt")]
        yes: bool,
    },
    #[command(
        hide = true,
        name = "update",
        about = "Compatibility alias for `pyenv self-update`"
    )]
    Update {
        #[arg(long = "check", help = "Check for updates without installing")]
        check: bool,
        #[arg(short = 'y', long = "yes", help = "Skip the confirmation prompt")]
        yes: bool,
        #[arg(
            short = 'f',
            long = "force",
            help = "Reinstall even when the current version already matches the target release"
        )]
        force: bool,
        #[arg(
            long = "github-repo",
            help = "GitHub owner/repo that publishes pyenv-native release bundles"
        )]
        github_repo: Option<String>,
        #[arg(long = "tag", help = "Specific release tag to install, such as v0.1.8")]
        tag: Option<String>,
    },
    #[command(about = "Display or modify pyenv-native configuration")]
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommands>,
    },
    #[command(about = "Detect the file that sets the current pyenv version")]
    VersionFile { dir: Option<PathBuf> },
    #[command(about = "Read the contents of a .python-version file")]
    VersionFileRead { file: PathBuf },
    #[command(hide = true, name = "version-file-write")]
    VersionFileWrite {
        #[arg(short = 'f', long = "force")]
        force: bool,
        file: PathBuf,
        versions: Vec<String>,
    },
    #[command(about = "Explain how the current Python version is set")]
    VersionOrigin,
    #[command(about = "Show the current Python version")]
    VersionName {
        #[arg(short = 'f', long = "force")]
        force: bool,
    },
    #[command(about = "Show the current Python version and its origin")]
    Version {
        #[arg(long = "bare")]
        bare: bool,
    },
    #[command(about = "Show the comprehensive environment status (versions, origins, venvs)")]
    Status {
        #[arg(long = "json")]
        json: bool,
    },
    #[command(about = "Print a concise prompt string for the current environment")]
    Prompt,

    #[command(about = "Set or show the global Python version")]
    Global {
        #[arg(long = "unset", help = "Remove the global version file")]
        unset: bool,
        #[arg(help = "Version(s) to set globally (e.g. 3.13.12, 3.12)")]
        versions: Vec<String>,
    },
    #[command(about = "Set or show the local directory Python version")]
    Local {
        #[arg(
            short = 'f',
            long = "force",
            help = "Write even if version is not installed"
        )]
        force: bool,
        #[arg(long = "unset", help = "Remove the .python-version file")]
        unset: bool,
        #[arg(help = "Version(s) to set locally (e.g. 3.13.12, 3.12)")]
        versions: Vec<String>,
    },
    #[command(about = "Print the latest installed or known version matching the prefix")]
    Latest {
        #[arg(short = 'k', long = "known")]
        known: bool,
        #[arg(short = 'b', long = "bypass")]
        bypass: bool,
        #[arg(short = 'f', long = "force")]
        force: bool,
        prefix: String,
    },
    #[command(about = "Display paths where the given Python versions are installed")]
    Prefix { versions: Vec<String> },
    #[command(
        about = "Configure the shell environment for pyenv",
        trailing_var_arg = true
    )]
    Init {
        #[arg(allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(
        about = "Set or show the shell-specific Python version",
        trailing_var_arg = true
    )]
    Shell {
        #[arg(allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(
        hide = true,
        about = "Compatibility alias for activating a managed virtual environment",
        trailing_var_arg = true
    )]
    Activate {
        #[arg(allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(
        hide = true,
        about = "Compatibility alias for deactivating the current managed virtual environment",
        trailing_var_arg = true
    )]
    Deactivate {
        #[arg(allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(about = "List all Python versions available to pyenv")]
    Versions {
        #[arg(long = "bare")]
        bare: bool,
        #[arg(long = "skip-aliases")]
        skip_aliases: bool,
        #[arg(long = "skip-envs")]
        skip_envs: bool,
        #[arg(long = "executables")]
        executables: bool,
    },
    #[command(about = "Display the full path to an executable")]
    Which {
        #[arg(long = "nosystem")]
        no_system: bool,
        #[arg(long = "skip-advice")]
        skip_advice: bool,
        command: String,
    },
    #[command(about = "List all Python versions that contain the given executable")]
    Whence {
        #[arg(long = "path")]
        path: bool,
        command: String,
    },
    #[command(about = "Install Python versions from native providers")]
    Install {
        #[arg(short = 'l', long = "list", help = "List all installable versions")]
        list: bool,
        #[arg(
            short = 'f',
            long = "force",
            help = "Reinstall even if already installed"
        )]
        force: bool,
        #[arg(long = "dry-run", help = "Preview without downloading")]
        dry_run: bool,
        #[arg(long = "json", help = "Output results as JSON")]
        json: bool,
        #[arg(long = "known", help = "Use embedded catalog instead of providers")]
        known: bool,
        #[arg(long = "family", help = "Filter by runtime family (cpython, pypy)")]
        family: Option<String>,
        #[arg(help = "Version(s) to install (e.g. 3.13.12, 3.12, pypy3.11)")]
        versions: Vec<String>,
    },
    #[command(about = "List installable Python versions from native providers")]
    Available {
        #[arg(long = "json", help = "Output results as JSON")]
        json: bool,
        #[arg(
            long = "known",
            help = "Use the embedded known catalog instead of providers"
        )]
        known: bool,
        #[arg(long = "family", help = "Filter by runtime family (cpython, pypy)")]
        family: Option<String>,
        #[arg(help = "Optional pattern such as 3, 3.12, 3.13, or pypy3.11")]
        pattern: Option<String>,
    },
    #[command(about = "Uninstall a specific Python version")]
    Uninstall {
        #[arg(short = 'f', long = "force")]
        force: bool,
        versions: Vec<String>,
    },
    #[command(about = "Create, inspect, and assign managed virtual environments")]
    Venv {
        #[command(subcommand)]
        command: VenvCommands,
    },
    #[command(hide = true, about = "Compatibility alias for `pyenv venv create`")]
    Virtualenv {
        #[arg(short = 'f', long = "force")]
        force: bool,
        #[arg(long = "set-local")]
        set_local: bool,
        #[arg(help = "Runtime + name, or just the env name to reuse the current selected runtime")]
        args: Vec<String>,
    },
    #[command(hide = true, about = "Compatibility alias for `pyenv venv list`")]
    Virtualenvs {
        #[arg(long = "bare")]
        bare: bool,
        #[arg(long = "json")]
        json: bool,
    },
    #[command(
        hide = true,
        name = "virtualenv-delete",
        about = "Compatibility alias for `pyenv venv delete`"
    )]
    VirtualenvDelete {
        #[arg(short = 'f', long = "force")]
        force: bool,
        spec: String,
    },
    #[command(
        hide = true,
        name = "virtualenv-prefix",
        about = "Print the full path to a managed virtual environment"
    )]
    VirtualenvPrefix { spec: Option<String> },
    #[command(
        hide = true,
        name = "virtualenv-init",
        about = "Compatibility alias for shell init output that supports activate/deactivate",
        trailing_var_arg = true
    )]
    VirtualenvInit {
        #[arg(allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(about = "Rehash pyenv shims (installs executables across all versions)")]
    Rehash,
    #[command(about = "List existing pyenv shims")]
    Shims {
        #[arg(long = "short")]
        short: bool,
    },
    #[command(about = "Print command completion script", trailing_var_arg = true)]
    Completions {
        command: String,
        #[arg(allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(hide = true, trailing_var_arg = true, name = "sh-shell")]
    ShShell {
        #[arg(allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(hide = true, trailing_var_arg = true, name = "sh-activate")]
    ShActivate {
        #[arg(allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(hide = true, trailing_var_arg = true, name = "sh-deactivate")]
    ShDeactivate {
        #[arg(allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(hide = true, name = "sh-rehash")]
    ShRehash,
    #[command(hide = true, trailing_var_arg = true, name = "sh-cmd")]
    ShCmd {
        #[arg(allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(
        about = "Run an executable with the selected Python version",
        trailing_var_arg = true
    )]
    Exec {
        command: String,
        #[arg(allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(Debug, Subcommand)]
pub(crate) enum ConfigCommands {
    #[command(about = "Show the path to the config file")]
    Path,
    #[command(about = "Print all current configuration")]
    Show,
    #[command(about = "Print the value of a specific config key")]
    Get { key: String },
    #[command(about = "Update a config key")]
    Set { key: String, value: String },
}

#[derive(Debug, Subcommand)]
pub(crate) enum VenvCommands {
    #[command(about = "List managed virtual environments under PYENV_ROOT/venvs/<runtime>/<name>")]
    List {
        #[arg(long = "bare", help = "Print only env specs like 3.13.12/envs/demo")]
        bare: bool,
        #[arg(long = "json", help = "Output the env inventory as JSON")]
        json: bool,
    },
    #[command(about = "Show details for a managed virtual environment")]
    Info {
        #[arg(long = "json", help = "Output the env details as JSON")]
        json: bool,
        #[arg(help = "Env name or full env spec like 3.13.12/envs/demo")]
        spec: String,
    },
    #[command(about = "Create a managed virtual environment under a specific runtime")]
    Create {
        #[arg(
            short = 'f',
            long = "force",
            help = "Recreate the env if the exact target already exists"
        )]
        force: bool,
        #[arg(
            long = "set-local",
            help = "Write the new env spec into the current directory's .python-version file"
        )]
        set_local: bool,
        #[arg(help = "Installed runtime version or prefix, such as 3.12 or 3.13.12")]
        version: String,
        #[arg(help = "Managed env name, such as app or tooling")]
        name: String,
    },
    #[command(about = "Remove a managed virtual environment")]
    Delete {
        #[arg(short = 'f', long = "force", help = "Skip the confirmation prompt")]
        force: bool,
        #[arg(help = "Env name or full env spec like 3.13.12/envs/demo")]
        spec: String,
    },
    #[command(about = "Rename a managed virtual environment")]
    Rename {
        #[arg(help = "Env name or full env spec like 3.13.12/envs/demo")]
        spec: String,
        #[arg(help = "New env name")]
        new_name: String,
    },
    #[command(about = "Assign a managed virtual environment to the current directory or globally")]
    Use {
        #[arg(
            long = "global",
            help = "Write the env spec into the global version file"
        )]
        global: bool,
        #[arg(help = "Env name or full env spec like 3.13.12/envs/demo")]
        spec: String,
    },
}
