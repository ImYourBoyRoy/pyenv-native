// ./crates/pyenv-cli/src/main.rs
//! CLI entrypoint for the native-first pyenv implementation.

use std::io::{self, Write};
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
use pyenv_core::{
    AppContext, CommandReport, DoctorFix, InstallCommandOptions, SelfUpdateOptions, VenvUseScope,
    VersionsCommandOptions, apply_doctor_fixes, cmd_available, cmd_commands, cmd_completions,
    cmd_config_get, cmd_config_path, cmd_config_set, cmd_config_show, cmd_doctor, cmd_exec,
    cmd_external, cmd_global, cmd_help, cmd_hooks, cmd_init, cmd_install, cmd_latest, cmd_local,
    cmd_prefix, cmd_rehash, cmd_root, cmd_self_update, cmd_sh_cmd, cmd_sh_rehash, cmd_sh_shell,
    cmd_shell, cmd_shims, cmd_uninstall, cmd_venv_create, cmd_venv_delete, cmd_venv_info,
    cmd_venv_list, cmd_venv_rename, cmd_venv_use, cmd_version, cmd_version_file,
    cmd_version_file_read, cmd_version_file_write, cmd_version_name, cmd_version_origin,
    cmd_versions, cmd_whence, cmd_which, doctor_fix_plan,
};

#[derive(Debug, Parser)]
#[command(
    name = "pyenv",
    version,
    about = "Native-first, cross-platform Python version manager",
    long_about = "Native-first, cross-platform Python version manager.\n\nManage multiple Python versions with local, global, and shell-scoped selection.\nRun `pyenv help` for detailed command information and examples.",
    after_help = "CORE CONCEPTS:\n  Shims:       Lightweight executables (like `python` or `pip`) that intercept your commands\n               and route them to the correct Python version based on your current environment.\n               Run `pyenv rehash` to refresh these after installing new pip packages.\n  Versions:    Python environments installed via `pyenv install`. Located in `~/.pyenv/versions`.\n  Managed envs: Named virtual environments can live under `~/.pyenv/versions/<runtime>/envs`.\n               Use `pyenv venv create 3.13 api` and bind a folder with `pyenv venv use api`.\n  Discovery:   Search installable runtimes with `pyenv install --list 3.13` or `pyenv available 3.13`.\n  Selection:   Pyenv decides which Python version to use in this order (highest priority first):\n                 1. PYENV_VERSION environment variable (set via `pyenv shell`)\n                 2. .python-version file in the current directory (set via `pyenv local`)\n                 3. The global version file (set via `pyenv global`)\n\nRun `pyenv help <command>` for detailed help on any command.\nFull documentation: https://github.com/imyourboyroy/pyenv-native",
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

#[derive(Debug, Subcommand)]
enum VenvCommands {
    #[command(about = "List managed virtual environments under PYENV_ROOT/versions/*/envs")]
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
        Commands::Doctor { json, fix, force } => {
            if fix {
                if json {
                    CommandReport::failure(
                        vec!["pyenv: `doctor --json` cannot be combined with `--fix`".to_string()],
                        1,
                    )
                } else {
                    run_doctor_fix_flow(&ctx, force)
                }
            } else {
                cmd_doctor(&ctx, json)
            }
        }
        Commands::SelfUpdate {
            check,
            yes,
            force,
            github_repo,
            tag,
        } => cmd_self_update(
            &ctx,
            &SelfUpdateOptions {
                check,
                yes,
                force,
                github_repo,
                tag,
            },
        ),
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
        Commands::Available {
            json,
            known,
            family,
            pattern,
        } => cmd_available(&ctx, family, pattern, known, json),
        Commands::Uninstall { force, versions } => cmd_uninstall(&ctx, &versions, force),
        Commands::Venv { command } => match command {
            VenvCommands::List { bare, json } => cmd_venv_list(&ctx, bare, json),
            VenvCommands::Info { json, spec } => cmd_venv_info(&ctx, &spec, json),
            VenvCommands::Create {
                force,
                set_local,
                version,
                name,
            } => cmd_venv_create(&ctx, &version, &name, force, set_local),
            VenvCommands::Delete { force, spec } => cmd_venv_delete(&ctx, &spec, force),
            VenvCommands::Rename { spec, new_name } => cmd_venv_rename(&ctx, &spec, &new_name),
            VenvCommands::Use { global, spec } => cmd_venv_use(
                &ctx,
                &spec,
                if global {
                    VenvUseScope::Global
                } else {
                    VenvUseScope::Local
                },
            ),
        },
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

fn run_doctor_fix_flow(ctx: &AppContext, force: bool) -> CommandReport {
    let plan = doctor_fix_plan(ctx);
    let automated = plan
        .iter()
        .filter(|item| item.automated)
        .cloned()
        .collect::<Vec<_>>();
    let manual = plan
        .iter()
        .filter(|item| !item.automated)
        .cloned()
        .collect::<Vec<_>>();

    if automated.is_empty() {
        let mut stdout = vec!["No automated doctor fixes are currently available.".to_string()];
        if !manual.is_empty() {
            stdout.push(String::new());
            stdout.push("Manual follow-up suggested:".to_string());
            stdout.extend(render_doctor_fixes(&manual));
        }
        stdout.push(String::new());
        stdout.extend(cmd_doctor(ctx, false).stdout);
        return CommandReport::success(stdout);
    }

    if !force && !confirm_doctor_fixes(&automated) {
        return CommandReport::failure(vec!["pyenv: doctor fixes cancelled".to_string()], 1);
    }

    match apply_doctor_fixes(ctx) {
        Ok(outcome) => {
            let mut stdout = vec!["Applied automated doctor fixes:".to_string()];
            stdout.extend(
                outcome
                    .applied
                    .into_iter()
                    .map(|item| format!("  - {item}")),
            );
            if !manual.is_empty() {
                stdout.push(String::new());
                stdout.push("Manual follow-up still recommended:".to_string());
                stdout.extend(render_doctor_fixes(&manual));
            }
            stdout.push(String::new());
            stdout.push("Updated doctor report:".to_string());
            stdout.push(String::new());
            stdout.extend(cmd_doctor(ctx, false).stdout);
            CommandReport::success(stdout)
        }
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

fn render_doctor_fixes(fixes: &[DoctorFix]) -> Vec<String> {
    fixes
        .iter()
        .map(|item| {
            if let Some(command_hint) = &item.command_hint {
                format!("  - {} ({command_hint})", item.description)
            } else {
                format!("  - {}", item.description)
            }
        })
        .collect()
}

fn confirm_doctor_fixes(fixes: &[DoctorFix]) -> bool {
    let _ = writeln!(io::stdout(), "Automated doctor fixes available:");
    for line in render_doctor_fixes(fixes) {
        let _ = writeln!(io::stdout(), "{line}");
    }
    let _ = write!(io::stdout(), "\nApply these fixes now? [y/N] ");
    let _ = io::stdout().flush();

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes"),
        Err(_) => false,
    }
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
