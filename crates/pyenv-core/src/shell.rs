// ./crates/pyenv-core/src/shell.rs
//! Shell integration and init output for PowerShell-first pyenv workflows.

use std::fs;

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::manage::cmd_prefix;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InitMode {
    Help,
    Print,
    Path,
    DetectShell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitCommandOptions {
    mode: InitMode,
    shell: ShellKind,
    no_push_path: bool,
    no_rehash: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellKind {
    Pwsh,
    Cmd,
    Bash,
    Zsh,
    Fish,
    Sh,
}

impl ShellKind {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "pwsh" | "powershell" | "ps" => Some(Self::Pwsh),
            "cmd" | "cmd.exe" | "batch" => Some(Self::Cmd),
            "bash" => Some(Self::Bash),
            "zsh" => Some(Self::Zsh),
            "fish" => Some(Self::Fish),
            "sh" => Some(Self::Sh),
            _ => None,
        }
    }

    fn canonical_name(self) -> &'static str {
        match self {
            Self::Pwsh => "pwsh",
            Self::Cmd => "cmd",
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
            Self::Sh => "sh",
        }
    }
}

pub fn cmd_shell(_ctx: &AppContext, _args: &[String]) -> CommandReport {
    CommandReport::failure(
        vec![
            "pyenv: shell integration not enabled. Run `pyenv init' for instructions.".to_string(),
        ],
        1,
    )
}

pub fn cmd_sh_shell(ctx: &AppContext, args: &[String]) -> CommandReport {
    let shell = effective_shell(ctx);
    let args = if matches!(args.first().map(String::as_str), Some("--")) {
        &args[1..]
    } else {
        args
    };

    if args.is_empty() {
        return match &ctx.env_version {
            Some(_) => CommandReport::success(shell_emit_show_current(shell)),
            None => CommandReport::failure(
                vec!["pyenv: no shell-specific version configured".to_string()],
                1,
            ),
        };
    }

    if args.len() == 1 && args[0] == "--unset" {
        return CommandReport::success(shell_emit_unset(shell));
    }

    if args.len() == 1 && args[0] == "-" {
        return CommandReport::success(shell_emit_revert(shell));
    }

    let requested = args
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if requested.is_empty() {
        return CommandReport::failure(
            vec!["pyenv: no shell-specific version configured".to_string()],
            1,
        );
    }

    if let Err(error) = validate_shell_versions(ctx, &requested) {
        return CommandReport::failure(vec![error.to_string()], 1);
    }

    let version_value = requested.join(":");
    if ctx.env_version.as_deref() == Some(version_value.as_str()) {
        return CommandReport::empty_success();
    }

    CommandReport::success(shell_emit_set(shell, &version_value))
}

pub fn cmd_sh_rehash(ctx: &AppContext) -> CommandReport {
    let shell = effective_shell(ctx);
    CommandReport::success(shell_emit_rehash(
        shell,
        &ctx.exe_path.display().to_string(),
    ))
}

pub fn cmd_sh_cmd(ctx: &AppContext, args: &[String]) -> CommandReport {
    let args = if matches!(args.first().map(String::as_str), Some("--")) {
        &args[1..]
    } else {
        args
    };

    let Some((command, rest)) = args.split_first() else {
        return CommandReport::success(vec![format!("\"{}\"", ctx.exe_path.display())]);
    };

    match command.to_ascii_lowercase().as_str() {
        "shell" => cmd_sh_shell(ctx, rest),
        "rehash" => cmd_sh_rehash(ctx),
        _ => CommandReport::success(vec![render_cmd_exec_line(&ctx.exe_path, args)]),
    }
}

pub fn cmd_init(ctx: &AppContext, args: &[String]) -> CommandReport {
    let options = match parse_init_args(ctx, args) {
        Ok(options) => options,
        Err(error) => return CommandReport::failure(vec![error], 1),
    };

    if let Err(error) = ensure_init_dirs(ctx) {
        return CommandReport::failure(vec![error.to_string()], 1);
    }

    match options.mode {
        InitMode::Help => CommandReport {
            stdout: Vec::new(),
            stderr: render_init_help(options.shell),
            exit_code: 1,
        },
        InitMode::DetectShell => CommandReport::success(render_detect_shell(options.shell)),
        InitMode::Path => CommandReport::success(render_init_path(ctx, &options)),
        InitMode::Print => CommandReport::success(render_init_print(ctx, &options)),
    }
}

