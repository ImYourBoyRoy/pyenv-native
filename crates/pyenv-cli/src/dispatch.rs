// ./crates/pyenv-cli/src/dispatch.rs
//! CLI dispatch and terminal interaction for the `pyenv` binary. This module parses the
//! command context, routes clap subcommands into `pyenv-core`, and handles prompt/report I/O.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use pyenv_core::{
    AppContext, CommandReport, DoctorFix, InstallCommandOptions, SelfUpdateOptions, VenvUseScope,
    VersionsCommandOptions, apply_doctor_fixes, cmd_activate, cmd_available, cmd_commands,
    cmd_completions, cmd_config_get, cmd_config_path, cmd_config_set, cmd_config_show,
    cmd_deactivate, cmd_doctor, cmd_exec, cmd_external, cmd_global, cmd_help, cmd_hooks, cmd_init,
    cmd_install, cmd_latest, cmd_local, cmd_prefix, cmd_prompt, cmd_rehash, cmd_root,
    cmd_self_uninstall, cmd_self_update, cmd_sh_activate, cmd_sh_cmd, cmd_sh_deactivate,
    cmd_sh_rehash, cmd_sh_shell, cmd_shell, cmd_shims, cmd_status, cmd_uninstall, cmd_venv_create,
    cmd_venv_delete, cmd_venv_info, cmd_venv_list, cmd_venv_rename, cmd_venv_use, cmd_version,
    cmd_version_file, cmd_version_file_read, cmd_version_file_write, cmd_version_name,
    cmd_version_origin, cmd_versions, cmd_virtualenv, cmd_virtualenv_delete, cmd_virtualenv_init,
    cmd_virtualenv_prefix, cmd_virtualenvs, cmd_whence, cmd_which, doctor_fix_plan,
};

use crate::cli::{Cli, Commands, ConfigCommands, VenvCommands};

pub(crate) fn run() -> ExitCode {
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

    let cli = Cli::parse_from(normalize_cli_args(std::env::args_os()));
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

    emit_report(dispatch_command(&mut ctx, command))
}

fn normalize_cli_args(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let mut normalized = args.into_iter().collect::<Vec<_>>();
    map_windows_help_aliases(&mut normalized);

    if normalized.len() >= 3 {
        match normalized
            .get(1)
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .as_deref()
        {
            Some("help") => {
                if let Some(joined) = join_single_char_tokens(&normalized[2..]) {
                    normalized.truncate(2);
                    normalized.push(OsString::from(joined));
                }
            }
            Some("install") => {
                if let Some(joined) = join_single_char_tokens(&normalized[2..])
                    && matches!(joined.as_str(), "--list" | "-help" | "--help" | "/?")
                {
                    normalized.truncate(2);
                    normalized.push(OsString::from(joined));
                }
            }
            _ => {}
        }
    }

    map_windows_help_aliases(&mut normalized);
    normalized
}

fn map_windows_help_aliases(args: &mut [OsString]) {
    for arg in args {
        if matches!(arg.to_str(), Some("-help") | Some("/?")) {
            *arg = OsString::from("--help");
        }
    }
}

fn join_single_char_tokens(tokens: &[OsString]) -> Option<String> {
    if tokens.is_empty() {
        return None;
    }

    let mut joined = String::new();
    for token in tokens {
        let text = token.to_str()?;
        if text.chars().count() != 1 {
            return None;
        }
        joined.push_str(text);
    }
    Some(joined)
}

