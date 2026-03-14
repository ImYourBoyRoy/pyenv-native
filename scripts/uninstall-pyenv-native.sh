#!/usr/bin/env sh
# ./scripts/uninstall-pyenv-native.sh
# Purpose: Removes a portable POSIX pyenv-native installation and optionally cleans shell-profile integration.
# How to run: sh ./scripts/uninstall-pyenv-native.sh [--install-root <dir>] [--remove-root]
# Inputs: Optional install root plus booleans controlling profile cleanup and full-root deletion.
# Outputs/side effects: Removes installed binaries and optionally the shell profile block and full managed root.
# Notes: Keeps uninstall portable and avoids touching any system package manager state.

set -eu

INSTALL_ROOT="${HOME}/.pyenv"
REMOVE_SHELL_PROFILE_BLOCK="true"
REMOVE_ROOT="false"
SHELL_KIND="sh"

parse_bool() {
  case "$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" in
    1|true|yes|on) printf '%s\n' "true" ;;
    0|false|no|off) printf '%s\n' "false" ;;
    *) printf 'Invalid boolean value `%s`\n' "$1" >&2; exit 1 ;;
  esac
}

profile_path_for_shell() {
  case "$1" in
    bash) printf '%s\n' "${HOME}/.bashrc" ;;
    zsh) printf '%s\n' "${HOME}/.zshrc" ;;
    fish) printf '%s\n' "${HOME}/.config/fish/config.fish" ;;
    sh) printf '%s\n' "${HOME}/.profile" ;;
    none) printf '%s\n' "" ;;
    *) printf '%s\n' "" ;;
  esac
}

remove_profile_block() {
  profile_path="$1"
  [ -n "$profile_path" ] || return 0
  [ -f "$profile_path" ] || return 0

  begin_marker="# >>> pyenv-native init >>>"
  end_marker="# <<< pyenv-native init <<<"
  updated="$(awk -v begin="$begin_marker" -v end="$end_marker" '
    BEGIN { skipping = 0 }
    index($0, begin) == 1 { skipping = 1; next }
    index($0, end) == 1 { skipping = 0; next }
    skipping == 0 { print }
  ' "$profile_path")"
  if [ -n "$updated" ]; then
    printf '%s\n' "$updated" > "$profile_path"
  else
    rm -f "$profile_path"
  fi
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --install-root)
      INSTALL_ROOT="${2:-}"
      shift 2
      ;;
    --remove-shell-profile-block)
      REMOVE_SHELL_PROFILE_BLOCK="$(parse_bool "${2:-}")"
      shift 2
      ;;
    --remove-root)
      REMOVE_ROOT="true"
      shift 1
      ;;
    --shell)
      SHELL_KIND="${2:-}"
      shift 2
      ;;
    *)
      printf 'Unknown option `%s`\n' "$1" >&2
      exit 1
      ;;
  esac
done

INSTALL_ROOT="$(cd "$(dirname -- "$INSTALL_ROOT")" 2>/dev/null && pwd)/$(basename -- "$INSTALL_ROOT")"
INSTALL_BIN="${INSTALL_ROOT}/bin"

rm -f "${INSTALL_BIN}/pyenv"

if [ "$REMOVE_SHELL_PROFILE_BLOCK" = "true" ]; then
  remove_profile_block "$(profile_path_for_shell "$SHELL_KIND")"
fi

if [ "$REMOVE_ROOT" = "true" ] && [ -d "$INSTALL_ROOT" ]; then
  rm -rf "$INSTALL_ROOT"
fi

printf 'install_root: %s\n' "$INSTALL_ROOT"
printf 'install_bin: %s\n' "$INSTALL_BIN"
printf 'remove_shell_profile_block: %s\n' "$REMOVE_SHELL_PROFILE_BLOCK"
printf 'remove_root: %s\n' "$REMOVE_ROOT"
