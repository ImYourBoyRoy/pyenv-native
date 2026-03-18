// ./crates/pyenv-cli/src/main.rs
//! CLI entrypoint for the native-first pyenv implementation.

use std::io::{self, Write};
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
use pyenv_core::{
    AppContext, CommandReport, InstallCommandOptions, VersionsCommandOptions, cmd_commands,
    cmd_completions, cmd_config_get, cmd_config_path, cmd_config_set, cmd_config_show, cmd_doctor,
    cmd_exec, cmd_external, cmd_global, cmd_help, cmd_hooks, cmd_init, cmd_install, cmd_latest,
    cmd_local, cmd_prefix, cmd_rehash, cmd_root, cmd_sh_cmd, cmd_sh_rehash, cmd_sh_shell,
    cmd_shell, cmd_shims, cmd_uninstall, cmd_version, cmd_version_file, cmd_version_file_read,
    cmd_version_file_write, cmd_version_name, cmd_version_origin, cmd_versions, cmd_whence,
    cmd_which,
};

#[derive(Debug, Parser)]
#[command(
    name = "pyenv",
    version,
    about = "Native-first, cross-platform Python version manager",
    long_about = "Native-first, cross-platform Python version manager.\n\nManage multiple Python versions with local, global, and shell-scoped selection.\nRun `pyenv help` for detailed command information and examples.",
    after_help = "CORE CONCEPTS:\n  Shims:       Lightweight executables (like `python` or `pip`) that intercept your commands\n               and route them to the correct Python version based on your current environment.\n               Run `pyenv rehash` to refresh these after installing new pip packages.\n  Versions:    Python environments installed via `pyenv install`. Located in `~/.pyenv/versions`.\n  Selection:   Pyenv decides which Python version to use in this order (highest priority first):\n                 1. PYENV_VERSION environment variable (set via `pyenv shell`)\n                 2. .python-version file in the current directory (set via `pyenv local`)\n                 3. The global version file (set via `pyenv global`)\n\nRun `pyenv help <command>` for detailed help on any command.\nFull documentation: https://github.com/imyourboyroy/pyenv-native",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
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
    #[command(about = "List executable hooks for a given command")]
    Hooks { hook: String },
    #[command(about = "Verify pyenv installation and environment health")]
    Doctor {
        #[arg(long = "json")]
        json: bool,
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
    #[command(about = "Uninstall a specific Python version")]
    Uninstall {
        #[arg(short = 'f', long = "force")]
        force: bool,
        versions: Vec<String>,
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
enum ConfigCommands {
    #[command(about = "Show the path to the config file")]
    Path,
    #[command(about = "Print all current configuration")]
    Show,
    #[command(about = "Print the value of a specific config key")]
    Get { key: String },
    #[command(about = "Update a config key")]
    Set { key: String, value: String },
}

fn main() -> ExitCode {
    if let Some(command_name) = shim_invocation_name() {
        let args = std::env::args().skip(1).collect::<Vec<_>>();
        let ctx = match AppContext::from_system() {
            Ok(ctx) => ctx,
            Err(error) => {
                let _ = writeln!(io::stderr(), "{error}");
                return ExitCode::from(1);
            }
        };
        return emit_report(cmd_exec(&ctx, &command_name, &args));
    }

    let cli = Cli::parse();

    let Some(command) = cli.command else {
        let _ = Cli::command().print_help();
        let _ = writeln!(io::stdout());
        return ExitCode::from(1);
    };

    let mut ctx = match AppContext::from_system() {
        Ok(ctx) => ctx,
        Err(error) => {
            let _ = writeln!(io::stderr(), "{error}");
            return ExitCode::from(1);
        }
    };

    let report = match command {
        Commands::Help { usage, command } => cmd_help(&ctx, command.as_deref(), usage),
        Commands::Commands { sh, no_sh } => cmd_commands(&ctx, sh, no_sh),
        Commands::Root => cmd_root(&ctx),
        Commands::Hooks { hook } => cmd_hooks(&ctx, &hook),
        Commands::Doctor { json } => cmd_doctor(&ctx, json),
        Commands::Config { command } => match command.unwrap_or(ConfigCommands::Show) {
            ConfigCommands::Path => cmd_config_path(&ctx),
            ConfigCommands::Show => cmd_config_show(&ctx),
            ConfigCommands::Get { key } => cmd_config_get(&ctx, &key),
            ConfigCommands::Set { key, value } => cmd_config_set(&mut ctx, &key, &value),
        },
        Commands::VersionFile { dir } => cmd_version_file(&ctx, dir.as_deref()),
        Commands::VersionFileRead { file } => cmd_version_file_read(&file),
        Commands::VersionFileWrite {
            force,
            file,
            versions,
        } => cmd_version_file_write(&ctx, &file, &versions, force),
        Commands::VersionOrigin => cmd_version_origin(&ctx),
        Commands::VersionName { force } => cmd_version_name(&ctx, force),
        Commands::Version { bare } => cmd_version(&ctx, bare),
        Commands::Global { unset, versions } => cmd_global(&ctx, &versions, unset),
        Commands::Local {
            force,
            unset,
            versions,
        } => cmd_local(&ctx, &versions, unset, force),
        Commands::Latest {
            known,
            bypass,
            force,
            prefix,
        } => cmd_latest(&ctx, &prefix, known, bypass, force),
        Commands::Prefix { versions } => cmd_prefix(&ctx, &versions),
        Commands::Init { args } => cmd_init(&ctx, &args),
        Commands::Shell { args } => cmd_shell(&ctx, &args),
        Commands::Versions {
            bare,
            skip_aliases,
            skip_envs,
            executables,
        } => cmd_versions(
            &ctx,
            &VersionsCommandOptions {
                bare,
                skip_aliases,
                skip_envs,
                executables,
            },
        ),
        Commands::Which {
            no_system,
            skip_advice,
            command,
        } => cmd_which(&ctx, &command, no_system, skip_advice),
        Commands::Whence { path, command } => cmd_whence(&ctx, &command, path),
        Commands::Install {
            list,
            force,
            dry_run,
            json,
            known,
            family,
            versions,
        } => cmd_install(
            &ctx,
            &InstallCommandOptions {
                list,
                force,
                dry_run,
                json,
                known,
                family,
                versions,
            },
        ),
        Commands::Uninstall { force, versions } => cmd_uninstall(&ctx, &versions, force),
        Commands::Rehash => cmd_rehash(&ctx),
        Commands::Shims { short } => cmd_shims(&ctx, short),
        Commands::Completions { command, args } => cmd_completions(&ctx, &command, &args),
        Commands::ShShell { args } => cmd_sh_shell(&ctx, &args),
        Commands::ShRehash => cmd_sh_rehash(&ctx),
        Commands::ShCmd { args } => cmd_sh_cmd(&ctx, &args),
        Commands::Exec { command, args } => cmd_exec(&ctx, &command, &args),
        Commands::External(args) => cmd_external(&ctx, &args),
    };

    emit_report(report)
}

fn shim_invocation_name() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let stem = Path::new(&exe).file_stem()?.to_string_lossy().to_string();
    let lowered = stem.to_ascii_lowercase();
    if matches!(lowered.as_str(), "pyenv" | "pyenv-cli" | "cargo" | "rustc") {
        None
    } else {
        Some(stem)
    }
}

fn emit_report(report: CommandReport) -> ExitCode {
    for line in report.stderr {
        let _ = writeln!(io::stderr(), "{line}");
    }

    for line in report.stdout {
        let _ = writeln!(io::stdout(), "{line}");
    }

    ExitCode::from(report.exit_code as u8)
}
