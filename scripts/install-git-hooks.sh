#!/usr/bin/env sh
# ./scripts/install-git-hooks.sh
# Point this clone at repo-local hooks so pre-push runs `cargo xtask check`.
set -eu
ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
git -C "$ROOT" config core.hooksPath .githooks
chmod +x "$ROOT/.githooks/pre-push"
printf 'Git hooksPath set to .githooks (pre-push runs cargo xtask check).\n'
