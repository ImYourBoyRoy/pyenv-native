#!/bin/sh
# ./scripts/check-i18n.sh
# Verify Fluent catalogs, overlay JSON parity, generated install-strings, and
# hardcoded GUI empty-state/badge copy.

set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
EN_DIR="$ROOT/locales/en-US"
LOCALES_DIR="$ROOT/locales"

fail() {
    printf 'i18n check failed: %s\n' "$1" >&2
    exit 1
}

extract_ids() {
    grep -E '^[A-Za-z0-9_-]+[ \t]*=' "$1" | sed -E 's/[ \t]*=.*//' | sort -u
}

extract_ids_raw() {
    grep -E '^[A-Za-z0-9_-]+[ \t]*=' "$1" | sed -E 's/[ \t]*=.*//' | sort
}

[ -d "$EN_DIR" ] || fail "missing $EN_DIR"

TMPDIR_I18N="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_I18N"' EXIT

EN_ALL="$TMPDIR_I18N/en-all"
: >"$EN_ALL"
for ftl in "$EN_DIR"/*.ftl; do
    extract_ids "$ftl" >>"$EN_ALL"
done
sort -u -o "$EN_ALL" "$EN_ALL"

LOCALE_COUNT=0
for locale_dir in "$LOCALES_DIR"/*/; do
    tag="$(basename -- "$locale_dir")"
    LOCALE_COUNT=$((LOCALE_COUNT + 1))
    case "$tag" in
        zh-TW|zh-Hant|zh-HK|zh-MO)
            fail "Traditional Chinese catalog $tag is not shipped; map those tags to zh-CN"
            ;;
    esac
    for ftl in "$EN_DIR"/*.ftl; do
        name="$(basename -- "$ftl")"
        target="$locale_dir$name"
        [ -f "$target" ] || fail "$tag is missing $name"
        dups="$(extract_ids_raw "$target" | uniq -d || true)"
        if [ -n "$dups" ]; then
            printf 'i18n check failed: %s/%s duplicate ids:\n%s\n' "$tag" "$name" "$dups" >&2
            exit 1
        fi
        extract_ids "$ftl" >"$TMPDIR_I18N/en"
        extract_ids "$target" >"$TMPDIR_I18N/loc"
        missing="$(comm -23 "$TMPDIR_I18N/en" "$TMPDIR_I18N/loc" || true)"
        extra="$(comm -13 "$TMPDIR_I18N/en" "$TMPDIR_I18N/loc" || true)"
        if [ -n "$missing" ]; then
            printf 'i18n check failed: %s/%s missing ids:\n%s\n' "$tag" "$name" "$missing" >&2
            exit 1
        fi
        if [ -n "$extra" ]; then
            printf 'i18n check failed: %s/%s extra ids:\n%s\n' "$tag" "$name" "$extra" >&2
            exit 1
        fi
    done
done

if [ "$LOCALE_COUNT" -lt 14 ]; then
    fail "expected 14 locale directories (en-US + 13 translations); found $LOCALE_COUNT"
fi

python3 "$ROOT/scripts/i18n/seed_locales.py" --check

printf 'i18n checks passed (%s locales, %s message ids).\n' "$LOCALE_COUNT" "$(wc -l <"$EN_ALL" | tr -d ' ')"
