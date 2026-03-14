# ./scripts/sync-known-versions.ps1
<#
Purpose: Regenerates the embedded known-versions catalog from an upstream pyenv clone.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/sync-known-versions.ps1 [-UpstreamRoot ..\..\pyenv]
Inputs: Optional upstream pyenv root containing plugins/python-build/share/python-build definitions.
Outputs/side effects: Writes crates/pyenv-core/data/known_versions.txt with sorted definition names.
Notes: This is a maintenance helper for syncing the local catalog seed with upstream pyenv.
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
$definitions | Set-Content -Encoding utf8 $outputPath
Write-Host "Wrote $($definitions.Count) version definitions to $outputPath"
