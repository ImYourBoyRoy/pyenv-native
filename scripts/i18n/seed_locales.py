#!/usr/bin/env python3
# ./scripts/i18n/seed_locales.py
# Copy en-US Fluent catalogs into every shipped locale, overlaying
# scripts/i18n/overlays/<tag>.json. English remains the source of truth for ids.
# How to run: python3 ./scripts/i18n/seed_locales.py
#             python3 ./scripts/i18n/seed_locales.py --check
# Inputs: locales/en-US/*.ftl and overlays/<tag>.json
# Outputs: locales/<tag>/*.ftl and generated install-strings.sh/.ps1
# --check compares overlay key sets, regenerated install-strings, and app.js
# hardcoded empty-state/badge copy without writing files.

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EN_DIR = ROOT / "locales" / "en-US"
OVERLAY_DIR = ROOT / "scripts" / "i18n" / "overlays"
APP_JS = ROOT / "crates" / "pyenv-gui" / "ui" / "app.js"
LOCALES = [
    "zh-CN",
    "es",
    "ja",
    "ko",
    "pt-BR",
    "fr",
    "de",
    "ru",
    "fa",
    "ar",
    "hi",
    "it",
    "tr",
]

# Translations live only in overlays/<tag>.json. Do not add an in-file overlay table.


def overlay_path(tag: str) -> Path:
    return OVERLAY_DIR / f"{tag}.json"


def load_json_overlay(tag: str) -> dict[str, str]:
    path = overlay_path(tag)
    if not path.is_file():
        raise SystemExit(f"missing overlay {path}")
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise SystemExit(f"{path} must be a JSON object of message-id -> translation")
    return {str(key): str(value) for key, value in data.items()}


def overlay_ftl(source: str, translations: dict[str, str]) -> str:
    lines = source.splitlines(keepends=True)
    out: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()
        replaced = False
        if stripped and not stripped.startswith("#") and not stripped.startswith(".") and "=" in stripped:
            ident = stripped.split("=", 1)[0].strip()
            if ident in translations:
                indent = line[: len(line) - len(line.lstrip())]
                value = translations[ident]
                if "\n" in value:
                    out.append(f"{indent}{ident} =\n")
                    for part in value.split("\n"):
                        out.append(f"{indent}    {part}\n")
                else:
                    out.append(f"{indent}{ident} = {value}\n")
                replaced = True
                # Skip continuation lines of the original multiline value,
                # including blank lines that sit inside the message.
                i += 1
                while i < len(lines) and (
                    lines[i].startswith(" ")
                    or lines[i].startswith("\t")
                    or not lines[i].strip()
                ):
                    i += 1
                continue
        if not replaced:
            out.append(line)
        i += 1
    return "".join(out)


def _ftl_messages(path: Path) -> dict[str, str]:
    messages: dict[str, str] = {}
    ident: str | None = None
    parts: list[str] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if ident and (line.startswith(" ") or line.startswith("\t")):
            parts.append(stripped)
            continue
        if ident:
            messages[ident] = "\n".join(parts).strip()
            ident = None
            parts = []
        if stripped and not stripped.startswith("#") and not stripped.startswith(".") and "=" in stripped:
            name, value = stripped.split("=", 1)
            ident = name.strip()
            value = value.strip()
            if value:
                messages[ident] = value
                ident = None
            else:
                parts = []
    if ident:
        messages[ident] = "\n".join(parts).strip()
    return messages


def en_us_ids() -> set[str]:
    ids: set[str] = set()
    for ftl in sorted(EN_DIR.glob("*.ftl")):
        ids.update(_ftl_messages(ftl))
    return ids


def _shell_single_quote(value: str) -> str:
    return "'" + value.replace("'", "'\\''") + "'"


_INSTALL_PLACEHOLDER = re.compile(r"\{\s*\$[A-Za-z0-9_-]+\s*\}")
_HARDCODED_COPY = re.compile(
    r"(showEmptyState|appendBadge)\s*\(\s*[^,]+,\s*(['\"`])",
)


