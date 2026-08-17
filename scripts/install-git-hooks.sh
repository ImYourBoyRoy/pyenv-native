#!/usr/bin/env sh
# ./scripts/install-git-hooks.sh
# Point this clone at repo-local hooks so pre-push runs scripts/check-all.sh.
set -eu
ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
git -C "$ROOT" config core.hooksPath .githooks
chmod +x "$ROOT/.githooks/pre-push" "$ROOT/scripts/check-all.sh"
printf 'Git hooksPath set to .githooks (pre-push runs scripts/check-all.sh).\n'
