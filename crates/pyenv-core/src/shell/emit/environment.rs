// ./crates/pyenv-core/src/shell/emit/environment.rs
//! Environment-variable and rehash emitters for shell-scoped version management.

use super::quotes::{ps_double_quote, ps_single_quote};
use crate::shell::types::ShellKind;

pub(crate) fn shell_emit_set_shell(shell: ShellKind) -> Vec<String> {
    match shell {
        ShellKind::Pwsh => vec![format!("$Env:PYENV_SHELL=\"{}\"", shell.canonical_name())],
        ShellKind::Cmd => vec![format!("set \"PYENV_SHELL={}\"", shell.canonical_name())],
        ShellKind::Fish => vec![format!("set -gx PYENV_SHELL {}", shell.canonical_name())],
        _ => vec![format!("export PYENV_SHELL={}", shell.canonical_name())],
    }
}

pub(crate) fn shell_emit_show_current(shell: ShellKind) -> Vec<String> {
    match shell {
        ShellKind::Pwsh => vec!["Write-Output $Env:PYENV_VERSION".to_string()],
        ShellKind::Cmd => vec!["echo %PYENV_VERSION%".to_string()],
        _ => vec!["echo \"$PYENV_VERSION\"".to_string()],
    }
}

pub(crate) fn shell_emit_unset(shell: ShellKind) -> Vec<String> {
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

pub(crate) fn shell_emit_revert(shell: ShellKind) -> Vec<String> {
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

pub(crate) fn shell_emit_set(shell: ShellKind, version_value: &str) -> Vec<String> {
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

pub(crate) fn shell_emit_rehash(shell: ShellKind, exe_path: &str) -> Vec<String> {
    match shell {
        ShellKind::Pwsh => vec![format!("& '{}' rehash", ps_single_quote(exe_path))],
        ShellKind::Cmd => vec![format!("\"{}\" rehash", exe_path)],
        _ => vec![
            format!("\"{}\" rehash", exe_path),
            "hash -r 2>/dev/null || true".to_string(),
        ],
    }
}