fn parse_init_args(ctx: &AppContext, args: &[String]) -> Result<InitCommandOptions, String> {
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

fn effective_shell(ctx: &AppContext) -> ShellKind {
    detect_shell(ctx)
}

fn ensure_init_dirs(ctx: &AppContext) -> Result<(), PyenvError> {
    fs::create_dir_all(ctx.shims_dir()).map_err(io_error)?;
    fs::create_dir_all(ctx.versions_dir()).map_err(io_error)?;
    Ok(())
}

fn render_init_help(shell: ShellKind) -> Vec<String> {
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

fn render_detect_shell(shell: ShellKind) -> Vec<String> {
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

fn render_init_print(ctx: &AppContext, options: &InitCommandOptions) -> Vec<String> {
    let mut lines = render_init_path(ctx, options);
    lines.extend(shell_emit_set_shell(options.shell));
    lines.extend(render_shell_function(
        options.shell,
        &ctx.exe_path.display().to_string(),
    ));
    lines
}

fn render_init_path(ctx: &AppContext, options: &InitCommandOptions) -> Vec<String> {
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

fn render_shell_function(shell: ShellKind, exe_path: &str) -> Vec<String> {
    match shell {
        ShellKind::Pwsh => vec![
            "function Join-PyenvWindowsArguments {".to_string(),
            "  param([string[]]$PyenvArgs)".to_string(),
            "  $parts = foreach ($arg in $PyenvArgs) {".to_string(),
            "    if ($null -eq $arg) { '\"\"'; continue }".to_string(),
            "    if ($arg.Length -eq 0) { '\"\"'; continue }".to_string(),
            "    if ($arg -notmatch '[\\s\"]') { $arg; continue }".to_string(),
            "    $escaped = $arg -replace '(\\\\*)\"', '$1$1\\\"'".to_string(),
            "    $escaped = $escaped -replace '(\\\\+)$', '$1$1'".to_string(),
            "    '\"' + $escaped + '\"'".to_string(),
            "  }".to_string(),
            "  return ($parts -join ' ')".to_string(),
            "}".to_string(),
            "function Invoke-PyenvCaptured {".to_string(),
            "  param([string]$PyenvExe, [string[]]$PyenvArgs)".to_string(),
            "  $psi = [System.Diagnostics.ProcessStartInfo]::new()".to_string(),
            "  $psi.FileName = $PyenvExe".to_string(),
            "  $psi.UseShellExecute = $false".to_string(),
            "  $psi.RedirectStandardOutput = $true".to_string(),
            "  $psi.RedirectStandardError = $true".to_string(),
            "  $psi.WorkingDirectory = (Get-Location).Path".to_string(),
            "  if ($psi.PSObject.Properties.Name -contains 'ArgumentList' -and $null -ne $psi.ArgumentList) {"
                .to_string(),
            "    foreach ($arg in $PyenvArgs) { [void]$psi.ArgumentList.Add([string]$arg) }"
                .to_string(),
            "  } else {".to_string(),
            "    $psi.Arguments = Join-PyenvWindowsArguments $PyenvArgs".to_string(),
            "  }".to_string(),
            "  $process = [System.Diagnostics.Process]::Start($psi)".to_string(),
            "  $stdout = $process.StandardOutput.ReadToEnd()".to_string(),
            "  $stderr = $process.StandardError.ReadToEnd()".to_string(),
            "  $process.WaitForExit()".to_string(),
            "  $global:LASTEXITCODE = $process.ExitCode".to_string(),
            "  if ($stderr.Length -gt 0) { [Console]::Error.Write($stderr) }".to_string(),
            "  if ($stdout.Length -eq 0) { return @() }".to_string(),
            "  return ($stdout.TrimEnd() -split \"`r?`n\")".to_string(),
            "}".to_string(),
            "function Invoke-PyenvPassthrough {".to_string(),
            "  param([string]$PyenvExe, [string[]]$PyenvArgs)".to_string(),
            "  $psi = [System.Diagnostics.ProcessStartInfo]::new()".to_string(),
            "  $psi.FileName = $PyenvExe".to_string(),
            "  $psi.UseShellExecute = $false".to_string(),
            "  $psi.WorkingDirectory = (Get-Location).Path".to_string(),
            "  if ($psi.PSObject.Properties.Name -contains 'ArgumentList' -and $null -ne $psi.ArgumentList) {"
                .to_string(),
            "    foreach ($arg in $PyenvArgs) { [void]$psi.ArgumentList.Add([string]$arg) }"
                .to_string(),
            "  } else {".to_string(),
            "    $psi.Arguments = Join-PyenvWindowsArguments $PyenvArgs".to_string(),
            "  }".to_string(),
            "  $psi.RedirectStandardOutput = $false".to_string(),
            "  $psi.RedirectStandardError = $false".to_string(),
            "  $process = [System.Diagnostics.Process]::Start($psi)".to_string(),
            "  $process.WaitForExit()".to_string(),
            "  $global:LASTEXITCODE = $process.ExitCode".to_string(),
            "  return $process.ExitCode".to_string(),
            "}".to_string(),
            "function pyenv {".to_string(),
            format!("  $pyenvExe = '{}'", ps_single_quote(exe_path)),
            "  if ($args.Count -eq 0) {".to_string(),
            "    Invoke-PyenvPassthrough $pyenvExe @() | Out-Null".to_string(),
            "    return".to_string(),
            "  }".to_string(),
            "  $command = $args[0]".to_string(),
            "  $arguments = if ($args.Count -gt 1) { @($args[1..($args.Count - 1)]) } else { @() }".to_string(),
            "  switch ($command) {".to_string(),
            "    'shell' {".to_string(),
            "      $shellCmds = Invoke-PyenvCaptured $pyenvExe (@('sh-shell', '--') + $arguments)".to_string(),
            "      if ($LASTEXITCODE -eq 0 -and $shellCmds.Count -gt 0) { Invoke-Expression ($shellCmds -join \"`n\") }".to_string(),
            "    }".to_string(),
            "    'rehash' {".to_string(),
            "      $shellCmds = Invoke-PyenvCaptured $pyenvExe (@('sh-rehash') + $arguments)".to_string(),
            "      if ($LASTEXITCODE -eq 0 -and $shellCmds.Count -gt 0) { Invoke-Expression ($shellCmds -join \"`n\") }".to_string(),
            "    }".to_string(),
            "    default {".to_string(),
            "      Invoke-PyenvPassthrough $pyenvExe (@([string]$command) + $arguments) | Out-Null".to_string(),
            "    }".to_string(),
            "  }".to_string(),
            "}".to_string(),
        ],
        ShellKind::Cmd => vec![
            format!(
                "doskey pyenv=for /f \"delims=\" %i in ('\"{}\" sh-cmd $*') do %i",
                exe_path
            ),
        ],
        ShellKind::Fish => vec![
            "function pyenv".to_string(),
            format!("  set pyenv_exe '{}'", fish_single_quote(exe_path)),
            "  if test (count $argv) -eq 0".to_string(),
            "    $pyenv_exe".to_string(),
            "    return $status".to_string(),
            "  end".to_string(),
            "  set command $argv[1]".to_string(),
            "  set -e argv[1]".to_string(),
            "  switch \"$command\"".to_string(),
            "  case shell".to_string(),
            "    set shell_cmds ($pyenv_exe sh-shell -- $argv)".to_string(),
            "    set shell_status $status".to_string(),
            "    if test $shell_status -ne 0".to_string(),
            "      return $shell_status".to_string(),
            "    end".to_string(),
            "    if test (count $shell_cmds) -gt 0".to_string(),
            "      string join \\n -- $shell_cmds | source".to_string(),
            "    end".to_string(),
            "  case rehash".to_string(),
            "    set shell_cmds ($pyenv_exe sh-rehash $argv)".to_string(),
            "    set shell_status $status".to_string(),
            "    if test $shell_status -ne 0".to_string(),
            "      return $shell_status".to_string(),
            "    end".to_string(),
            "    if test (count $shell_cmds) -gt 0".to_string(),
            "      string join \\n -- $shell_cmds | source".to_string(),
            "    end".to_string(),
            "  case '*'".to_string(),
            "    $pyenv_exe $command $argv".to_string(),
            "  end".to_string(),
            "end".to_string(),
        ],
        _ => vec![
            "pyenv() {".to_string(),
            format!("  pyenv_exe='{}'", sh_single_quote(exe_path)),
            "  if [ \"$#\" -eq 0 ]; then".to_string(),
            "    \"$pyenv_exe\"".to_string(),
            "    return $?".to_string(),
            "  fi".to_string(),
            "  command_name=${1:-}".to_string(),
            "  shift".to_string(),
            "  case \"$command_name\" in".to_string(),
            "    shell)".to_string(),
            "      pyenv_output=\"$(\"$pyenv_exe\" sh-shell -- \"$@\")\" || return $?".to_string(),
            "      [ -z \"$pyenv_output\" ] || eval \"$pyenv_output\"".to_string(),
            "      ;;".to_string(),
            "    rehash)".to_string(),
            "      pyenv_output=\"$(\"$pyenv_exe\" sh-rehash \"$@\")\" || return $?".to_string(),
            "      [ -z \"$pyenv_output\" ] || eval \"$pyenv_output\"".to_string(),
            "      ;;".to_string(),
            "    *)".to_string(),
            "      \"$pyenv_exe\" \"$command_name\" \"$@\"".to_string(),
            "      ;;".to_string(),
            "  esac".to_string(),
            "}".to_string(),
        ],
    }
}

fn shell_emit_set_shell(shell: ShellKind) -> Vec<String> {
    match shell {
        ShellKind::Pwsh => vec![format!("$Env:PYENV_SHELL=\"{}\"", shell.canonical_name())],
        ShellKind::Cmd => vec![format!("set \"PYENV_SHELL={}\"", shell.canonical_name())],
        ShellKind::Fish => vec![format!("set -gx PYENV_SHELL {}", shell.canonical_name())],
        _ => vec![format!("export PYENV_SHELL={}", shell.canonical_name())],
    }
}

fn shell_emit_show_current(shell: ShellKind) -> Vec<String> {
    match shell {
        ShellKind::Pwsh => vec!["Write-Output $Env:PYENV_VERSION".to_string()],
        ShellKind::Cmd => vec!["echo %PYENV_VERSION%".to_string()],
        ShellKind::Fish => vec!["echo \"$PYENV_VERSION\"".to_string()],
        _ => vec!["echo \"$PYENV_VERSION\"".to_string()],
    }
}

fn shell_emit_unset(shell: ShellKind) -> Vec<String> {
    match shell {
        ShellKind::Pwsh => vec![
            "$Env:PYENV_VERSION_OLD = $Env:PYENV_VERSION".to_string(),
            "Remove-Item Env:PYENV_VERSION -ErrorAction SilentlyContinue".to_string(),
        ],
        ShellKind::Cmd => vec![
            "set \"PYENV_VERSION_OLD=%PYENV_VERSION%\"".to_string(),
            "set \"PYENV_VERSION=\"".to_string(),
        ],
        ShellKind::Fish => vec![
            "set -gu PYENV_VERSION_OLD \"$PYENV_VERSION\"".to_string(),
            "set -e PYENV_VERSION".to_string(),
        ],
        _ => vec![
            "PYENV_VERSION_OLD=\"${PYENV_VERSION-}\"".to_string(),
            "unset PYENV_VERSION".to_string(),
        ],
    }
}

fn shell_emit_revert(shell: ShellKind) -> Vec<String> {
    match shell {
        ShellKind::Pwsh => vec![
            "if (Test-Path Env:PYENV_VERSION_OLD) {".to_string(),
            "  $pyenvVersionOld = $Env:PYENV_VERSION_OLD".to_string(),
            "  $Env:PYENV_VERSION_OLD = $Env:PYENV_VERSION".to_string(),
            "  if ([string]::IsNullOrEmpty($pyenvVersionOld)) {".to_string(),
            "    Remove-Item Env:PYENV_VERSION -ErrorAction SilentlyContinue".to_string(),
            "  } else {".to_string(),
            "    $Env:PYENV_VERSION = $pyenvVersionOld".to_string(),
            "  }".to_string(),
            "} else {".to_string(),
            "  Write-Error \"pyenv: Env:PYENV_VERSION_OLD is not set\"".to_string(),
            "  return $false".to_string(),
            "}".to_string(),
        ],
        ShellKind::Cmd => vec![
            "if not defined PYENV_VERSION_OLD echo pyenv: PYENV_VERSION_OLD is not set & exit /b 1"
                .to_string(),
            "set \"__PYENV_VERSION_SWAP=%PYENV_VERSION%\"".to_string(),
            "set \"PYENV_VERSION=%PYENV_VERSION_OLD%\"".to_string(),
            "set \"PYENV_VERSION_OLD=%__PYENV_VERSION_SWAP%\"".to_string(),
            "set \"__PYENV_VERSION_SWAP=\"".to_string(),
        ],
        ShellKind::Fish => vec![
            "if set -q PYENV_VERSION_OLD".to_string(),
            "  if [ -n \"$PYENV_VERSION_OLD\" ]".to_string(),
            "    set PYENV_VERSION_OLD_ \"$PYENV_VERSION\"".to_string(),
            "    set -gx PYENV_VERSION \"$PYENV_VERSION_OLD\"".to_string(),
            "    set -gu PYENV_VERSION_OLD \"$PYENV_VERSION_OLD_\"".to_string(),
            "    set -e PYENV_VERSION_OLD_".to_string(),
            "  else".to_string(),
            "    set -gu PYENV_VERSION_OLD \"$PYENV_VERSION\"".to_string(),
            "    set -e PYENV_VERSION".to_string(),
            "  end".to_string(),
            "else".to_string(),
            "  echo \"pyenv: PYENV_VERSION_OLD is not set\" >&2".to_string(),
            "  false".to_string(),
            "end".to_string(),
        ],
        _ => vec![
            "if [ -n \"${PYENV_VERSION_OLD+x}\" ]; then".to_string(),
            "  if [ -n \"$PYENV_VERSION_OLD\" ]; then".to_string(),
            "    PYENV_VERSION_OLD_=\"$PYENV_VERSION\"".to_string(),
            "    export PYENV_VERSION=\"$PYENV_VERSION_OLD\"".to_string(),
            "    PYENV_VERSION_OLD=\"$PYENV_VERSION_OLD_\"".to_string(),
            "    unset PYENV_VERSION_OLD_".to_string(),
            "  else".to_string(),
            "    PYENV_VERSION_OLD=\"$PYENV_VERSION\"".to_string(),
            "    unset PYENV_VERSION".to_string(),
            "  fi".to_string(),
            "else".to_string(),
            "  echo \"pyenv: PYENV_VERSION_OLD is not set\" >&2".to_string(),
            "  false".to_string(),
            "fi".to_string(),
        ],
    }
}

fn shell_emit_set(shell: ShellKind, version_value: &str) -> Vec<String> {
    match shell {
        ShellKind::Pwsh => vec![
            "$Env:PYENV_VERSION_OLD = $Env:PYENV_VERSION".to_string(),
            format!(
                "$Env:PYENV_VERSION = \"{}\"",
                ps_double_quote(version_value)
            ),
        ],
        ShellKind::Cmd => vec![
            "set \"PYENV_VERSION_OLD=%PYENV_VERSION%\"".to_string(),
            format!("set \"PYENV_VERSION={version_value}\""),
        ],
        ShellKind::Fish => vec![
            "set -gu PYENV_VERSION_OLD \"$PYENV_VERSION\"".to_string(),
            format!("set -gx PYENV_VERSION \"{version_value}\""),
        ],
        _ => vec![
            "PYENV_VERSION_OLD=\"${PYENV_VERSION-}\"".to_string(),
            format!("export PYENV_VERSION=\"{version_value}\""),
        ],
    }
}

fn shell_emit_rehash(shell: ShellKind, exe_path: &str) -> Vec<String> {
    match shell {
        ShellKind::Pwsh => vec![format!("& '{}' rehash", ps_single_quote(exe_path))],
        ShellKind::Cmd => vec![format!("\"{}\" rehash", exe_path)],
        _ => vec![
            format!("\"{}\" rehash", exe_path),
            "hash -r 2>/dev/null || true".to_string(),
        ],
    }
}

fn render_cmd_exec_line(exe_path: &std::path::Path, args: &[String]) -> String {
    let mut parts = vec![format!("\"{}\"", exe_path.display())];
    parts.extend(args.iter().map(|arg| cmd_quote(arg)));
    parts.join(" ")
}

fn cmd_quote(value: &str) -> String {
    if value.is_empty()
        || value
            .chars()
            .any(|ch| ch.is_whitespace() || matches!(ch, '"' | '&' | '|' | '<' | '>' | '^'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn validate_shell_versions(ctx: &AppContext, versions: &[String]) -> Result<(), PyenvError> {
    for version in versions {
        #[allow(clippy::cloned_ref_to_slice_refs)]
        let report = cmd_prefix(ctx, &[version.clone()]);
        if report.exit_code != 0 {
            let message = report
                .stderr
                .first()
                .cloned()
                .unwrap_or_else(|| format!("pyenv: version `{version}` not installed"));
            return Err(PyenvError::Io(message));
        }
    }

    Ok(())
}

fn ps_single_quote(value: &str) -> String {
    value.replace('\'', "''")
}

fn ps_double_quote(value: &str) -> String {
    value.replace('`', "``").replace('"', "`\"")
}

fn sh_single_quote(value: &str) -> String {
    value.replace('\'', "'\\''")
}

fn fish_single_quote(value: &str) -> String {
    value.replace('\'', "\\'")
}

fn io_error(error: std::io::Error) -> PyenvError {
    PyenvError::Io(format!("pyenv: {error}"))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::{cmd_init, cmd_sh_cmd, cmd_sh_rehash, cmd_sh_shell, cmd_shell};

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

    #[test]
    fn shell_command_requires_integration() {
        let (_temp, ctx) = test_context();
        let report = cmd_shell(&ctx, &[]);
        assert_eq!(report.exit_code, 1);
        assert!(report.stderr[0].contains("shell integration not enabled"));
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
}
