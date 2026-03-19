# ./scripts/set-version.ps1
<#
Purpose: Synchronizes the project version across the Rust workspace and Python install package metadata files.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/set-version.ps1 -Version 0.2.0
Inputs: Target semantic version string.
Outputs/side effects: Rewrites version fields in Cargo.toml, python-package/pyproject.toml, and python-package/src/pyenv_native_bootstrap/__init__.py.
Notes: Intended for release preparation so native and Python install-package artifacts stay aligned.
#>

param(
    [Parameter(Mandatory = $true)]
    [string]$Version
)

$ErrorActionPreference = 'Stop'

function Write-Utf8NoBomFile {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string]$Content
    )

    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Content, $utf8NoBom)
}

if ($Version -notmatch '^\d+\.\d+\.\d+([-.][0-9A-Za-z.-]+)?$') {
    throw "Invalid version '$Version'. Expected a semver-like value such as 0.2.0 or 0.2.0-rc.1."
}

function Update-FileText {
    param(
        [string]$Path,
        [string]$Pattern,
        [string]$Replacement
    )

    $content = Get-Content $Path -Raw
    $updated = [regex]::Replace($content, $Pattern, $Replacement)
    if ($updated -eq $content) {
        throw "No version match was updated in $Path"
    }
    Write-Utf8NoBomFile -Path $Path -Content $updated
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$cargoToml = Join-Path $repoRoot 'Cargo.toml'
$pyproject = Join-Path $repoRoot 'python-package\pyproject.toml'
$pythonInit = Join-Path $repoRoot 'python-package\src\pyenv_native_bootstrap\__init__.py'

Update-FileText -Path $cargoToml -Pattern '(?m)^version\s*=\s*"[^"]+"\s*$' -Replacement "version = `"$Version`""
Update-FileText -Path $pyproject -Pattern '(?m)^version\s*=\s*"[^"]+"\s*$' -Replacement "version = `"$Version`""
Update-FileText -Path $pythonInit -Pattern '(?m)^__version__\s*=\s*"[^"]+"\s*$' -Replacement "__version__ = `"$Version`""

$summary = [ordered]@{
    version = $Version
    cargo_toml = $cargoToml
    pyproject = $pyproject
    python_init = $pythonInit
}

$summary.GetEnumerator() | ForEach-Object {
    '{0}: {1}' -f $_.Key, $_.Value
}
