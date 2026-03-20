// ./crates/pyenv-core/src/shell/emit/activation.rs
//! Virtual-environment activation and deactivation emitters for supported shells.

use crate::shell::types::ShellKind;

use super::quotes::{fish_single_quote, ps_double_quote, ps_single_quote, sh_single_quote};

pub(crate) fn shell_emit_activate(
    shell: ShellKind,
    version_value: &str,
    venv_path: &str,
    bin_path: &str,
) -> Vec<String> {
    match shell {
        ShellKind::Pwsh => vec![
            "$Env:PYENV_VERSION_OLD = $Env:PYENV_VERSION".to_string(),
            format!("$Env:PYENV_VERSION = \"{}\"", ps_double_quote(version_value)),
            "if (Test-Path Env:_PYENV_VIRTUAL_PATH_OLD) { $Env:PATH = $Env:_PYENV_VIRTUAL_PATH_OLD }".to_string(),
            "$Env:_PYENV_VIRTUAL_ENV_OLD = $Env:VIRTUAL_ENV".to_string(),
            format!("$Env:VIRTUAL_ENV = '{}'", ps_single_quote(venv_path)),
            "$Env:_PYENV_VIRTUAL_PATH_OLD = $Env:PATH".to_string(),
            format!("$Env:PATH = '{};' + $Env:PATH", ps_single_quote(bin_path)),
        ],
        ShellKind::Cmd => vec![
            "set \"PYENV_VERSION_OLD=%PYENV_VERSION%\"".to_string(),
            format!("set \"PYENV_VERSION={version_value}\""),
            "if defined _PYENV_VIRTUAL_PATH_OLD set \"PATH=%_PYENV_VIRTUAL_PATH_OLD%\"".to_string(),
            "set \"_PYENV_VIRTUAL_ENV_OLD=%VIRTUAL_ENV%\"".to_string(),
            format!("set \"VIRTUAL_ENV={venv_path}\""),
            "set \"_PYENV_VIRTUAL_PATH_OLD=%PATH%\"".to_string(),
            format!("set \"PATH={bin_path};%PATH%\""),
        ],
        ShellKind::Fish => vec![
            "set -gu PYENV_VERSION_OLD \"$PYENV_VERSION\"".to_string(),
            format!("set -gx PYENV_VERSION \"{version_value}\""),
            "if set -q _PYENV_VIRTUAL_PATH_OLD".to_string(),
            "  set -gx PATH (string split ':' -- $_PYENV_VIRTUAL_PATH_OLD)".to_string(),
            "end".to_string(),
            "set -gu _PYENV_VIRTUAL_ENV_OLD \"$VIRTUAL_ENV\"".to_string(),
            format!("set -gx VIRTUAL_ENV '{}'", fish_single_quote(venv_path)),
            "set -gx _PYENV_VIRTUAL_PATH_OLD (string join ':' -- $PATH)".to_string(),
            format!("set -gx PATH '{}' $PATH", fish_single_quote(bin_path)),
        ],
        _ => vec![
            "PYENV_VERSION_OLD=\"${PYENV_VERSION-}\"".to_string(),
            format!("export PYENV_VERSION=\"{version_value}\""),
            "if [ -n \"${_PYENV_VIRTUAL_PATH_OLD+x}\" ]; then PATH=\"$_PYENV_VIRTUAL_PATH_OLD\"; fi".to_string(),
            "_PYENV_VIRTUAL_ENV_OLD=\"${VIRTUAL_ENV-}\"".to_string(),
            format!("export VIRTUAL_ENV='{}'", sh_single_quote(venv_path)),
            "export _PYENV_VIRTUAL_PATH_OLD=\"$PATH\"".to_string(),
            format!("export PATH='{}':\"$PATH\"", sh_single_quote(bin_path)),
        ],
    }
}

