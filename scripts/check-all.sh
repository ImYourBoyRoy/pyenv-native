#!/usr/bin/env bash
# ./scripts/check-all.sh
# Local + pre-push quality gate: format, clippy, tests, Python package, GUI a11y, optional cargo-audit.
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "--- Format ---"
cargo fmt --check

echo "--- Clippy (workspace) ---"
cargo clippy --workspace -- -D warnings

echo "--- Tests (workspace) ---"
cargo test --workspace

echo "--- Version sync ---"
sh ./scripts/check-version-sync.sh

echo "--- GUI patterns ---"
sh ./scripts/check-gui-patterns.sh

echo "--- Python bootstrap tests ---"
PYTHONPATH=./python-package/src python3 -m unittest discover -s ./python-package/tests -p "test_*.py"

if command -v pnpm >/dev/null 2>&1; then
  echo "--- GUI a11y (pnpm) ---"
  (cd scripts/gui-a11y && pnpm install --frozen-lockfile && pnpm audit --prod && pnpm run check)
else
  echo "error: pnpm is required for GUI a11y checks (corepack enable && corepack prepare pnpm@latest --activate)" >&2
  exit 1
fi

if command -v cargo-audit >/dev/null 2>&1; then
  echo "--- cargo audit ---"
  cargo audit
else
  echo "--- Skipping cargo audit (install with: cargo install cargo-audit --locked) ---"
fi

echo "--- All checks PASSED ---"