def render_install_strings() -> tuple[str, str]:
    tags = ["en-US", *LOCALES]
    catalog: dict[str, dict[str, str]] = {}
    for tag in tags:
        catalog[tag] = _ftl_messages(ROOT / "locales" / tag / "install.ftl")
    keys = sorted(catalog["en-US"].keys())

    sh_lines = [
        "#!/bin/sh",
        "# ./scripts/i18n/install-strings.sh",
        "# Generated from locales/*/install.ftl by seed_locales.py. Do not edit by hand.",
        "",
        "pyenv_normalize_install_lang() {",
        "  raw=\"$(printf '%s' \"$1\" | tr '[:upper:]' '[:lower:]' | tr '_' '-')\"",
        "  case \"$raw\" in",
        "    zh|zh-cn|zh-hans|zh-hans-cn|zh-sg|zh-tw|zh-hant|zh-hant-tw|zh-hk|zh-mo) printf 'zh-CN' ;;",
        "    pt|pt-pt|pt-br) printf 'pt-BR' ;;",
        "    en|en-us|en-gb|en-au|en-ca|'') printf 'en-US' ;;",
        "    es|es-mx|es-ar|es-es|es-419) printf 'es' ;;",
        "    ja|ko|fr|de|ru|fa|ar|hi|it|tr) printf '%s' \"$raw\" ;;",
        "    *) printf 'en-US' ;;",
        "  esac",
        "}",
        "",
        "pyenv_install_tr() {",
        "  key=\"$1\"",
        "  lang=\"$(pyenv_normalize_install_lang \"${PYENV_INSTALL_LANG:-en-US}\")\"",
        "  case \"$lang:$key\" in",
    ]
    for tag in tags:
        for key in keys:
            value = catalog[tag].get(key, catalog["en-US"][key])
            value = _INSTALL_PLACEHOLDER.sub("%s", value)
            quoted = _shell_single_quote(value)
            if "%s" in value:
                sh_lines.append(f"    {tag}:{key}) printf {quoted} \"$2\" ;;")
            else:
                sh_lines.append(f"    {tag}:{key}) printf %s {quoted} ;;")
    sh_lines.extend(
        [
            "    *) printf '%s' \"$key\" ;;",
            "  esac",
            "}",
            "",
        ]
    )

    ps_lines = [
        "# ./scripts/i18n/install-strings.ps1",
        "# Generated from locales/*/install.ftl by seed_locales.py. Do not edit by hand.",
        "",
        "function ConvertTo-PyenvInstallLang {",
        "    param([string]$Lang)",
        "    $raw = ([string]$Lang).ToLowerInvariant().Replace('_', '-')",
        "    switch -Regex ($raw) {",
        "        '^(zh|zh-cn|zh-hans|zh-hans-cn|zh-sg|zh-tw|zh-hant|zh-hant-tw|zh-hk|zh-mo)$' { 'zh-CN'; break }",
        "        '^(pt|pt-pt|pt-br)$' { 'pt-BR'; break }",
        "        '^(en|en-us|en-gb|en-au|en-ca)$' { 'en-US'; break }",
        "        '^(es|es-mx|es-ar|es-es|es-419)$' { 'es'; break }",
        "        '^(ja|ko|fr|de|ru|fa|ar|hi|it|tr)$' { $raw; break }",
        "        default { 'en-US' }",
        "    }",
        "}",
        "",
        "function Get-PyenvInstallString {",
        "    param([string]$Key, [string]$Lang, [string]$Arg = '')",
        "    $normalized = ConvertTo-PyenvInstallLang $Lang",
        "    $table = @{",
    ]
    for tag in tags:
        for key in keys:
            value = catalog[tag].get(key, catalog["en-US"][key])
            escaped = value.replace("'", "''")
            ps_lines.append(f"        '{tag}:{key}' = '{escaped}'")
    ps_lines.extend(
        [
            "    }",
            "    $template = $table[\"$normalized`:$Key\"]",
            "    if (-not $template) { $template = $table[\"en-US:$Key\"] }",
            "    if (-not $template) { return $Key }",
            r"    return [regex]::Replace($template, '\{\s*\$[A-Za-z0-9_-]+\s*\}', [System.Text.RegularExpressions.MatchEvaluator]{ param($m) $Arg })",
            "}",
            "",
        ]
    )
    return "\n".join(sh_lines) + "\n", "\n".join(ps_lines) + "\n"


