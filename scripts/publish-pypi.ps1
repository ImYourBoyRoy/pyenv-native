# ./scripts/publish-pypi.ps1
<#
Purpose: Validates and optionally uploads the pyenv-native wheel/sdist to PyPI or TestPyPI using a repeatable local release script.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/publish-pypi.ps1 -PythonPath <python> [-Repository testpypi] [-DryRun]
Inputs: Python interpreter path, target repository/repository URL, and switches controlling local tests/build/check-only behavior.
Outputs/side effects: Optionally runs tests, rebuilds wheel/sdist artifacts, performs Twine checks, and uploads the artifacts with token-based authentication.
Notes: Intended as the final manual fallback path; the GitHub Actions release workflow remains the preferred public publish path once credentials are configured.
#>

param(
    [string]$PythonPath = 'python',
    [ValidateSet('pypi', 'testpypi')]
    [string]$Repository = 'pypi',
    [string]$RepositoryUrl,
    [switch]$SkipTests,
    [switch]$SkipBuild,
    [switch]$CheckOnly,
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'

function Write-Step {
    param([string]$Message)
    Write-Host "[publish-pypi] $Message"
}

function Format-CommandArgument {
    param([string]$Value)

    if ($null -eq $Value) {
        return "''"
    }

    if ($Value -match '[\s`"\$]') {
        return "'" + ($Value -replace "'", "''") + "'"
    }

    return $Value
}

function Invoke-PythonCommand {
    param([string[]]$Arguments)

    $rendered = ($Arguments | ForEach-Object { Format-CommandArgument -Value $_ }) -join ' '

    if ($DryRun) {
        Write-Host "DRY-RUN: $PythonPath $rendered"
        return
    }

    & $PythonPath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Python command failed with exit code ${LASTEXITCODE}: $PythonPath $rendered"
    }
}

function Ensure-TwineAvailable {
    if ($DryRun) {
        Write-Host 'DRY-RUN: python -m twine --version'
        return
    }

    & $PythonPath -m twine --version *> $null
    if ($LASTEXITCODE -ne 0) {
        throw "Twine is not available for '$PythonPath'. Install it with `$PythonPath -m pip install twine` first."
    }
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$packageDist = Join-Path $repoRoot 'python-package\dist'

if (-not $SkipTests) {
    Write-Step 'Running Python install-package tests'
    $testCommand = @('powershell', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $PSScriptRoot 'test-python-bootstrap.ps1'), '-PythonPath', $PythonPath)
    if ($DryRun) {
        Write-Host ('DRY-RUN: ' + (($testCommand | ForEach-Object { Format-CommandArgument -Value $_ }) -join ' '))
    } else {
        & $testCommand[0] @($testCommand | Select-Object -Skip 1)
        if ($LASTEXITCODE -ne 0) {
            throw "Python install-package tests failed with exit code ${LASTEXITCODE}."
        }
    }
}

if (-not $SkipBuild) {
    Write-Step 'Building Python install-package artifacts'
    $buildCommand = @('powershell', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $PSScriptRoot 'build-python-bootstrap.ps1'), '-PythonPath', $PythonPath)
    if ($DryRun) {
        Write-Host ('DRY-RUN: ' + (($buildCommand | ForEach-Object { Format-CommandArgument -Value $_ }) -join ' '))
    } else {
        & $buildCommand[0] @($buildCommand | Select-Object -Skip 1)
        if ($LASTEXITCODE -ne 0) {
            throw "Python install-package build failed with exit code ${LASTEXITCODE}."
        }
    }
}

$artifacts = @(
    Get-ChildItem -Path $packageDist -File -Filter '*.whl' -ErrorAction SilentlyContinue
    Get-ChildItem -Path $packageDist -File -Filter '*.tar.gz' -ErrorAction SilentlyContinue
) | Sort-Object FullName -Unique
if (-not $artifacts) {
    throw "No Python distribution artifacts were found under $packageDist"
}

Ensure-TwineAvailable

$artifactPaths = $artifacts.FullName
Write-Step 'Running Twine metadata checks'
Invoke-PythonCommand -Arguments (@('-m', 'twine', 'check') + $artifactPaths)

if ($CheckOnly) {
    Write-Step 'Check-only mode completed; skipping upload.'
} else {
    if (-not $DryRun -and -not $env:TWINE_PASSWORD -and $env:PYPI_API_TOKEN) {
        $env:TWINE_USERNAME = '__token__'
        $env:TWINE_PASSWORD = $env:PYPI_API_TOKEN
    }

    if (-not $DryRun -and -not $env:TWINE_PASSWORD) {
        throw 'PyPI credentials were not found. Set TWINE_PASSWORD/TWINE_USERNAME or PYPI_API_TOKEN before uploading.'
    }

    $uploadArguments = @('-m', 'twine', 'upload', '--non-interactive')
    if ($RepositoryUrl) {
        $uploadArguments += @('--repository-url', $RepositoryUrl)
    } elseif ($Repository -eq 'testpypi') {
        $uploadArguments += @('--repository', 'testpypi')
    }
    $uploadArguments += $artifactPaths

    Write-Step "Uploading artifacts to $(if ($RepositoryUrl) { $RepositoryUrl } else { $Repository })"
    Invoke-PythonCommand -Arguments $uploadArguments
}

$summary = [ordered]@{
    python_path = $PythonPath
    repository = $Repository
    repository_url = $RepositoryUrl
    artifact_count = $artifacts.Count
    skip_tests = [bool]$SkipTests
    skip_build = [bool]$SkipBuild
    check_only = [bool]$CheckOnly
    dry_run = [bool]$DryRun
}

$summary.GetEnumerator() | ForEach-Object {
    '{0}: {1}' -f $_.Key, $_.Value
}
