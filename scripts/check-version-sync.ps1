# ./scripts/check-version-sync.ps1
<#
Purpose: Fail CI/release when Cargo workspace, Python package, and bootstrap __version__ diverge.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/check-version-sync.ps1
            powershell -ExecutionPolicy Bypass -File ./scripts/check-version-sync.ps1 -ExpectedVersion 0.2.34
Inputs: Cargo.toml, python-package/pyproject.toml, python-package/src/pyenv_native_bootstrap/__init__.py;
        optional -ExpectedVersion (or EXPECTED_VERSION env) for tag release gates.
Outputs/side effects: Prints the shared version; throws on mismatch.
Notes: Keep in sync with scripts/set-version.ps1 and scripts/check-version-sync.sh.
#>

param(
    [string]$ExpectedVersion = $env:EXPECTED_VERSION
)

$ErrorActionPreference = 'Stop'

function Get-FirstMatchValue {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Pattern,
        [Parameter(Mandatory = $true)][string]$Label
    )

    $content = Get-Content -LiteralPath $Path -Raw
    $match = [regex]::Match($content, $Pattern, [System.Text.RegularExpressions.RegexOptions]::Multiline)
    if (-not $match.Success -or [string]::IsNullOrWhiteSpace($match.Groups[1].Value)) {
        throw "Could not read $Label from $Path"
    }
    return $match.Groups[1].Value.Trim()
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$cargoToml = Join-Path $repoRoot 'Cargo.toml'
$pyproject = Join-Path $repoRoot 'python-package\pyproject.toml'
$pythonInit = Join-Path $repoRoot 'python-package\src\pyenv_native_bootstrap\__init__.py'

$cargoVersion = Get-FirstMatchValue -Path $cargoToml -Pattern '(?m)^version\s*=\s*"([^"]+)"\s*$' -Label 'Cargo.toml version'
$pyVersion = Get-FirstMatchValue -Path $pyproject -Pattern '(?m)^version\s*=\s*"([^"]+)"\s*$' -Label 'pyproject.toml version'
$initVersion = Get-FirstMatchValue -Path $pythonInit -Pattern '(?m)^__version__\s*=\s*"([^"]+)"\s*$' -Label '__version__'

if ($cargoVersion -ne $pyVersion -or $cargoVersion -ne $initVersion) {
    throw @"
Version mismatch across release metadata:
  Cargo.toml:           $cargoVersion
  python-package/pyproject.toml: $pyVersion
  __init__.__version__: $initVersion
Fix with: powershell -ExecutionPolicy Bypass -File ./scripts/set-version.ps1 -Version <semver>
"@
}

if (-not [string]::IsNullOrWhiteSpace($ExpectedVersion) -and $cargoVersion -ne $ExpectedVersion) {
    throw "Workspace version $cargoVersion does not match expected $ExpectedVersion"
}

Write-Host "Version sync OK: $cargoVersion"
