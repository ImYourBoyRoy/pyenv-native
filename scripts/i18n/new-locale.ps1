# ./scripts/i18n/new-locale.ps1
# Seed a new Fluent locale from en-US. Usage: pwsh -File ./scripts/i18n/new-locale.ps1 -Tag <bcp47-tag>

param(
    [Parameter(Mandatory = $true)]
    [string]$Tag
)

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if ($Tag -in @('zh-TW', 'zh-Hant', 'zh-HK', 'zh-MO')) {
    throw 'Traditional Chinese is mapped to zh-CN. Do not add a separate catalog.'
}

$Src = Join-Path $Root 'locales/en-US'
$Dest = Join-Path $Root "locales/$Tag"
if (Test-Path $Dest) {
    throw "locale $Tag already exists at $Dest"
}

New-Item -ItemType Directory -Path $Dest | Out-Null
Get-ChildItem $Src -Filter '*.ftl' | ForEach-Object {
    (Get-Content -Raw $_.FullName) -replace '\./locales/en-US/', "./locales/$Tag/" |
        Set-Content -NoNewline (Join-Path $Dest $_.Name)
}

Write-Host "Created $Dest from en-US. Add scripts/i18n/overlays/$Tag.json, list the tag in seed_locales.py LOCALES, translate, seed, then run sh ./scripts/check-i18n.sh"
