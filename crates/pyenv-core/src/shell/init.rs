// ./crates/pyenv-core/src/shell/init.rs
//! Init parsing and shell bootstrap rendering for PATH setup, shell detection, and helper
//! function emission.

use std::fs;

use crate::context::AppContext;
use crate::error::PyenvError;

use super::emit::{
    ps_single_quote, render_shell_function, shell_emit_rehash, shell_emit_set_shell,
};
use super::helpers::io_error;
use super::types::{InitCommandOptions, InitMode, ShellKind};

pub(super) fn parse_init_args(
    ctx: &AppContext,
    args: &[String],
) -> Result<InitCommandOptions, String> {
    let mut mode = InitMode::Help;
    let mut shell = None;
    let mut no_push_path = false;
    let mut no_rehash = false;

    for arg in args {
        match arg.as_str() {
            "-" => mode = InitMode::Print,
            "--path" => mode = InitMode::Path,
            "--detect-shell" => mode = InitMode::DetectShell,
            "--no-push-path" => no_push_path = true,
            "--no-rehash" => no_rehash = true,
            value if value.starts_with('-') => {
                return Err(format!("pyenv: unknown init option `{value}`"));
            }
            value => {
                shell = Some(
                    ShellKind::parse(value)
                        .ok_or_else(|| format!("pyenv: unsupported shell `{value}`"))?,
                );
            }
        }
    }

    Ok(InitCommandOptions {
        mode,
        shell: shell.unwrap_or_else(|| detect_shell(ctx)),
        no_push_path,
        no_rehash,
    })
}

pub(super) fn effective_shell(ctx: &AppContext) -> ShellKind {
    detect_shell(ctx)
}

pub(super) fn ensure_init_dirs(ctx: &AppContext) -> Result<(), PyenvError> {
    fs::create_dir_all(ctx.shims_dir()).map_err(io_error)?;
    fs::create_dir_all(ctx.versions_dir()).map_err(io_error)?;
    Ok(())
}

pub(super) fn render_init_help(shell: ShellKind) -> Vec<String> {
    match shell {
        ShellKind::Pwsh => vec![
            "# Load pyenv automatically by appending".to_string(),
            "# the following to $PROFILE.CurrentUserCurrentHost:".to_string(),
            String::new(),
            "iex ((pyenv init - pwsh) -join \"`n\")".to_string(),
            String::new(),
            "# Restart your shell for the changes to take effect.".to_string(),
        ],
        ShellKind::Cmd => vec![
            "REM For CMD, initialize pyenv in each session with:".to_string(),
            "FOR /F \"delims=\" %i IN ('pyenv init - cmd') DO %i".to_string(),
            String::new(),
            "REM This adds shims to PATH and installs a doskey macro for `pyenv shell`."
                .to_string(),
        ],
        ShellKind::Fish => vec![
            "# Load pyenv automatically by evaluating the generated init script:".to_string(),
            "pyenv init - fish | source".to_string(),
            String::new(),
            "# Restart your shell for the changes to take effect.".to_string(),
        ],
        _ => vec![
            "# Load pyenv automatically by evaluating the generated init script:".to_string(),
            format!("eval \"$(pyenv init - {})\"", shell.canonical_name()),
            String::new(),
            "# Restart your shell for the changes to take effect.".to_string(),
        ],
    }
}

pub(super) fn render_detect_shell(shell: ShellKind) -> Vec<String> {
    let (profile, rc) = match shell {
        ShellKind::Pwsh => (
            "$PROFILE.CurrentUserCurrentHost",
            "$PROFILE.CurrentUserCurrentHost",
        ),
        ShellKind::Cmd => (
            "HKCU\\Software\\Microsoft\\Command Processor\\AutoRun",
            "HKCU\\Software\\Microsoft\\Command Processor\\AutoRun",
        ),
        ShellKind::Bash => ("~/.bash_profile", "~/.bashrc"),
        ShellKind::Zsh => ("~/.zprofile", "~/.zshrc"),
        ShellKind::Fish => ("~/.config/fish/config.fish", "~/.config/fish/config.fish"),
        ShellKind::Sh => ("~/.profile", "~/.profile"),
    };

    vec![
        format!("PYENV_SHELL_DETECT={}", shell.canonical_name()),
        format!("PYENV_PROFILE_DETECT={profile}"),
        format!("PYENV_RC_DETECT={rc}"),
    ]
}

pub(super) fn render_init_print(ctx: &AppContext, options: &InitCommandOptions) -> Vec<String> {
    let mut lines = render_init_path(ctx, options);
    lines.extend(shell_emit_set_shell(options.shell));
    lines.extend(render_shell_function(
        options.shell,
        &ctx.exe_path.display().to_string(),
    ));
    lines
}

pub(super) fn render_init_path(ctx: &AppContext, options: &InitCommandOptions) -> Vec<String> {
    let shims = ctx.shims_dir().display().to_string();
    let mut lines = match options.shell {
        ShellKind::Pwsh => render_pwsh_path_lines(&shims, options.no_push_path),
        ShellKind::Cmd => render_cmd_path_lines(&shims, options.no_push_path),
        ShellKind::Fish => render_fish_path_lines(&shims, options.no_push_path),
        _ => render_sh_path_lines(&shims, options.no_push_path),
    };

    if !options.no_rehash {
        lines.extend(shell_emit_rehash(
            options.shell,
            &ctx.exe_path.display().to_string(),
        ));
    }
    lines
}