fn dispatch_command(ctx: &mut AppContext, command: Commands) -> CommandReport {
    match command {
        Commands::Help { usage, command } => cmd_help(ctx, command.as_deref(), usage),
        Commands::Commands { sh, no_sh } => cmd_commands(ctx, sh, no_sh),
        Commands::Root => cmd_root(ctx),
        Commands::Gui => dispatch_gui(ctx),
        Commands::Hooks { hook } => cmd_hooks(ctx, &hook),
        Commands::Doctor { json, fix, force } => dispatch_doctor(ctx, json, fix, force),
        Commands::SelfUpdate {
            check,
            yes,
            force,
            github_repo,
            tag,
        } => cmd_self_update(
            ctx,
            &SelfUpdateOptions {
                check,
                yes,
                force,
                github_repo,
                tag,
            },
        ),
        Commands::SelfUninstall { yes } => cmd_self_uninstall(ctx, yes),
        Commands::Update {
            check,
            yes,
            force,
            github_repo,
            tag,
        } => cmd_self_update(
            ctx,
            &SelfUpdateOptions {
                check,
                yes,
                force,
                github_repo,
                tag,
            },
        ),
        Commands::Config { command } => dispatch_config(ctx, command),
        Commands::VersionFile { dir } => cmd_version_file(ctx, dir.as_deref()),
        Commands::VersionFileRead { file } => cmd_version_file_read(&file),
        Commands::VersionFileWrite {
            force,
            file,
            versions,
        } => cmd_version_file_write(ctx, &file, &versions, force),
        Commands::VersionOrigin => cmd_version_origin(ctx),
        Commands::VersionName { force } => cmd_version_name(ctx, force),
        Commands::Version { bare } => cmd_version(ctx, bare),
        Commands::Status { json } => cmd_status(ctx, json),
        Commands::Prompt => cmd_prompt(ctx),
        Commands::Global { unset, versions } => cmd_global(ctx, &versions, unset),
        Commands::Local {
            force,
            unset,
            versions,
        } => cmd_local(ctx, &versions, unset, force),
        Commands::Latest {
            known,
            bypass,
            force,
            prefix,
        } => cmd_latest(ctx, &prefix, known, bypass, force),
        Commands::Prefix { versions } => cmd_prefix(ctx, &versions),
        Commands::Init { args } => cmd_init(ctx, &args),
        Commands::Shell { args } => cmd_shell(ctx, &args),
        Commands::Activate { args } => cmd_activate(ctx, &args),
        Commands::Deactivate { args } => cmd_deactivate(ctx, &args),
        Commands::Versions {
            bare,
            skip_aliases,
            skip_envs,
            executables,
        } => cmd_versions(
            ctx,
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
        } => cmd_which(ctx, &command, no_system, skip_advice),
        Commands::Whence { path, command } => cmd_whence(ctx, &command, path),
        Commands::Install {
            list,
            force,
            dry_run,
            json,
            known,
            family,
            versions,
        } => cmd_install(
            ctx,
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
        } => cmd_available(ctx, family, pattern, known, json),
        Commands::Uninstall { force, versions } => cmd_uninstall(ctx, &versions, force),
        Commands::Venv { command } => dispatch_venv(ctx, command),
        Commands::Virtualenv {
            force,
            set_local,
            args,
        } => dispatch_virtualenv(ctx, args, force, set_local),
        Commands::Virtualenvs { bare, json } => cmd_virtualenvs(ctx, bare, json),
        Commands::VirtualenvDelete { force, spec } => cmd_virtualenv_delete(ctx, &spec, force),
        Commands::VirtualenvPrefix { spec } => cmd_virtualenv_prefix(ctx, spec.as_deref()),
        Commands::VirtualenvInit { args } => cmd_virtualenv_init(ctx, &args),
        Commands::Rehash => cmd_rehash(ctx),
        Commands::Shims { short } => cmd_shims(ctx, short),
        Commands::Completions { command, args } => cmd_completions(ctx, &command, &args),
        Commands::ShShell { args } => cmd_sh_shell(ctx, &args),
        Commands::ShActivate { args } => cmd_sh_activate(ctx, &args),
        Commands::ShDeactivate { args } => cmd_sh_deactivate(ctx, &args),
        Commands::ShRehash => cmd_sh_rehash(ctx),
        Commands::ShCmd { args } => cmd_sh_cmd(ctx, &args),
        Commands::Exec { command, args } => cmd_exec(ctx, &command, &args),
        Commands::External(args) => cmd_external(ctx, &args),
    }
}

fn dispatch_gui(_ctx: &AppContext) -> CommandReport {
    use std::path::PathBuf;
    use std::process::Command;

    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("pyenv"));
    let mut gui_exe = exe.clone();
    gui_exe.set_file_name(if cfg!(windows) {
        "pyenv-gui.exe"
    } else {
        "pyenv-gui"
    });

    if !gui_exe.exists() {
        return CommandReport::failure(
            vec![
                "pyenv: GUI companion not found.".to_string(),
                format!("Expected: {}", gui_exe.display()),
                "Please reinstall pyenv-native to acquire the GUI binary.".to_string(),
            ],
            1,
        );
    }

    let mut child = Command::new(&gui_exe);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        child.creation_flags(0x00000008); // DETACHED_PROCESS
    }

    match child.spawn() {
        Ok(_) => CommandReport::success(vec!["Launching pyenv-gui...".to_string()]),
        Err(e) => CommandReport::failure(vec![format!("pyenv: failed to launch GUI: {e}")], 1),
    }
}