pub(crate) fn shell_emit_deactivate(shell: ShellKind) -> Vec<String> {
    match shell {
        ShellKind::Pwsh => vec![
            "if (Test-Path Env:_PYENV_VIRTUAL_PATH_OLD) { $Env:PATH = $Env:_PYENV_VIRTUAL_PATH_OLD; Remove-Item Env:_PYENV_VIRTUAL_PATH_OLD -ErrorAction SilentlyContinue }".to_string(),
            "if (Test-Path Env:_PYENV_VIRTUAL_ENV_OLD) {".to_string(),
            "  $oldVirtualEnv = $Env:_PYENV_VIRTUAL_ENV_OLD".to_string(),
            "  if ([string]::IsNullOrEmpty($oldVirtualEnv)) { Remove-Item Env:VIRTUAL_ENV -ErrorAction SilentlyContinue } else { $Env:VIRTUAL_ENV = $oldVirtualEnv }".to_string(),
            "  Remove-Item Env:_PYENV_VIRTUAL_ENV_OLD -ErrorAction SilentlyContinue".to_string(),
            "} else { Remove-Item Env:VIRTUAL_ENV -ErrorAction SilentlyContinue }".to_string(),
            "if (Test-Path Env:PYENV_VERSION_OLD) {".to_string(),
            "  $oldPyenvVersion = $Env:PYENV_VERSION_OLD".to_string(),
            "  if ([string]::IsNullOrEmpty($oldPyenvVersion)) { Remove-Item Env:PYENV_VERSION -ErrorAction SilentlyContinue } else { $Env:PYENV_VERSION = $oldPyenvVersion }".to_string(),
            "  Remove-Item Env:PYENV_VERSION_OLD -ErrorAction SilentlyContinue".to_string(),
            "} else { Remove-Item Env:PYENV_VERSION -ErrorAction SilentlyContinue }".to_string(),
        ],
        ShellKind::Cmd => vec![
            "if defined _PYENV_VIRTUAL_PATH_OLD set \"PATH=%_PYENV_VIRTUAL_PATH_OLD%\"".to_string(),
            "set \"_PYENV_VIRTUAL_PATH_OLD=\"".to_string(),
            "if defined _PYENV_VIRTUAL_ENV_OLD (set \"VIRTUAL_ENV=%_PYENV_VIRTUAL_ENV_OLD%\") else set \"VIRTUAL_ENV=\"".to_string(),
            "set \"_PYENV_VIRTUAL_ENV_OLD=\"".to_string(),
            "if defined PYENV_VERSION_OLD (set \"PYENV_VERSION=%PYENV_VERSION_OLD%\") else set \"PYENV_VERSION=\"".to_string(),
            "set \"PYENV_VERSION_OLD=\"".to_string(),
        ],
        ShellKind::Fish => vec![
            "if set -q _PYENV_VIRTUAL_PATH_OLD".to_string(),
            "  set -gx PATH (string split ':' -- $_PYENV_VIRTUAL_PATH_OLD)".to_string(),
            "  set -e _PYENV_VIRTUAL_PATH_OLD".to_string(),
            "end".to_string(),
            "if set -q _PYENV_VIRTUAL_ENV_OLD".to_string(),
            "  if test -n \"$_PYENV_VIRTUAL_ENV_OLD\"".to_string(),
            "    set -gx VIRTUAL_ENV \"$_PYENV_VIRTUAL_ENV_OLD\"".to_string(),
            "  else".to_string(),
            "    set -e VIRTUAL_ENV".to_string(),
            "  end".to_string(),
            "  set -e _PYENV_VIRTUAL_ENV_OLD".to_string(),
            "else".to_string(),
            "  set -e VIRTUAL_ENV".to_string(),
            "end".to_string(),
            "if set -q PYENV_VERSION_OLD".to_string(),
            "  if test -n \"$PYENV_VERSION_OLD\"".to_string(),
            "    set -gx PYENV_VERSION \"$PYENV_VERSION_OLD\"".to_string(),
            "  else".to_string(),
            "    set -e PYENV_VERSION".to_string(),
            "  end".to_string(),
            "  set -e PYENV_VERSION_OLD".to_string(),
            "else".to_string(),
            "  set -e PYENV_VERSION".to_string(),
            "end".to_string(),
        ],
        _ => vec![
            "if [ -n \"${_PYENV_VIRTUAL_PATH_OLD+x}\" ]; then export PATH=\"$_PYENV_VIRTUAL_PATH_OLD\"; unset _PYENV_VIRTUAL_PATH_OLD; fi".to_string(),
            "if [ -n \"${_PYENV_VIRTUAL_ENV_OLD+x}\" ]; then".to_string(),
            "  if [ -n \"$_PYENV_VIRTUAL_ENV_OLD\" ]; then export VIRTUAL_ENV=\"$_PYENV_VIRTUAL_ENV_OLD\"; else unset VIRTUAL_ENV; fi".to_string(),
            "  unset _PYENV_VIRTUAL_ENV_OLD".to_string(),
            "else".to_string(),
            "  unset VIRTUAL_ENV".to_string(),
            "fi".to_string(),
            "if [ -n \"${PYENV_VERSION_OLD+x}\" ]; then".to_string(),
            "  if [ -n \"$PYENV_VERSION_OLD\" ]; then export PYENV_VERSION=\"$PYENV_VERSION_OLD\"; else unset PYENV_VERSION; fi".to_string(),
            "  unset PYENV_VERSION_OLD".to_string(),
            "else".to_string(),
            "  unset PYENV_VERSION".to_string(),
            "fi".to_string(),
        ],
    }
}
