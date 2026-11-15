// ./crates/enchi-cli/src/dispatch.rs
//! CLI dispatch and terminal interaction for the `enchi` binary. This module parses the
//! command context, routes clap subcommands into `enchi-core`, and handles prompt/report I/O.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches};
use enchi_core::{
    AgentSkillsOptions, AppContext, BulkScope, BulkUpdateOptions, CommandReport, DoctorFix,
    InstallCommandOptions, PowerShell7Mode, Scope as AgentScope, SelfInstallOptions,
    SelfUninstallOptions, SelfUpdateOptions, ShellKind, UpgradeOptions, VenvUseScope,
    VersionsCommandOptions, apply_doctor_fixes, cmd_activate, cmd_agent_skills, cmd_available,
    cmd_commands, cmd_completions, cmd_config_get, cmd_config_path, cmd_config_set,
    cmd_config_show, cmd_deactivate, cmd_doctor, cmd_environment, cmd_exec, cmd_external,
    cmd_global, cmd_help, cmd_hooks, cmd_init, cmd_install, cmd_latest, cmd_local,
    cmd_pip_analyze_imports, cmd_pip_check, cmd_pip_install, cmd_pip_list, cmd_pip_outdated,
    cmd_pip_precheck_requirements, cmd_pip_update, cmd_pip_update_all, cmd_prefix, cmd_preflight,
    cmd_prompt, cmd_rehash, cmd_root, cmd_self_uninstall_with, cmd_self_update, cmd_sh_activate,
    cmd_sh_cmd, cmd_sh_deactivate, cmd_sh_rehash, cmd_sh_shell, cmd_shell, cmd_shims, cmd_status,
    cmd_uninstall, cmd_upgrade, cmd_venv_create, cmd_venv_delete, cmd_venv_info, cmd_venv_list,
    cmd_venv_rename, cmd_venv_upgrade, cmd_venv_use, cmd_version, cmd_version_file,
    cmd_version_file_read, cmd_version_file_write, cmd_version_name, cmd_version_origin,
    cmd_versions, cmd_virtualenv, cmd_virtualenv_delete, cmd_virtualenv_init,
    cmd_virtualenv_prefix, cmd_virtualenvs, cmd_whence, cmd_which, doctor_fix_plan, parse_agents,
    plan_pip_install, plan_pip_update, plan_uninstall, plan_venv_create, plan_venv_delete,
    plan_venv_rename, plan_venv_upgrade,
};

use crate::cli::{Cli, Commands, ConfigCommands, PipCommands, VenvCommands};

