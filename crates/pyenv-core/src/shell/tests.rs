// ./crates/pyenv-core/src/shell/tests.rs
//! Regression coverage for shell init, shell-scoped version selection, and managed-venv
//! activation compatibility output.

use std::ffi::OsString;
use std::fs;

use tempfile::TempDir;

use crate::config::AppConfig;
use crate::context::AppContext;
use crate::venv_paths::managed_venv_dir;

use super::{
    cmd_activate, cmd_deactivate, cmd_init, cmd_sh_activate, cmd_sh_cmd, cmd_sh_deactivate,
    cmd_sh_rehash, cmd_sh_shell, cmd_shell, cmd_virtualenv_init,
};

fn test_context() -> (TempDir, AppContext) {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path().join(".pyenv");
    let dir = temp.path().join("work");
    fs::create_dir_all(root.join("versions")).expect("versions dir");
    fs::create_dir_all(&dir).expect("work dir");

    let ctx = AppContext {
        root,
        dir,
        exe_path: std::path::PathBuf::from("pyenv"),
        env_version: None,
        env_shell: Some("pwsh".to_string()),
        path_env: Some(OsString::from("C:\\Windows\\System32")),
        path_ext: Some(OsString::from(".EXE;.CMD;.BAT")),
        config: AppConfig::default(),
    };

    (temp, ctx)
}

fn seed_managed_venv(ctx: &AppContext, base_version: &str, name: &str) {
    let version_dir = ctx.versions_dir().join(base_version);
    fs::create_dir_all(&version_dir).expect("version dir");
    let venv_dir = managed_venv_dir(ctx, base_version, name);
    let bin_dir = if cfg!(windows) {
        venv_dir.join("Scripts")
    } else {
        venv_dir.join("bin")
    };
    fs::create_dir_all(&bin_dir).expect("bin dir");
    let python = if cfg!(windows) {
        venv_dir.join("python.exe")
    } else {
        bin_dir.join("python")
    };
    if let Some(parent) = python.parent() {
        fs::create_dir_all(parent).expect("python parent");
    }
    fs::write(&python, "").expect("python");
}

#[test]
fn shell_command_requires_integration() {
    let (_temp, ctx) = test_context();
    let report = cmd_shell(&ctx, &[]);
    assert_eq!(report.exit_code, 1);
    assert!(report.stderr[0].contains("shell integration not enabled"));
}

#[test]
fn activate_and_deactivate_require_integration_without_shell_wrapper() {
    let (_temp, ctx) = test_context();
    let activate = cmd_activate(&ctx, &[String::from("demo")]);
    assert_eq!(activate.exit_code, 1);
    assert!(activate.stderr[0].contains("shell integration not enabled"));

    let deactivate = cmd_deactivate(&ctx, &[]);
    assert_eq!(deactivate.exit_code, 1);
    assert!(deactivate.stderr[0].contains("shell integration not enabled"));
}

#[test]
fn sh_shell_reports_missing_shell_version() {
    let (_temp, mut ctx) = test_context();
    ctx.env_version = None;
    let report = cmd_sh_shell(&ctx, &[]);
    assert_eq!(report.exit_code, 1);
    assert!(report.stderr[0].contains("no shell-specific version"));
}

#[test]
fn sh_shell_sets_requested_version_for_pwsh() {
    let (_temp, ctx) = test_context();
    fs::create_dir_all(ctx.versions_dir().join("3.12.6")).expect("version");
    let report = cmd_sh_shell(&ctx, &[String::from("3.12")]);
    assert_eq!(report.exit_code, 0);
    assert_eq!(
        report.stdout,
        vec![
            "$Env:PYENV_VERSION_OLD = $Env:PYENV_VERSION".to_string(),
            "$Env:PYENV_VERSION = \"3.12\"".to_string()
        ]
    );
}

#[test]
fn sh_shell_unset_and_rehash_use_pwsh_syntax() {
    let (_temp, ctx) = test_context();
    let unset_report = cmd_sh_shell(&ctx, &[String::from("--unset")]);
    assert_eq!(unset_report.exit_code, 0);
    assert!(unset_report.stdout[0].contains("PYENV_VERSION_OLD"));

    let rehash_report = cmd_sh_rehash(&ctx);
    assert_eq!(rehash_report.exit_code, 0);
    assert!(rehash_report.stdout[0].contains("& 'pyenv' rehash"));
}

#[test]
fn sh_activate_emits_virtualenv_environment_updates() {
    let (_temp, ctx) = test_context();
    seed_managed_venv(&ctx, "3.12.6", "demo");

    let report = cmd_sh_activate(&ctx, &[String::from("demo")]);
    assert_eq!(report.exit_code, 0);
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("$Env:PYENV_VERSION = \"3.12.6/envs/demo\""))
    );
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("$Env:VIRTUAL_ENV ="))
    );
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("$Env:_PYENV_VIRTUAL_PATH_OLD = $Env:PATH"))
    );
}

#[test]
fn sh_deactivate_emits_restore_commands() {
    let (_temp, ctx) = test_context();
    let report = cmd_sh_deactivate(&ctx, &[]);
    assert_eq!(report.exit_code, 0);
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("_PYENV_VIRTUAL_PATH_OLD"))
    );
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("PYENV_VERSION_OLD"))
    );
}

