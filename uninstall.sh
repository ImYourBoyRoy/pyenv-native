#!/usr/bin/env sh
# ./uninstall.sh
# Purpose: Remote-friendly uninstaller for Enchi (formerly pyenv-native) on Linux, macOS, and Termux.
#   Runs the installed binary's `enchi self-uninstall`, which removes the program files, shims, and
#   the shell-profile block it added.
# How to run: curl -fsSL <release>/uninstall.sh | sh -s -- --remove-root
#             sh ./uninstall.sh [--install-root <dir>] [--remove-root] [--remove-shell-profile-block <bool>]
# Inputs:
#   --install-root <dir>   Install to remove (default: $ENCHI_ROOT, else $PYENV_ROOT, else ~/.enchi,
#                          else an existing pyenv-native install at ~/.pyenv)
#   --remove-root          Also delete installed Pythons, virtual environments, and caches.
#                          Without it those are kept (`enchi self-uninstall --keep-data`).
#   --remove-shell-profile-block <bool>   Pass false to leave shell integration in place.
# Notes: Needs the installed enchi binary (or `pyenv`, for installs made before the rename); this
#   script contains no uninstall logic of its own.

set -eu

INSTALL_ROOT="${ENCHI_ROOT:-${PYENV_ROOT:-}}"
REMOVE_ROOT="false"
KEEP_INTEGRATION="false"

fail() { printf 'enchi: %s\n' "$*" >&2; exit 1; }

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

# Find the install to remove: the first of ~/.enchi and ~/.pyenv that holds a binary.
if [ -z "$INSTALL_ROOT" ]; then
  for candidate in "${HOME}/.enchi" "${HOME}/.pyenv"; do
    if [ -x "${candidate}/bin/enchi" ] || [ -x "${candidate}/bin/pyenv" ]; then
      INSTALL_ROOT="$candidate"
      break
    fi
  done
  [ -n "$INSTALL_ROOT" ] || INSTALL_ROOT="${HOME}/.enchi"
fi

case "$INSTALL_ROOT" in
  "~") INSTALL_ROOT="$HOME" ;;
  "~/"*) INSTALL_ROOT="$HOME${INSTALL_ROOT#\~}" ;;
esac

# Installs made before the rename only have `pyenv`; upgraded ones have both.
ENCHI_BIN=""
for name in enchi pyenv; do
  if [ -x "${INSTALL_ROOT}/bin/${name}" ]; then ENCHI_BIN="${INSTALL_ROOT}/bin/${name}"; break; fi
done
[ -n "$ENCHI_BIN" ] || fail "no Enchi install found at ${INSTALL_ROOT} (missing ${INSTALL_ROOT}/bin/enchi). Pass --install-root, or remove the shell-profile block by hand."

set -- self-uninstall --yes
[ "$REMOVE_ROOT" = "true" ] || set -- "$@" --keep-data
[ "$KEEP_INTEGRATION" = "false" ] || set -- "$@" --keep-shell-integration
# Both variables, so whichever generation of the binary runs honours the root.
exec env ENCHI_ROOT="$INSTALL_ROOT" PYENV_ROOT="$INSTALL_ROOT" "$ENCHI_BIN" "$@"
