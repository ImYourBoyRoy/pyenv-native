#!/usr/bin/env bash

set -e
trap 'if [ $? -ne 0 ]; then echo -e "\033[31m--- Tests FAILED ---\033[0m"; fi' EXIT

FULL_CLEAN=0
if [[ "$1" == "--full-clean" ]]; then
    FULL_CLEAN=1
fi

if [ $FULL_CLEAN -eq 1 ]; then
    echo -e "\033[36m--- Running full clean ---\033[0m"
    cargo clean
    rm -rf ~/.pyenv
fi

echo -e "\033[36m--- Running all tests (workspace) ---\033[0m"
cargo test --workspace

echo -e "\033[36m--- Running lint checks ---\033[0m"
cargo fmt --check
cargo clippy --workspace -- -D warnings

echo -e "\033[36m--- Running POSIX shell smoke tests ---\033[0m"
cargo build -p pyenv-cli

if [ -f "./scripts/smoke-shells.sh" ]; then
    bash ./scripts/smoke-shells.sh
else
    echo -e "\033[33m--- Skipping shell smoke tests (script missing) ---\033[0m"
fi

trap - EXIT
echo -e "\033[32m--- All checks PASSED ---\033[0m"
