#!/bin/sh
# ./scripts/check-gui-patterns.sh
# Guardrails for pyenv-native GUI JavaScript patterns.

set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
APP_JS="$ROOT/crates/pyenv-gui/ui/app.js"
DOM_UTILS="$ROOT/crates/pyenv-gui/ui/dom-utils.js"

fail() {
    printf 'GUI pattern check failed: %s\n' "$1" >&2
    exit 1
}

if rg -n 'onclick=' "$APP_JS" >/dev/null 2>&1; then
    fail 'inline onclick handlers are not allowed in app.js'
fi

if rg -n '\.innerHTML\s*=' "$APP_JS" >/dev/null 2>&1; then
    fail 'innerHTML assignments are not allowed in app.js (use DomUtils helpers)'
fi

if ! rg -q 'dom-utils\.js' "$ROOT/crates/pyenv-gui/ui/index.html"; then
    fail 'index.html must load dom-utils.js before app.js'
fi

if ! rg -q 'createElement' "$DOM_UTILS"; then
    fail 'dom-utils.js must provide safe DOM helpers'
fi

printf 'GUI pattern checks passed.\n'
