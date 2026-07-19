#!/usr/bin/env sh
# ./scripts/check-version-sync.sh
# Purpose: Fail CI/release when Cargo workspace, Python package, and bootstrap
#          __version__ diverge (this previously left PyPI stuck while GitHub tags advanced).
# How to run: sh ./scripts/check-version-sync.sh
#             EXPECTED_VERSION=0.2.34 sh ./scripts/check-version-sync.sh  # optional pin
# Inputs: Cargo.toml, python-package/pyproject.toml,
#         python-package/src/pyenv_native_bootstrap/__init__.py; optional EXPECTED_VERSION.
# Outputs: Prints the shared version; non-zero exit on mismatch.
# Notes: Keep in sync with scripts/set-version.ps1 / check-version-sync.ps1.

set -eu

repo_root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cargo_toml="$repo_root/Cargo.toml"
pyproject="$repo_root/python-package/pyproject.toml"
python_init="$repo_root/python-package/src/pyenv_native_bootstrap/__init__.py"

extract_version() {
  path=$1
  pattern=$2
  label=$3
  value=$(sed -n "$pattern" "$path" | head -n 1)
  if [ -z "$value" ]; then
    printf 'error: could not read %s from %s\n' "$label" "$path" >&2
    exit 1
  fi
  printf '%s\n' "$value"
}

cargo_version=$(extract_version "$cargo_toml" 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' 'Cargo.toml version')
py_version=$(extract_version "$pyproject" 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' 'pyproject.toml version')
init_version=$(extract_version "$python_init" 's/^__version__[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' '__version__')

failed=0

if [ "$cargo_version" != "$py_version" ] || [ "$cargo_version" != "$init_version" ]; then
  printf 'error: version mismatch across release metadata:\n' >&2
  printf '  Cargo.toml:           %s\n' "$cargo_version" >&2
  printf '  python-package/pyproject.toml: %s\n' "$py_version" >&2
  printf '  __init__.__version__: %s\n' "$init_version" >&2
  printf 'Fix with: powershell -ExecutionPolicy Bypass -File ./scripts/set-version.ps1 -Version <semver>\n' >&2
  failed=1
fi

if [ -n "${EXPECTED_VERSION:-}" ] && [ "$cargo_version" != "$EXPECTED_VERSION" ]; then
  printf 'error: workspace version %s does not match expected %s\n' "$cargo_version" "$EXPECTED_VERSION" >&2
  failed=1
fi

if [ "$failed" -ne 0 ]; then
  exit 1
fi

printf 'Version sync OK: %s\n' "$cargo_version"
