#!/usr/bin/env bash
# Cross-platform managed-venv migration smoke test for Linux/macOS.
# The test uses a temporary PYENV_ROOT and symlinks to existing runtimes only.

set -Eeuo pipefail

repo_root="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
pyenv_bin="${PYENV_BIN:-$repo_root/target/release/pyenv}"
source_runtime="${1:-${SOURCE_RUNTIME:-3.14.3}}"
target_runtime="${2:-${TARGET_RUNTIME:-3.15.2}}"
target_prefix="${target_runtime%.*}"

if [[ ! -x "$pyenv_bin" ]]; then
    echo "migration test requires an executable release CLI at $pyenv_bin" >&2
    exit 2
fi

installed_pyenv="${INSTALLED_PYENV_BIN:-$(command -v pyenv || true)}"
if [[ -z "$installed_pyenv" ]]; then
    echo "migration test requires an installed pyenv-native CLI on PATH" >&2
    exit 2
fi
installed_root="${INSTALLED_PYENV_ROOT:-$(PYENV_ROOT= "$installed_pyenv" root)}"
source_install="$installed_root/versions/$source_runtime"
target_install="$installed_root/versions/$target_runtime"
source_python="$source_install/bin/python"
target_python="$target_install/bin/python"

for required in "$source_install" "$target_install" "$source_python" "$target_python"; do
    if [[ ! -e "$required" ]]; then
        echo "migration test runtime path is missing: $required" >&2
        exit 2
    fi
done

temp_base="${TMPDIR:-/tmp}"
if [[ ! -d "$temp_base" ]]; then
    temp_base=/tmp
fi
temp_base="$(CDPATH= cd -- "$temp_base" && pwd -P)"
test_root="$(mktemp -d "$temp_base/pyenv-native-migration.XXXXXX")"
resolved_test_root="$(CDPATH= cd -- "$test_root" && pwd -P)"
if [[ "$resolved_test_root" != "$temp_base"/pyenv-native-migration.* ]]; then
    echo "refusing to use an unexpected temporary root: $test_root" >&2
    exit 2
fi

cleanup() {
    if [[ -d "$test_root" ]] && [[ "$(CDPATH= cd -- "$test_root" && pwd -P)" == "$temp_base"/pyenv-native-migration.* ]]; then
        find "$test_root" -depth -mindepth 1 -delete
        rmdir "$test_root"
    fi
}
trap cleanup EXIT

export PYTHONDONTWRITEBYTECODE=1
export PYTHONNOUSERSITE=1
export PIP_DISABLE_PIP_VERSION_CHECK=1
export PIP_PROGRESS_BAR=off

mkdir -p "$test_root/versions"
ln -s "$source_install" "$test_root/versions/$source_runtime"
ln -s "$target_install" "$test_root/versions/$target_runtime"

fixture_dir="$test_root/fixtures"
"$source_python" "$repo_root/scripts/venv_migration_fixture.py" "$fixture_dir" "$target_runtime" >/dev/null

source_env="$test_root/venvs/$source_runtime/migration-test"
target_env="$test_root/venvs/$target_runtime/migration-test"
mkdir -p "$(dirname "$source_env")" "$(dirname "$target_env")"
env PIP_CONFIG_FILE=/dev/null PIP_CACHE_DIR="$test_root/pip-cache" \
    "$source_python" -m venv "$source_env"
env PIP_CONFIG_FILE=/dev/null PIP_CACHE_DIR="$test_root/pip-cache" \
    "$target_python" -m venv "$target_env"

env PIP_CONFIG_FILE=/dev/null PIP_CACHE_DIR="$test_root/pip-cache" \
    "$source_env/bin/python" -m pip install --no-index --find-links "$fixture_dir" \
    migration-compat-demo==1.0.0 >/dev/null

printf '%s\n' "$source_runtime/envs/migration-test" > "$test_root/.python-version"

set +e
upgrade_output="$(
    cd "$test_root" && \
    PYENV_ROOT="$test_root" PIP_CONFIG_FILE=/dev/null PIP_CACHE_DIR="$test_root/pip-cache" \
    PIP_NO_INDEX=1 PIP_FIND_LINKS="$fixture_dir" \
    "$pyenv_bin" venv upgrade --force --set-local \
    "$source_runtime/envs/migration-test" "$target_prefix" 2>&1
)"
upgrade_status=$?
set -e
printf '%s\n' "$upgrade_output"
if [[ $upgrade_status -ne 0 ]]; then
    echo "managed-venv migration failed with exit code $upgrade_status" >&2
    exit "$upgrade_status"
fi

target_probe="$($target_env/bin/python -c 'import migration_compat_demo,sys; print(f"python={sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"); print(f"fixture={migration_compat_demo.__version__}")')"
printf '%s\n' "$target_probe"
grep -qx "fixture=2.0.0" <<<"$target_probe"
grep -qx "$target_runtime/envs/migration-test" "$test_root/.python-version"
[[ ! -e "$source_env" ]]
[[ -x "$target_env/bin/python" ]]
! compgen -G "$test_root/venvs/$target_runtime/migration-test-previous*" >/dev/null

incompatible_fixture_dir="$test_root/fixtures-incompatible"
"$source_python" "$repo_root/scripts/venv_migration_fixture.py" "$incompatible_fixture_dir" "$target_runtime" >/dev/null
rm -f "$incompatible_fixture_dir/migration_compat_demo-2.0.0-py3-none-any.whl"
failure_env="$test_root/venvs/$source_runtime/failure-test"
failure_target="$test_root/venvs/$target_runtime/failure-test"
mkdir -p "$(dirname "$failure_env")"
env PIP_CONFIG_FILE=/dev/null PIP_CACHE_DIR="$test_root/pip-cache" \
    "$source_python" -m venv "$failure_env"
env PIP_CONFIG_FILE=/dev/null PIP_CACHE_DIR="$test_root/pip-cache" \
    "$failure_env/bin/python" -m pip install --no-index --find-links "$incompatible_fixture_dir" \
    migration-compat-demo==1.0.0 >/dev/null

set +e
failure_output="$(
    cd "$test_root" && \
    PYENV_ROOT="$test_root" PIP_CONFIG_FILE=/dev/null PIP_CACHE_DIR="$test_root/pip-cache" \
    PIP_NO_INDEX=1 PIP_FIND_LINKS="$incompatible_fixture_dir" \
    "$pyenv_bin" venv upgrade --force \
    "$source_runtime/envs/failure-test" "$target_prefix" 2>&1
)"
failure_status=$?
set -e
printf '%s\n' "$failure_output"
if [[ $failure_status -eq 0 ]]; then
    echo "incompatible migration unexpectedly succeeded" >&2
    exit 1
fi
[[ -d "$failure_env" ]]
[[ ! -e "$failure_target" ]]
! compgen -G "$test_root/venvs/$target_runtime/failure-test-migrating*" >/dev/null

echo "managed-venv migration smoke test passed: $source_runtime -> $target_runtime"