def write_install_strings() -> None:
    sh_text, ps_text = render_install_strings()
    (OVERLAY_DIR.parent / "install-strings.sh").write_text(sh_text, encoding="utf-8")
    (OVERLAY_DIR.parent / "install-strings.ps1").write_text(ps_text, encoding="utf-8")
    print("wrote install-strings.sh and install-strings.ps1")


def check_hardcoded_app_js() -> list[str]:
    errors: list[str] = []
    text = APP_JS.read_text(encoding="utf-8")
    for match in _HARDCODED_COPY.finditer(text):
        line_no = text.count("\n", 0, match.start()) + 1
        snippet = text[match.start() : match.start() + 80].splitlines()[0]
        errors.append(f"{APP_JS.relative_to(ROOT)}:{line_no}: {snippet}")
    return errors


def check() -> int:
    errors: list[str] = []
    expected = en_us_ids()
    if not expected:
        errors.append(f"no message ids in {EN_DIR}")
    for tag in LOCALES:
        path = overlay_path(tag)
        if not path.is_file():
            errors.append(f"missing overlay {path.relative_to(ROOT)}")
            continue
        keys = set(load_json_overlay(tag))
        missing = sorted(expected - keys)
        extra = sorted(keys - expected)
        if missing:
            preview = ", ".join(missing[:12])
            more = f" (+{len(missing) - 12} more)" if len(missing) > 12 else ""
            errors.append(f"{tag} overlay missing {len(missing)} ids: {preview}{more}")
        if extra:
            preview = ", ".join(extra[:12])
            more = f" (+{len(extra) - 12} more)" if len(extra) > 12 else ""
            errors.append(f"{tag} overlay extra {len(extra)} ids: {preview}{more}")

    sh_text, ps_text = render_install_strings()
    sh_path = OVERLAY_DIR.parent / "install-strings.sh"
    ps_path = OVERLAY_DIR.parent / "install-strings.ps1"
    if sh_path.read_text(encoding="utf-8") != sh_text:
        errors.append(f"{sh_path.relative_to(ROOT)} is stale; run python3 ./scripts/i18n/seed_locales.py")
    if ps_path.read_text(encoding="utf-8") != ps_text:
        errors.append(f"{ps_path.relative_to(ROOT)} is stale; run python3 ./scripts/i18n/seed_locales.py")

    for hit in check_hardcoded_app_js():
        errors.append(f"hardcoded GUI copy without t(): {hit}")

    if errors:
        for item in errors:
            print(f"i18n check failed: {item}", file=sys.stderr)
        return 1
    print(
        f"i18n authoring checks passed ({len(LOCALES)} overlays, {len(expected)} message ids)."
    )
    return 0


def seed() -> None:
    files = sorted(EN_DIR.glob("*.ftl"))
    if not files:
        raise SystemExit(f"no English catalogs in {EN_DIR}")
    for tag in LOCALES:
        dest = ROOT / "locales" / tag
        dest.mkdir(parents=True, exist_ok=True)
        translations = load_json_overlay(tag)
        for src in files:
            text = src.read_text(encoding="utf-8")
            text = overlay_ftl(text, translations)
            text = text.replace("./locales/en-US/", f"./locales/{tag}/")
            (dest / src.name).write_text(text, encoding="utf-8")
        print(f"seeded {tag} ({len(translations)} overlays)")
    write_install_strings()


def main() -> None:
    if len(sys.argv) > 1 and sys.argv[1] in {"--check", "-c"}:
        raise SystemExit(check())
    if len(sys.argv) > 1:
        raise SystemExit("usage: python3 ./scripts/i18n/seed_locales.py [--check]")
    seed()


if __name__ == "__main__":
    main()
