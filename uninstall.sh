#!/usr/bin/env sh
# ./uninstall.sh
# Purpose: Removes a portable pyenv-native POSIX installation without requiring a local repo checkout.
# How to run: sh ./uninstall.sh [--install-root <dir>] [--remove-root]
# Inputs: Optional install root plus booleans controlling shell-profile cleanup and full-root deletion.
# Outputs/side effects: Removes portable pyenv-native binaries/wrappers and optionally cleans shell-profile integration and the managed root.
# Notes: Intended to pair with the remote install.sh flow and keeps uninstall package-manager-free.

set -eu

INSTALL_ROOT="${HOME}/.pyenv"
REMOVE_SHELL_PROFILE_BLOCK="true"
REMOVE_ROOT="false"
SHELL_KIND=""

parse_bool() {
  case "$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" in
    1|true|yes|on) printf '%s\n' "true" ;;
    0|false|no|off) printf '%s\n' "false" ;;
    *) printf 'Invalid boolean value `%s`\n' "$1" >&2; exit 1 ;;
  esac
}

detect_shell_kind() {
  shell_name="$(basename -- "${SHELL:-sh}" | tr '[:upper:]' '[:lower:]')"
  case "$shell_name" in
    bash|zsh|fish|sh) printf '%s\n' "$shell_name" ;;
    *) printf '%s\n' "sh" ;;
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

if [ -z "$SHELL_KIND" ]; then
  SHELL_KIND="$(detect_shell_kind)"
fi

mkdir -p "$(dirname -- "$INSTALL_ROOT")"
INSTALL_ROOT="$(CDPATH= cd -- "$(dirname -- "$INSTALL_ROOT")" && pwd)/$(basename -- "$INSTALL_ROOT")"
INSTALL_BIN="${INSTALL_ROOT}/bin"

rm -f "${INSTALL_BIN}/pyenv" "${INSTALL_BIN}/pyenv.cmd" "${INSTALL_BIN}/pyenv.ps1" "${INSTALL_BIN}/pyenv-init.cmd"

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
