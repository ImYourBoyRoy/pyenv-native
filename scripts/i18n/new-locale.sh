#!/bin/sh
# ./scripts/i18n/new-locale.sh
# Seed a new Fluent locale from en-US. Usage: sh ./scripts/i18n/new-locale.sh <bcp47-tag>

set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
TAG="${1:-}"

if [ -z "$TAG" ]; then
    printf 'usage: sh ./scripts/i18n/new-locale.sh <bcp47-tag>\n' >&2
    exit 1
fi

case "$TAG" in
    zh-TW|zh-Hant|zh-HK|zh-MO)
        printf 'Traditional Chinese is mapped to zh-CN. Do not add a separate catalog.\n' >&2
        exit 1
        ;;
esac

SRC="$ROOT/locales/en-US"
DEST="$ROOT/locales/$TAG"
if [ -d "$DEST" ]; then
    printf 'locale %s already exists at %s\n' "$TAG" "$DEST" >&2
    exit 1
fi

mkdir -p "$DEST"
for ftl in "$SRC"/*.ftl; do
    sed "s|./locales/en-US/|./locales/${TAG}/|" "$ftl" > "$DEST/$(basename -- "$ftl")"
done

printf 'Created %s from en-US. Add scripts/i18n/overlays/%s.json, list the tag in seed_locales.py LOCALES, translate, seed, then run sh ./scripts/check-i18n.sh\n' "$DEST" "$TAG"
