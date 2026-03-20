# ./scripts/Run-Tests.ps1
<#
Purpose: Runs the full local Windows quality gate for pyenv-native, including Rust tests, Python bootstrap validation, lint checks, and shell smoke tests.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/Run-Tests.ps1 [-FullClean]
Inputs: Optional -FullClean switch to clear caches before validation.
Outputs/side effects: Executes workspace tests, may bootstrap a temporary local Python runtime when no usable interpreter is available, and removes any temporary bootstrap root before exiting.
Notes: Intended for local release validation; on Windows it can self-bootstrap a temporary Python via the local pyenv-native CLI instead of relying on the Microsoft Store alias.
#>

param(
    [switch]$FullClean
)

$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'python-common.ps1')

function Assert-LastExitCode {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Message
    )

    if ($LASTEXITCODE -ne 0) {
        $exception = [System.Exception]::new($Message)
        $exception.Data['ExitCode'] = $LASTEXITCODE
        throw $exception
    }
}

function New-BootstrapPythonFromLocalPyenv {
    $bootstrapRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("pyenv-native-bootstrap-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Force -Path $bootstrapRoot | Out-Null

    Write-Host "--- No usable system Python found; bootstrapping a temporary Python via local pyenv-native ---" -ForegroundColor Yellow

    $previousPyenvRoot = $env:PYENV_ROOT
    $hadPyenvVersion = Test-Path Env:PYENV_VERSION
    $previousPyenvVersion = $env:PYENV_VERSION

    try {
        $env:PYENV_ROOT = $bootstrapRoot
        if ($hadPyenvVersion) {
            Remove-Item Env:PYENV_VERSION -ErrorAction SilentlyContinue
        }

        $installJson = (& cargo run -q -p pyenv-cli -- install --json 3.13 2>$null | Out-String)
        Assert-LastExitCode -Message '--- Temporary Python bootstrap install FAILED ---'

        $installOutcome = $installJson | ConvertFrom-Json
        if ($installOutcome -isnot [System.Array]) {
            $installOutcome = @($installOutcome)
        }

        $pythonPath = $installOutcome[0].plan.python_executable
        if (-not $pythonPath -or -not (Test-Path $pythonPath)) {
            throw "Temporary Python bootstrap did not produce a usable interpreter under $bootstrapRoot"
        }

        return [pscustomobject]@{
            Root       = $bootstrapRoot
            PythonPath = (Resolve-Path $pythonPath).ProviderPath
        }
    }
    finally {
        if ($null -ne $previousPyenvRoot) {
            $env:PYENV_ROOT = $previousPyenvRoot
        } else {
            Remove-Item Env:PYENV_ROOT -ErrorAction SilentlyContinue
        }

        if ($hadPyenvVersion) {
            $env:PYENV_VERSION = $previousPyenvVersion
        } else {
            Remove-Item Env:PYENV_VERSION -ErrorAction SilentlyContinue
        }
    }
}

$bootstrapRoot = $null
$exitCode = 0

try {
    if ($FullClean) {
        & "$PSScriptRoot/Clear-Cache.ps1"
        Assert-LastExitCode -Message '--- Full clean FAILED ---'
    }

    Write-Host "--- Running all tests (workspace) ---" -ForegroundColor Cyan

    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        $env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
    }

    cargo test --workspace
    Assert-LastExitCode -Message '--- Tests FAILED ---'

    $resolvedPython = $null
    try {
        $resolvedPython = Resolve-PythonCommandPath
    }
    catch {
        if ($IsWindows -or $env:OS -eq 'Windows_NT') {
            $bootstrap = New-BootstrapPythonFromLocalPyenv
            $bootstrapRoot = $bootstrap.Root
            $resolvedPython = $bootstrap.PythonPath
        } else {
            Write-Host "--- Skipping Python bootstrap tests ($($_.Exception.Message)) ---" -ForegroundColor Yellow
        }
    }

    if ($null -ne $resolvedPython) {
        Write-Host "--- Running Python bootstrap tests ---" -ForegroundColor Cyan
        powershell -ExecutionPolicy Bypass -File "$PSScriptRoot/test-python-bootstrap.ps1" -PythonPath $resolvedPython
        Assert-LastExitCode -Message '--- Python bootstrap tests FAILED ---'

        Write-Host "--- Building Python bootstrap package smoke test ---" -ForegroundColor Cyan
        powershell -ExecutionPolicy Bypass -File "$PSScriptRoot/build-python-bootstrap.ps1" -PythonPath $resolvedPython
        Assert-LastExitCode -Message '--- Python bootstrap build FAILED ---'
    }

    Write-Host "--- Running lint checks ---" -ForegroundColor Cyan
    cargo fmt --check
    Assert-LastExitCode -Message '--- Formatting checks FAILED ---'

    cargo clippy --workspace -- -D warnings
    Assert-LastExitCode -Message '--- Lint checks FAILED ---'

    if ($IsWindows -or $env:OS -eq 'Windows_NT') {
        Write-Host "--- Running Windows shell smoke tests ---" -ForegroundColor Cyan
        cargo build -p pyenv-cli
        Assert-LastExitCode -Message '--- Windows smoke build FAILED ---'

        powershell -ExecutionPolicy Bypass -File "$PSScriptRoot/smoke-shells.ps1"
        Assert-LastExitCode -Message '--- Windows shell smoke tests FAILED ---'
    }

    Write-Host "--- All checks PASSED ---" -ForegroundColor Green
}
catch {
    if ($_.Exception.Message) {
        Write-Host $_.Exception.Message -ForegroundColor Red
    }

    if ($_.Exception.Data.Contains('ExitCode')) {
        $exitCode = [int]$_.Exception.Data['ExitCode']
    } elseif ($LASTEXITCODE -ne 0) {
        $exitCode = $LASTEXITCODE
    } else {
        $exitCode = 1
    }
}
finally {
    if ($bootstrapRoot -and (Test-Path $bootstrapRoot)) {
        Remove-Item -Recurse -Force $bootstrapRoot -ErrorAction SilentlyContinue
        Write-Host "--- Removed temporary Python bootstrap root: $bootstrapRoot ---" -ForegroundColor DarkGray
    }
}

if ($exitCode -ne 0) {
    exit $exitCode
}