fn dispatch_doctor(ctx: &AppContext, json: bool, fix: bool, force: bool) -> CommandReport {
    if fix {
        if json {
            CommandReport::failure(
                vec!["pyenv: `doctor --json` cannot be combined with `--fix`".to_string()],
                1,
            )
        } else {
            run_doctor_fix_flow(ctx, force)
        }
    } else {
        cmd_doctor(ctx, json)
    }
}

fn dispatch_config(ctx: &mut AppContext, command: Option<ConfigCommands>) -> CommandReport {
    match command.unwrap_or(ConfigCommands::Show) {
        ConfigCommands::Path => cmd_config_path(ctx),
        ConfigCommands::Show => cmd_config_show(ctx),
        ConfigCommands::Get { key } => cmd_config_get(ctx, &key),
        ConfigCommands::Set { key, value } => cmd_config_set(ctx, &key, &value),
    }
}

fn dispatch_venv(ctx: &AppContext, command: VenvCommands) -> CommandReport {
    match command {
        VenvCommands::List { bare, json } => cmd_venv_list(ctx, bare, json),
        VenvCommands::Info { json, spec } => cmd_venv_info(ctx, &spec, json),
        VenvCommands::Create {
            force,
            set_local,
            version,
            name,
        } => cmd_venv_create(ctx, &version, &name, force, set_local),
        VenvCommands::Delete { force, spec } => cmd_venv_delete(ctx, &spec, force),
        VenvCommands::Rename { spec, new_name } => cmd_venv_rename(ctx, &spec, &new_name),
        VenvCommands::Use { global, spec } => cmd_venv_use(
            ctx,
            &spec,
            if global {
                VenvUseScope::Global
            } else {
                VenvUseScope::Local
            },
        ),
    }
}

fn dispatch_virtualenv(
    ctx: &AppContext,
    args: Vec<String>,
    force: bool,
    set_local: bool,
) -> CommandReport {
    match args.as_slice() {
        [name] => cmd_virtualenv(ctx, None, name, force, set_local),
        [version, name] => cmd_virtualenv(ctx, Some(version), name, force, set_local),
        _ => CommandReport::failure(
            vec![
                "pyenv: `virtualenv` expects `<name>` or `<runtime> <name>`".to_string(),
                "hint: try `pyenv virtualenv 3.13 api` or `pyenv virtualenv api` when a runtime is already selected"
                    .to_string(),
            ],
            1,
        ),
    }
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

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::normalize_cli_args;

    #[test]
    fn normalize_cli_args_maps_windows_help_spellings() {
        let args = vec![
            OsString::from("pyenv"),
            OsString::from("install"),
            OsString::from("-help"),
            OsString::from("/?"),
        ];

        let normalized = normalize_cli_args(args);
        assert_eq!(
            normalized,
            vec![
                OsString::from("pyenv"),
                OsString::from("install"),
                OsString::from("--help"),
                OsString::from("--help"),
            ]
        );
    }

    #[test]
    fn normalize_cli_args_repairs_split_help_command_name() {
        let args = vec![
            OsString::from("pyenv"),
            OsString::from("help"),
            OsString::from("i"),
            OsString::from("n"),
            OsString::from("s"),
            OsString::from("t"),
            OsString::from("a"),
            OsString::from("l"),
            OsString::from("l"),
        ];

        let normalized = normalize_cli_args(args);
        assert_eq!(
            normalized,
            vec![
                OsString::from("pyenv"),
                OsString::from("help"),
                OsString::from("install"),
            ]
        );
    }

    #[test]
    fn normalize_cli_args_repairs_split_install_help_flags() {
        let args = vec![
            OsString::from("pyenv"),
            OsString::from("install"),
            OsString::from("-"),
            OsString::from("-"),
            OsString::from("l"),
            OsString::from("i"),
            OsString::from("s"),
            OsString::from("t"),
        ];

        let normalized = normalize_cli_args(args);
        assert_eq!(
            normalized,
            vec![
                OsString::from("pyenv"),
                OsString::from("install"),
                OsString::from("--list"),
            ]
        );
    }
}