fn detect_shell(ctx: &AppContext) -> ShellKind {
    ctx.env_shell
        .as_deref()
        .and_then(ShellKind::parse)
        .or({
            if cfg!(windows) {
                Some(ShellKind::Pwsh)
            } else {
                Some(ShellKind::Bash)
            }
        })
        .unwrap_or(ShellKind::Pwsh)
}

fn render_pwsh_path_lines(shims: &str, no_push_path: bool) -> Vec<String> {
    let quoted = ps_single_quote(shims);
    if no_push_path {
        vec![
            format!("$__pyenv_shims = '{quoted}'"),
            "$Env:_PYENV_SHELL_INIT_SHIMS = $__pyenv_shims".to_string(),
            "$__pyenv_path = if ($Env:PATH) { $Env:PATH -split ';' } else { @() }".to_string(),
            "if (-not ($__pyenv_path | Where-Object { $_ -and ($_ -ieq $__pyenv_shims) })) {"
                .to_string(),
            "  $Env:PATH = (@($__pyenv_shims) + $__pyenv_path) -join ';'".to_string(),
            "}".to_string(),
            "Remove-Variable __pyenv_shims, __pyenv_path -ErrorAction SilentlyContinue".to_string(),
        ]
    } else {
        vec![
            format!("$__pyenv_shims = '{quoted}'"),
            "if ($Env:_PYENV_SHELL_INIT_SHIMS -ine $__pyenv_shims) {".to_string(),
            "  $__pyenv_path = if ($Env:PATH) { $Env:PATH -split ';' | Where-Object { $_ -and ($_ -ine $__pyenv_shims) } } else { @() }".to_string(),
            "  $Env:PATH = (@($__pyenv_shims) + $__pyenv_path) -join ';'".to_string(),
            "  $Env:_PYENV_SHELL_INIT_SHIMS = $__pyenv_shims".to_string(),
            "}".to_string(),
            "Remove-Variable __pyenv_shims, __pyenv_path -ErrorAction SilentlyContinue".to_string(),
        ]
    }
}

fn render_cmd_path_lines(shims: &str, no_push_path: bool) -> Vec<String> {
    let mut lines = vec![format!("set \"__PYENV_SHIMS={shims}\"")];
    if no_push_path {
        lines.extend([
            "set \"__PYENV_PATH_CHECK=;%PATH%;\"".to_string(),
            "if /I \"%__PYENV_PATH_CHECK:;%__PYENV_SHIMS%;=%\"==\"%__PYENV_PATH_CHECK%\" set \"PATH=%__PYENV_SHIMS%;%PATH%\"".to_string(),
            "set \"_PYENV_SHELL_INIT_SHIMS=%__PYENV_SHIMS%\"".to_string(),
            "set \"__PYENV_PATH_CHECK=\"".to_string(),
        ]);
    } else {
        lines.extend([
            "if /I not \"%_PYENV_SHELL_INIT_SHIMS%\"==\"%__PYENV_SHIMS%\" set \"PATH=%__PYENV_SHIMS%;%PATH%\"".to_string(),
            "set \"_PYENV_SHELL_INIT_SHIMS=%__PYENV_SHIMS%\"".to_string(),
        ]);
    }
    lines.push("set \"__PYENV_SHIMS=\"".to_string());
    lines
}

fn render_sh_path_lines(shims: &str, no_push_path: bool) -> Vec<String> {
    if no_push_path {
        vec![
            "case \":${PATH}:\" in".to_string(),
            format!("  *\":{shims}:\"*) ;;"),
            format!("  *) export PATH=\"{shims}:${{PATH}}\" ;;"),
            "esac".to_string(),
            format!("export _PYENV_SHELL_INIT_SHIMS=\"{shims}\""),
        ]
    } else {
        vec![
            format!("if [ \"${{_PYENV_SHELL_INIT_SHIMS-}}\" != \"{shims}\" ]; then"),
            format!("  export PATH=\"{shims}:${{PATH}}\""),
            format!("  export _PYENV_SHELL_INIT_SHIMS=\"{shims}\""),
            "fi".to_string(),
        ]
    }
}

fn render_fish_path_lines(shims: &str, no_push_path: bool) -> Vec<String> {
    if no_push_path {
        vec![
            format!("if not contains -- '{shims}' $PATH"),
            format!("  set -gx PATH '{shims}' $PATH"),
            "end".to_string(),
            format!("set -gx _PYENV_SHELL_INIT_SHIMS '{shims}'"),
        ]
    } else {
        vec![
            format!("if test \"$ _PYENV_SHELL_INIT_SHIMS\" != '{shims}'")
                .replace("$ _PYENV_SHELL_INIT_SHIMS", "$_PYENV_SHELL_INIT_SHIMS"),
            format!("  set -gx PATH '{shims}' $PATH"),
            format!("  set -gx _PYENV_SHELL_INIT_SHIMS '{shims}'"),
            "end".to_string(),
        ]
    }
}
