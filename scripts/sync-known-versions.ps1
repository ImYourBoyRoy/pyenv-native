# ./scripts/sync-known-versions.ps1
<#
Purpose: Regenerates the embedded known-versions catalog from an upstream pyenv clone.
How to run: pwsh -NoLogo -NoProfile -File ./scripts/sync-known-versions.ps1 [-UpstreamRoot ..\..\pyenv]
Inputs: Optional upstream pyenv root containing plugins/python-build/share/python-build definitions.
Outputs/side effects: Writes crates/pyenv-core/data/known_versions.txt with sorted definition names.
Notes: This refreshes the offline seed used when python.org cannot be reached, plus non-CPython family names. Installable CPython revisions are discovered at runtime from https://www.python.org/ftp/python/ and cached; do not treat this file as the live catalog.
#>

param(
    [string]$UpstreamRoot = (Join-Path $PSScriptRoot '..\..\pyenv')
)

$ErrorActionPreference = 'Stop'

$resolvedUpstream = Resolve-Path $UpstreamRoot
$definitionsDir = Join-Path $resolvedUpstream 'plugins\python-build\share\python-build'
if (-not (Test-Path $definitionsDir)) {
    throw "python-build definitions directory not found at $definitionsDir"
}

$outputPath = Join-Path $PSScriptRoot '..\crates\pyenv-core\data\known_versions.txt'
$definitions = Get-ChildItem $definitionsDir -File | Select-Object -ExpandProperty Name | Sort-Object
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $outputPath) | Out-Null
$payload = ($definitions -join "`n") + "`n"
$utf8NoBom = New-Object System.Text.UTF8Encoding $false
[System.IO.File]::WriteAllText($outputPath, $payload, $utf8NoBom)
Write-Host "Wrote $($definitions.Count) version definitions to $outputPath"