pub(crate) fn run() -> ExitCode {
    enchi_core::init_i18n();
    if let Ok(ctx) = AppContext::from_system() {
        enchi_core::apply_i18n(&ctx.config);
    }

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

    let cli = parse_localized_cli();
    let Some(command) = cli.command else {
        let _ = localize_clap(Cli::command()).print_help();
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

fn parse_localized_cli() -> Cli {
    let args = normalize_cli_args(std::env::args_os());
    let cmd = localize_clap(Cli::command());
    match cmd.try_get_matches_from(&args) {
        Ok(matches) => Cli::from_arg_matches(&matches).unwrap_or_else(|err| err.exit()),
        Err(err) => err.exit(),
    }
}

fn localize_clap(mut cmd: clap::Command) -> clap::Command {
    cmd = cmd
        .about(enchi_core::lookup_i18n("cli-about"))
        .after_help(enchi_core::lookup_i18n("cli-help-concepts"))
        .help_template(localized_help_template());
    localize_command_tree(&mut cmd, None);
    cmd
}

fn lookup_about(parent: Option<&str>, name: &str) -> Option<String> {
    let candidates = match parent {
        Some(parent_name) => vec![
            format!("cli-{parent_name}-{name}-about"),
            format!("cli-{name}-about"),
        ],
        None => vec![format!("cli-{name}-about")],
    };
    for id in candidates {
        let about = enchi_core::lookup_i18n(&id);
        if about != id {
            return Some(about);
        }
    }
    None
}

fn localize_command_tree(cmd: &mut clap::Command, parent: Option<&str>) {
    let name = cmd.get_name().to_string();
    // Root already uses `cli-about`. Do not probe `cli-pyenv-about`.
    if parent.is_some()
        && let Some(about) = lookup_about(parent, &name)
    {
        *cmd = cmd.clone().about(about);
    }
    let children: Vec<String> = cmd
        .get_subcommands()
        .map(|child| child.get_name().to_string())
        .collect();
    for child_name in children {
        if let Some(child) = cmd.find_subcommand_mut(&child_name) {
            localize_command_tree(child, Some(&name));
        }
    }
}

fn help_row(name: &str, about_id: &str) -> String {
    format!("  {name:<18} {}", enchi_core::lookup_i18n(about_id))
}

fn help_section(heading_id: &str, rows: &[(&str, &str)]) -> String {
    let mut lines = vec![format!("{}:", enchi_core::lookup_i18n(heading_id))];
    lines.extend(rows.iter().map(|(name, about_id)| help_row(name, about_id)));
    lines.join("\n")
}

fn localized_help_template() -> String {
    let body = [
        help_section(
            "cli-help-selection",
            &[
                ("global", "cli-global-about"),
                ("local", "cli-local-about"),
                ("shell", "cli-shell-about"),
                ("latest", "cli-latest-about"),
                ("version", "cli-version-about"),
                ("version-name", "cli-version-name-about"),
                ("version-origin", "cli-version-origin-about"),
                ("prefix", "cli-prefix-about"),
            ],
        ),
        help_section(
            "cli-help-provisioning",
            &[
                ("install", "cli-install-about"),
                ("available", "cli-available-about"),
                ("versions", "cli-versions-about"),
                ("upgrade", "cli-upgrade-about"),
                ("uninstall", "cli-uninstall-about"),
            ],
        ),
        help_section(
            "cli-help-environment",
            &[("venv", "cli-venv-about"), ("pip", "cli-pip-about")],
        ),
        help_section(
            "cli-help-interface",
            &[
                ("init", "cli-init-about"),
                ("gui", "cli-gui-about"),
                ("rehash", "cli-rehash-about"),
                ("shims", "cli-shims-about"),
                ("prompt", "cli-prompt-about"),
                ("exec", "cli-exec-about"),
                ("completions", "cli-completions-about"),
            ],
        ),
        help_section(
            "cli-help-diagnostics",
            &[
                ("doctor", "cli-doctor-about"),
                ("preflight", "cli-preflight-about"),
                ("environment", "cli-environment-about"),
                ("status", "cli-status-about"),
                ("config", "cli-config-about"),
                ("root", "cli-root-about"),
                ("which", "cli-which-about"),
                ("whence", "cli-whence-about"),
                ("version-file", "cli-version-file-about"),
                ("version-file-read", "cli-version-file-read-about"),
            ],
        ),
        help_section(
            "cli-help-maintenance",
            &[
                ("self-install", "cli-self-install-about"),
                ("self-update", "cli-self-update-about"),
                ("self-uninstall", "cli-self-uninstall-about"),
                ("agent-skills", "cli-agent-skills-about"),
            ],
        ),
        help_section(
            "cli-help-support",
            &[
                ("help", "cli-help-about"),
                ("commands", "cli-commands-about"),
                ("hooks", "cli-hooks-about"),
            ],
        ),
    ]
    .join("\n\n");
    format!(
        "\
{{before-help}}{{name}} {{version}}
{{about-with-newline}}
{{usage-heading}} {{usage}}

{body}

{{options}}

{{after-help}}
"
    )
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

/// Run a command, rendering the result as JSON when the command was given `--json`.
fn dispatch_command(ctx: &mut AppContext, command: Commands) -> CommandReport {
    let Some(mode) = json_mode(&command) else {
        return run_command(ctx, command);
    };
    if let Some(refusal) = json_guard(&command) {
        return refusal.into_json(mode.command, mode.dry_run, None);
    }
    let report = run_command(ctx, command);
    let data = envelope_data(mode.command, &report);
    report.into_json(mode.command, mode.dry_run, data)
}

/// How a `--json` command is reported: its name in the envelope and whether it was a dry run.
struct JsonMode {
    command: &'static str,
    dry_run: bool,
}

impl JsonMode {
    fn new(command: &'static str, dry_run: bool) -> Option<Self> {
        Some(Self { command, dry_run })
    }
}

/// `Some` when the command asked for `--json` and has no typed JSON of its own, so its text
/// result is wrapped in the `enchi.result/1` envelope. Commands with native JSON (`status`,
/// `venv list`, `pip outdated`, ...) print their own documents and are not listed here.
fn json_mode(command: &Commands) -> Option<JsonMode> {
    match command {
        Commands::Global { json: true, .. } => JsonMode::new("global", false),
        Commands::Local { json: true, .. } => JsonMode::new("local", false),
        Commands::Latest { json: true, .. } => JsonMode::new("latest", false),
        Commands::Version { json: true, .. } => JsonMode::new("version", false),
        Commands::Prefix { json: true, .. } => JsonMode::new("prefix", false),
        Commands::Uninstall {
            json: true,
            dry_run,
            ..
        } => JsonMode::new("uninstall", *dry_run),
        Commands::Rehash { json: true } => JsonMode::new("rehash", false),
        Commands::Shims { json: true, .. } => JsonMode::new("shims", false),
        Commands::Root { json: true } => JsonMode::new("root", false),
        Commands::Which { json: true, .. } => JsonMode::new("which", false),
        Commands::SelfUpdate { json: true, .. } | Commands::Update { json: true, .. } => {
            JsonMode::new("self-update", false)
        }
        Commands::SelfUninstall { json: true, .. } => JsonMode::new("self-uninstall", false),
        Commands::AgentSkills { json: true, .. } => JsonMode::new("agent-skills", false),
        Commands::Config {
            command: Some(command),
        } => match command {
            ConfigCommands::Path { json: true } => JsonMode::new("config path", false),
            ConfigCommands::Show { json: true } => JsonMode::new("config show", false),
            ConfigCommands::Get { json: true, .. } => JsonMode::new("config get", false),
            ConfigCommands::Set { json: true, .. } => JsonMode::new("config set", false),
            _ => None,
        },
        Commands::Venv { command } => match command {
            VenvCommands::Create {
                json: true,
                dry_run,
                ..
            } => JsonMode::new("venv create", *dry_run),
            VenvCommands::Delete {
                json: true,
                dry_run,
                ..
            } => JsonMode::new("venv delete", *dry_run),
            VenvCommands::Rename {
                json: true,
                dry_run,
                ..
            } => JsonMode::new("venv rename", *dry_run),
            VenvCommands::Use { json: true, .. } => JsonMode::new("venv use", false),
            VenvCommands::Upgrade {
                json: true,
                dry_run,
                ..
            } => JsonMode::new("venv upgrade", *dry_run),
            _ => None,
        },
        Commands::Pip { command } => match command {
            PipCommands::Precheck { json: true, .. } => JsonMode::new("pip precheck", false),
            PipCommands::Analyze { json: true, .. } => JsonMode::new("pip analyze", false),
            PipCommands::Install {
                json: true,
                dry_run,
                ..
            } => JsonMode::new("pip install", *dry_run),
            PipCommands::Update {
                json: true,
                dry_run,
                ..
            } => JsonMode::new("pip update", *dry_run),
            _ => None,
        },
        _ => None,
    }
}

/// Machine mode never prompts. A destructive command without `--force`/`--yes` (and not a dry
/// run) is refused with a message saying how to confirm, instead of blocking on stdin.
fn json_guard(command: &Commands) -> Option<CommandReport> {
    let missing = |flag: &str, what: &str| {
        Some(CommandReport::failure(
            vec![format!(
                "enchi: `{what} --json` never prompts; pass {flag} to confirm (or --dry-run to preview)"
            )],
            1,
        ))
    };
    match command {
        Commands::Uninstall {
            json: true,
            force: false,
            dry_run: false,
            ..
        } => missing("--force", "uninstall"),
        Commands::SelfUpdate {
            json: true,
            yes: false,
            check: false,
            ..
        }
        | Commands::Update {
            json: true,
            yes: false,
            check: false,
            ..
        } => missing("--yes", "self-update"),
        Commands::SelfUninstall {
            json: true,
            yes: false,
            ..
        } => missing("--yes", "self-uninstall"),
        Commands::Venv { command } => match command {
            VenvCommands::Delete {
                json: true,
                force: false,
                dry_run: false,
                ..
            } => missing("--force", "venv delete"),
            VenvCommands::Upgrade {
                json: true,
                force: false,
                dry_run: false,
                ..
            } => missing("--force", "venv upgrade"),
            _ => None,
        },
        _ => None,
    }
}

/// Typed `data` for the envelope of commands whose result is a simple value or list.
fn envelope_data(command: &str, report: &CommandReport) -> Option<serde_json::Value> {
    if report.exit_code != 0 || report.stdout.is_empty() {
        return None;
    }
    let first = report.stdout[0].clone();
    match command {
        "root" => Some(serde_json::json!({ "path": first })),
        "which" => Some(serde_json::json!({ "path": first })),
        "config path" => Some(serde_json::json!({ "path": first })),
        "config get" => Some(serde_json::json!({ "value": first })),
        "latest" => Some(serde_json::json!({ "version": first })),
        "prefix" => {
            let paths: Vec<String> = std::env::split_paths(&first)
                .map(|path| path.display().to_string())
                .collect();
            Some(serde_json::json!({ "paths": paths }))
        }
        "global" | "local" => Some(serde_json::json!({ "versions": report.stdout })),
        "shims" => Some(serde_json::json!({ "shims": report.stdout })),
        _ => None,
    }
}

fn run_command(ctx: &mut AppContext, command: Commands) -> CommandReport {
    match command {
        Commands::ManualHelp { usage, command } => cmd_help(ctx, command.as_deref(), usage),
        Commands::Commands { sh, no_sh } => cmd_commands(ctx, sh, no_sh),
        Commands::Root { .. } => cmd_root(ctx),
        Commands::Gui => dispatch_gui(ctx),
        Commands::Hooks { hook } => cmd_hooks(ctx, &hook),
        Commands::Doctor { json, fix, force } => dispatch_doctor(ctx, json, fix, force),
        Commands::Preflight { json } => cmd_preflight(ctx, json),
        Commands::Environment { json } => cmd_environment(ctx, json),
        Commands::SelfUpdate {
            check,
            yes,
            force,
            github_repo,
            tag,
            ..
        } => cmd_self_update(
            ctx,
            &SelfUpdateOptions {
                check,
                yes,
                force,
                github_repo,
                tag,
                restart_gui: false,
            },
        ),
        Commands::SelfInstall {
            install_root,
            source_dir,
            source_path,
            source_mcp_path,
            source_gui_path,
            shell,
            add_to_user_path,
            update_shell_profile,
            refresh_shims,
            install_powershell7,
            log_path,
            lang,
            force,
            yes,
            legacy_aliases,
            wait_for_pid,
            relaunch_gui,
        } => dispatch_self_install(
            ctx,
            SelfInstallArgs {
                install_root,
                source_dir,
                source_path,
                source_mcp_path,
                source_gui_path,
                shell,
                add_to_user_path,
                update_shell_profile,
                refresh_shims,
                install_powershell7,
                log_path,
                lang,
                force,
                yes,
                legacy_aliases,
                wait_for_pid,
                relaunch_gui,
            },
        ),
        Commands::SelfUninstall {
            yes,
            keep_data,
            keep_shell_integration,
            system,
            finish_after_pid,
            ..
        } => {
            let mut options = SelfUninstallOptions::new(ctx.root.clone());
            options.system = system;
            options.yes = yes;
            options.keep_data = keep_data;
            options.keep_shell_integration = keep_shell_integration;
            options.finish_after_pid = finish_after_pid;
            cmd_self_uninstall_with(&options)
        }
        Commands::AgentSkills {
            agent,
            scope,
            repo_url,
            ..
        } => dispatch_agent_skills(&agent, &scope, repo_url),
        Commands::Update {
            check,
            yes,
            force,
            github_repo,
            tag,
            ..
        } => cmd_self_update(
            ctx,
            &SelfUpdateOptions {
                check,
                yes,
                force,
                github_repo,
                tag,
                restart_gui: false,
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
        Commands::Version { bare, .. } => cmd_version(ctx, bare),
        Commands::Status { json } => cmd_status(ctx, json),
        Commands::Prompt => cmd_prompt(ctx),
        Commands::Global {
            unset, versions, ..
        } => cmd_global(ctx, &versions, unset),
        Commands::Local {
            force,
            unset,
            versions,
            ..
        } => cmd_local(ctx, &versions, unset, force),
        Commands::Latest {
            known,
            bypass,
            force,
            prefix,
            ..
        } => cmd_latest(ctx, &prefix, known, bypass, force),
        Commands::Prefix { versions, .. } => cmd_prefix(ctx, &versions),
        Commands::Init { args } => cmd_init(ctx, &args),
        Commands::Shell { args } => cmd_shell(ctx, &args),
        Commands::Activate { args } => cmd_activate(ctx, &args),
        Commands::Deactivate { args } => cmd_deactivate(ctx, &args),
        Commands::Versions {
            bare,
            skip_aliases,
            skip_envs,
            executables,
            json,
        } => cmd_versions(
            ctx,
            &VersionsCommandOptions {
                bare,
                skip_aliases,
                skip_envs,
                executables,
                json,
            },
        ),
        Commands::Which {
            no_system,
            skip_advice,
            command,
            ..
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
        Commands::Upgrade {
            check,
            no_venvs,
            no_repin,
            remove_old,
            json,
            targets,
        } => {
            let options = UpgradeOptions {
                targets,
                dry_run: check,
                skip_venvs: no_venvs,
                skip_repin: no_repin,
                remove_old,
            };
            cmd_upgrade(ctx, &options, json, &mut |line| eprintln!("{line}"))
        }
        Commands::Uninstall {
            force,
            dry_run,
            versions,
            ..
        } => {
            if dry_run {
                plan_uninstall(ctx, &versions)
            } else {
                cmd_uninstall(ctx, &versions, force)
            }
        }
        Commands::Venv { command } => dispatch_venv(ctx, command),
        Commands::Pip { command } => dispatch_pip(ctx, command),
        Commands::Virtualenv {
            force,
            set_local,
            args,
        } => dispatch_virtualenv(ctx, args, force, set_local),
        Commands::Virtualenvs { bare, json } => cmd_virtualenvs(ctx, bare, json),
        Commands::VirtualenvDelete { force, spec } => cmd_virtualenv_delete(ctx, &spec, force),
        Commands::VirtualenvPrefix { spec } => cmd_virtualenv_prefix(ctx, spec.as_deref()),
        Commands::VirtualenvInit { args } => cmd_virtualenv_init(ctx, &args),
        Commands::Rehash { .. } => cmd_rehash(ctx),
        Commands::Shims { short, .. } => cmd_shims(ctx, short),
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

fn dispatch_agent_skills(agent: &str, scope: &str, repo_url: Option<String>) -> CommandReport {
    let resolved = (|| {
        let mut options = AgentSkillsOptions::from_environment()?;
        options.agents = parse_agents(agent)?;
        options.scope = AgentScope::parse(scope)?;
        if let Some(url) = repo_url {
            options.repo_url = url;
        }
        Ok::<_, String>(options)
    })();
    match resolved {
        Ok(options) => cmd_agent_skills(&options),
        Err(message) => CommandReport::failure(vec![message], 1),
    }
}

fn dispatch_gui(_ctx: &AppContext) -> CommandReport {
    use std::path::PathBuf;
    use std::process::Command;

    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from(enchi_core::brand::CLI));
    let mut gui_exe = exe.clone();
    gui_exe.set_file_name(if cfg!(windows) {
        "enchi-gui.exe"
    } else {
        "enchi-gui"
    });

    if !gui_exe.exists() {
        return CommandReport::failure(
            vec![
                "enchi: GUI companion not found.".to_string(),
                format!("Expected: {}", gui_exe.display()),
                "Please reinstall Enchi to acquire the GUI binary.".to_string(),
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
        Ok(_) => CommandReport::success(vec!["Launching enchi-gui...".to_string()]),
        Err(e) => CommandReport::failure(vec![format!("enchi: failed to launch GUI: {e}")], 1),
    }
}

fn dispatch_doctor(ctx: &AppContext, json: bool, fix: bool, force: bool) -> CommandReport {
    if fix {
        if json {
            CommandReport::failure(
                vec!["enchi: `doctor --json` cannot be combined with `--fix`".to_string()],
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
    match command.unwrap_or(ConfigCommands::Show { json: false }) {
        ConfigCommands::Path { .. } => cmd_config_path(ctx),
        ConfigCommands::Show { .. } => cmd_config_show(ctx),
        ConfigCommands::Get { key, .. } => cmd_config_get(ctx, &key),
        ConfigCommands::Set { key, value, .. } => cmd_config_set(ctx, &key, &value),
    }
}

fn dispatch_venv(ctx: &AppContext, command: VenvCommands) -> CommandReport {
    match command {
        VenvCommands::List { bare, json } => cmd_venv_list(ctx, bare, json),
        VenvCommands::Info { json, spec } => cmd_venv_info(ctx, &spec, json),
        VenvCommands::Create {
            force,
            set_local,
            dry_run,
            version,
            name,
            ..
        } => {
            if dry_run {
                plan_venv_create(ctx, &version, &name, force)
            } else {
                cmd_venv_create(ctx, &version, &name, force, set_local)
            }
        }
        VenvCommands::Delete {
            force,
            dry_run,
            spec,
            ..
        } => {
            if dry_run {
                plan_venv_delete(ctx, &spec)
            } else {
                cmd_venv_delete(ctx, &spec, force)
            }
        }
        VenvCommands::Rename {
            dry_run,
            spec,
            new_name,
            ..
        } => {
            if dry_run {
                plan_venv_rename(ctx, &spec, &new_name)
            } else {
                cmd_venv_rename(ctx, &spec, &new_name)
            }
        }
        VenvCommands::Use { global, spec, .. } => cmd_venv_use(
            ctx,
            &spec,
            if global {
                VenvUseScope::Global
            } else {
                VenvUseScope::Local
            },
        ),
        VenvCommands::Upgrade {
            force,
            set_local,
            dry_run,
            spec,
            new_runtime,
            ..
        } => {
            if dry_run {
                plan_venv_upgrade(ctx, &spec, &new_runtime, force)
            } else {
                cmd_venv_upgrade(ctx, &spec, &new_runtime, force, set_local)
            }
        }
    }
}

fn dispatch_pip(ctx: &AppContext, command: PipCommands) -> CommandReport {
    match command {
        PipCommands::List { json, target } => cmd_pip_list(ctx, &target, json),
        PipCommands::Outdated { json, target } => cmd_pip_outdated(ctx, &target, json),
        PipCommands::Check { json, target } => cmd_pip_check(ctx, &target, json),
        PipCommands::Precheck {
            requirement,
            target,
            ..
        } => cmd_pip_precheck_requirements(ctx, &target, &requirement),
        PipCommands::Analyze { target, dir, .. } => {
            let dir = dir.unwrap_or_else(|| ctx.dir.display().to_string());
            cmd_pip_analyze_imports(ctx, &target, &dir)
        }
        PipCommands::Install {
            requirement,
            target,
            dry_run,
            ..
        } => {
            if dry_run {
                plan_pip_install(ctx, &target, &requirement)
            } else {
                cmd_pip_install(ctx, &target, &requirement)
            }
        }
        PipCommands::UpdateAll {
            dry_run,
            runtimes,
            venvs,
            allow_conflicts,
            json,
            only,
        } => {
            let options = BulkUpdateOptions {
                scope: match (runtimes, venvs) {
                    (true, _) => BulkScope::Runtimes,
                    (_, true) => BulkScope::Venvs,
                    _ => BulkScope::All,
                },
                only,
                dry_run,
                allow_new_conflicts: allow_conflicts,
            };
            cmd_pip_update_all(ctx, &options, json, &mut |line| eprintln!("{line}"))
        }
        PipCommands::Update {
            all,
            target,
            packages,
            dry_run,
            ..
        } => {
            if dry_run {
                plan_pip_update(ctx, &target, &packages, all)
            } else {
                cmd_pip_update(ctx, &target, &packages, all)
            }
        }
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
                "enchi: `virtualenv` expects `<name>` or `<runtime> <name>`".to_string(),
                "hint: try `enchi virtualenv 3.13 api` or `enchi virtualenv api` when a runtime is already selected"
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
        return CommandReport::failure(vec!["enchi: doctor fixes cancelled".to_string()], 1);
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
    if enchi_core::brand::is_cli_name(&lowered)
        || enchi_core::brand::is_companion_name(&lowered)
        || matches!(lowered.as_str(), "cargo" | "rustc")
    {
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

/// Parsed `self-install` arguments, bundled so the dispatch arm stays readable.
struct SelfInstallArgs {
    install_root: Option<std::path::PathBuf>,
    source_dir: Option<std::path::PathBuf>,
    source_path: Option<std::path::PathBuf>,
    source_mcp_path: Option<std::path::PathBuf>,
    source_gui_path: Option<std::path::PathBuf>,
    shell: Option<String>,
    add_to_user_path: bool,
    update_shell_profile: bool,
    refresh_shims: bool,
    install_powershell7: String,
    log_path: Option<std::path::PathBuf>,
    lang: Option<String>,
    force: bool,
    yes: bool,
    legacy_aliases: String,
    wait_for_pid: Option<u32>,
    relaunch_gui: bool,
}

/// Run `self-install`, printing progress as it happens rather than after the fact.
fn dispatch_self_install(ctx: &AppContext, args: SelfInstallArgs) -> CommandReport {
    if let Some(lang) = args.lang.as_deref() {
        enchi_core::set_lang_tag(lang);
    }
    let shell = match args.shell.as_deref() {
        None => ShellKind::detect(),
        Some(name) => match ShellKind::parse(name) {
            Some(kind) => kind,
            None => {
                return CommandReport::failure(
                    vec![format!(
                        "enchi: unsupported shell `{name}`; use bash, zsh, fish, sh, pwsh, cmd, or none"
                    )],
                    1,
                );
            }
        },
    };
    let Some(powershell7) = PowerShell7Mode::parse(&args.install_powershell7) else {
        return CommandReport::failure(
            vec![format!(
                "enchi: invalid --install-powershell7 value `{}`; use auto, true, or false",
                args.install_powershell7
            )],
            1,
        );
    };

    let Some(legacy_aliases) = enchi_core::LegacyAliases::parse(&args.legacy_aliases) else {
        return CommandReport::failure(
            vec![format!(
                "enchi: invalid --legacy-aliases value `{}`; use auto, always, or never",
                args.legacy_aliases
            )],
            1,
        );
    };

    let mut options =
        SelfInstallOptions::new(args.install_root.unwrap_or_else(|| ctx.root.clone()));
    options.legacy_aliases = legacy_aliases;
    options.source_dir = args.source_dir;
    options.source_cli = args.source_path;
    options.source_mcp = args.source_mcp_path;
    options.source_gui = args.source_gui_path;
    options.shell = shell;
    options.add_to_user_path = args.add_to_user_path;
    options.update_shell_profile = args.update_shell_profile;
    options.refresh_shims = args.refresh_shims;
    options.install_powershell7 = powershell7;
    options.log_path = args.log_path;
    options.force = args.force;
    options.yes = args.yes;
    options.wait_for_pid = args.wait_for_pid;
    options.relaunch_gui = args.relaunch_gui;

    let mut sink = |line: &str| {
        let mut stdout = io::stdout().lock();
        let _ = writeln!(stdout, "{line}");
        let _ = stdout.flush();
    };
    match enchi_core::run_self_install(&options, &mut sink) {
        Ok(_) => CommandReport::empty_success(),
        Err(message) => CommandReport::failure(vec![message], 1),
    }
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
