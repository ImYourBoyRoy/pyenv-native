#!/usr/bin/env sh
# ./uninstall.sh
# Purpose: Remote-friendly uninstaller for pyenv-native on Linux, macOS, and Termux. Runs the
#   installed binary's `pyenv self-uninstall`, which removes the program files, shims, and the
#   shell-profile block it added.
# How to run: curl -fsSL <release>/uninstall.sh | sh -s -- --remove-root
#             sh ./uninstall.sh [--install-root <dir>] [--remove-root] [--remove-shell-profile-block <bool>]
# Inputs:
#   --install-root <dir>   Install to remove (default: $PYENV_ROOT or ~/.pyenv)
#   --remove-root          Also delete installed Pythons, virtual environments, and caches.
#                          Without it those are kept (`pyenv self-uninstall --keep-data`).
#   --remove-shell-profile-block <bool>   Pass false to leave shell integration in place.
# Notes: Needs the installed pyenv binary; this script contains no uninstall logic of its own.

set -eu

INSTALL_ROOT="${PYENV_ROOT:-${HOME}/.pyenv}"
REMOVE_ROOT="false"
KEEP_INTEGRATION="false"

fail() { printf 'pyenv-native: %s\n' "$*" >&2; exit 1; }

while [ "$#" -gt 0 ]; do
  case "$1" in
    --install-root) [ "$#" -ge 2 ] || fail "--install-root requires a value"; INSTALL_ROOT="$2"; shift 2 ;;
    --remove-root) REMOVE_ROOT="true"; shift ;;
    --remove-shell-profile-block)
      [ "$#" -ge 2 ] || fail "--remove-shell-profile-block requires a value"
      case "$(printf '%s' "$2" | tr '[:upper:]' '[:lower:]')" in
        0|false|no|off) KEEP_INTEGRATION="true" ;;
        1|true|yes|on) KEEP_INTEGRATION="false" ;;
        *) fail "invalid boolean '$2' for --remove-shell-profile-block" ;;
      esac
      shift 2
      ;;
    -h|--help) sed -n '2,/^set -eu/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) fail "unknown option '$1'" ;;
  esac
done

case "$INSTALL_ROOT" in
  "~") INSTALL_ROOT="$HOME" ;;
  "~/"*) INSTALL_ROOT="$HOME${INSTALL_ROOT#\~}" ;;
esac

PYENV_BIN="${INSTALL_ROOT}/bin/pyenv"
[ -x "$PYENV_BIN" ] || fail "no pyenv-native install found at ${INSTALL_ROOT} (missing ${PYENV_BIN}). Pass --install-root, or remove the shell-profile block by hand."

set -- self-uninstall --yes
[ "$REMOVE_ROOT" = "true" ] || set -- "$@" --keep-data
[ "$KEEP_INTEGRATION" = "false" ] || set -- "$@" --keep-shell-integration
exec env PYENV_ROOT="$INSTALL_ROOT" "$PYENV_BIN" "$@"
