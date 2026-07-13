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

if command -v rg >/dev/null 2>&1; then
    has_match() {
        rg -n "$1" "$2" >/dev/null 2>&1
    }
    has_pattern() {
        rg -q "$1" "$2"
    }
else
    has_match() {
        grep -nE "$1" "$2" >/dev/null 2>&1
    }
    has_pattern() {
        grep -qE "$1" "$2"
    }
fi

if has_match 'onclick=' "$APP_JS"; then
    fail 'inline onclick handlers are not allowed in app.js'
fi

if has_match '\.innerHTML[[:space:]]*=' "$APP_JS"; then
    fail 'innerHTML assignments are not allowed in app.js (use DomUtils helpers)'
fi

if ! has_pattern 'dom-utils\.js' "$ROOT/crates/pyenv-gui/ui/index.html"; then
    fail 'index.html must load dom-utils.js before app.js'
fi

if ! has_pattern 'createElement' "$DOM_UTILS"; then
    fail 'dom-utils.js must provide safe DOM helpers'
fi

printf 'GUI pattern checks passed.\n'
