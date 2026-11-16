#!/usr/bin/env bash
# ./scripts/smoke-shells.sh
# Purpose: Smoke-test POSIX shell integration for Enchi using bash and a temporary portable root.
# How to run: bash ./scripts/smoke-shells.sh [path-to-enchi]
# Inputs: Optional path to a built enchi binary. Defaults to ./target/debug/enchi relative to the repo root.
# Outputs/side effects: Creates a temporary workspace, evaluates `enchi init - bash`, and verifies that the shell wrapper (`enchi`, or the `pyenv` compatibility alias, whichever the init code defines) resolves `shell 3.13.12` correctly.
# Notes: Intended for CI smoke coverage of bash shell integration and dotted-version forwarding on Linux/macOS.

set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "${SCRIPT_DIR}/.." && pwd)"
ENCHI_EXE_INPUT="${1:-${REPO_ROOT}/target/debug/enchi}"
SMOKE_ROOT="${REPO_ROOT}/.tmp-shell-smoke-posix"
ENCHI_ROOT_PATH="${SMOKE_ROOT}/.enchi"
WORK_DIR="${SMOKE_ROOT}/work"

case "$ENCHI_EXE_INPUT" in
  /*) ENCHI_EXE="$ENCHI_EXE_INPUT" ;;
  *) ENCHI_EXE="${REPO_ROOT}/${ENCHI_EXE_INPUT#./}" ;;
esac

rm -rf "$SMOKE_ROOT"
mkdir -p "${ENCHI_ROOT_PATH}/versions/3.13.12" "$WORK_DIR"

export ENCHI_ROOT="$ENCHI_ROOT_PATH"
cd "$WORK_DIR"
eval "$("$ENCHI_EXE" init --no-rehash - bash)"

# The init code defines the `enchi` wrapper and may also define the `pyenv` alias; test each that exists.
tested=0
for wrapper in enchi pyenv; do
  if [ "$(type -t "$wrapper" 2>/dev/null)" = function ]; then
    "$wrapper" shell 3.13.12
    RESULT="$("$wrapper" version-name)"
    [ "$RESULT" = "3.13.12" ] || {
      printf 'Expected %s version-name to resolve to 3.13.12 but found %s\n' "$wrapper" "$RESULT" >&2
      exit 1
    }
    tested=$((tested + 1))
  fi
done
[ "$tested" -gt 0 ] || {
  printf 'The init code defined neither an enchi nor a pyenv shell function\n' >&2
  exit 1
}

printf 'Bash shell smoke test passed.\n'
