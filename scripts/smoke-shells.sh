# ./scripts/smoke-shells.sh
# Purpose: Smoke-test POSIX shell integration for pyenv-native using bash and a temporary portable root.
# How to run: sh ./scripts/smoke-shells.sh [path-to-pyenv]
# Inputs: Optional path to a built pyenv binary. Defaults to ./target/debug/pyenv relative to the repo root.
# Outputs/side effects: Creates a temporary workspace, evaluates `pyenv init - bash`, and verifies that `pyenv shell 3.13.12` resolves correctly.
# Notes: Intended for CI smoke coverage of bash shell integration and dotted-version forwarding on Linux/macOS.

set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "${SCRIPT_DIR}/.." && pwd)"
PYENV_EXE="${1:-${REPO_ROOT}/target/debug/pyenv}"
SMOKE_ROOT="${REPO_ROOT}/.tmp-shell-smoke-posix"
PYENV_ROOT_PATH="${SMOKE_ROOT}/.pyenv"
WORK_DIR="${SMOKE_ROOT}/work"

rm -rf "$SMOKE_ROOT"
mkdir -p "${PYENV_ROOT_PATH}/versions/3.13.12" "$WORK_DIR"

export PYENV_ROOT="$PYENV_ROOT_PATH"
cd "$WORK_DIR"
eval "$("$PYENV_EXE" init - bash)"
pyenv shell 3.13.12
RESULT="$(pyenv version-name)"
[ "$RESULT" = "3.13.12" ] || {
  printf 'Expected version-name to resolve to 3.13.12 but found %s\n' "$RESULT" >&2
  exit 1
}

printf 'Bash shell smoke test passed.\n'