#[test]
fn init_print_for_pwsh_sets_path_env_and_function() {
    let (_temp, ctx) = test_context();
    let report = cmd_init(&ctx, &[String::from("-"), String::from("pwsh")]);
    assert_eq!(report.exit_code, 0);
    assert!(ctx.shims_dir().is_dir());
    assert!(ctx.versions_dir().is_dir());
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("$Env:PYENV_SHELL=\"pwsh\""))
    );
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("function pyenv"))
    );
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("function Invoke-PyenvPassthrough"))
    );
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("ArgumentList.Add"))
    );
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("Join-PyenvWindowsArguments"))
    );
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("$psi.Arguments = Join-PyenvWindowsArguments"))
    );
    assert!(report.stdout.iter().any(|line| {
        line.contains("Invoke-PyenvPassthrough $pyenvExe (@([string]$command) + $arguments)")
    }));
    assert!(report.stdout.iter().any(|line| line.contains("sh-shell")));
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("sh-activate"))
    );
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("sh-deactivate"))
    );
}

#[test]
fn init_path_no_push_path_guards_duplicate_shims() {
    let (_temp, ctx) = test_context();
    let report = cmd_init(
        &ctx,
        &[
            String::from("--path"),
            String::from("--no-push-path"),
            String::from("pwsh"),
        ],
    );
    assert_eq!(report.exit_code, 0);
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("Where-Object { $_ -and ($_ -ieq $__pyenv_shims) }"))
    );
}

#[test]
fn init_help_and_detect_shell_work() {
    let (_temp, ctx) = test_context();
    let help = cmd_init(&ctx, &[String::from("pwsh")]);
    assert_eq!(help.exit_code, 1);
    assert!(
        help.stderr
            .iter()
            .any(|line| line.contains("$PROFILE.CurrentUserCurrentHost"))
    );

    let detect = cmd_init(&ctx, &[String::from("--detect-shell")]);
    assert_eq!(detect.exit_code, 0);
    assert_eq!(detect.stdout[0], "PYENV_SHELL_DETECT=pwsh");
}

#[test]
fn init_help_for_fish_uses_source_syntax() {
    let (_temp, ctx) = test_context();
    let report = cmd_init(&ctx, &[String::from("fish")]);
    assert_eq!(report.exit_code, 1);
    assert!(
        report
            .stderr
            .iter()
            .any(|line| line == "pyenv init - fish | source")
    );
}

#[test]
fn init_print_for_bash_and_zsh_emit_sh_function_safely() {
    let (_temp, ctx) = test_context();
    for shell in ["bash", "zsh"] {
        let report = cmd_init(&ctx, &[String::from("-"), String::from(shell)]);
        assert_eq!(report.exit_code, 0);
        assert!(report.stdout.iter().any(|line| line == "pyenv() {"));
        assert!(
            report
                .stdout
                .iter()
                .any(|line| line.contains("if [ \"$#\" -eq 0 ]; then"))
        );
        assert!(report.stdout.iter().all(|line| !line.contains("local ")));
        assert!(
            report
                .stdout
                .iter()
                .any(|line| line.contains("pyenv_output=\"$(\"$pyenv_exe\""))
        );
    }
}

#[test]
fn init_print_for_fish_emits_fish_specific_function() {
    let (_temp, ctx) = test_context();
    let report = cmd_init(&ctx, &[String::from("-"), String::from("fish")]);
    assert_eq!(report.exit_code, 0);
    assert!(report.stdout.iter().any(|line| line == "function pyenv"));
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("if test (count $argv) -eq 0"))
    );
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("switch \"$command\""))
    );
}

#[test]
fn init_no_push_path_for_bash_uses_case_guard() {
    let (_temp, ctx) = test_context();
    let report = cmd_init(
        &ctx,
        &[
            String::from("--path"),
            String::from("--no-push-path"),
            String::from("bash"),
        ],
    );
    assert_eq!(report.exit_code, 0);
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line == "case \":${PATH}:\" in")
    );
    assert!(report.stdout.iter().all(|line| !line.contains("[[")));
}

#[test]
fn init_path_for_pwsh_tracks_shell_init_guard() {
    let (_temp, ctx) = test_context();
    let report = cmd_init(&ctx, &[String::from("--path"), String::from("pwsh")]);
    assert_eq!(report.exit_code, 0);
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("_PYENV_SHELL_INIT_SHIMS"))
    );
}

#[test]
fn sh_cmd_generates_cmd_lines() {
    let (_temp, ctx) = test_context();
    let report = cmd_sh_cmd(&ctx, &[String::from("versions"), String::from("--bare")]);
    assert_eq!(report.exit_code, 0);
    assert!(report.stdout[0].contains("\"pyenv\" versions --bare"));
}

#[test]
fn virtualenv_init_delegates_to_init_output() {
    let (_temp, ctx) = test_context();
    let report = cmd_virtualenv_init(&ctx, &[String::from("-"), String::from("pwsh")]);
    assert_eq!(report.exit_code, 0);
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("function pyenv"))
    );
    assert!(
        report
            .stdout
            .iter()
            .any(|line| line.contains("sh-activate"))
    );
}
