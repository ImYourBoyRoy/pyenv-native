# ./scripts/Run-Tests.ps1
# Maintenance script to run all project tests cleanly.
# Optionally clears the cache first to ensure a fresh build.

param(
    [switch]$FullClean
)

if ($FullClean) {
    & "$PSScriptRoot/Clear-Cache.ps1"
}

Write-Host "--- Running all tests (workspace) ---" -ForegroundColor Cyan

# Ensure cargo is in PATH if not already (standard fallback)
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
}

cargo test --workspace

if ($LASTEXITCODE -ne 0) {
    Write-Host "--- Tests FAILED ---" -ForegroundColor Red
    exit $LASTEXITCODE
}

$pythonCommand = Get-Command python -ErrorAction SilentlyContinue
if ($null -ne $pythonCommand) {
    Write-Host "--- Running Python bootstrap tests ---" -ForegroundColor Cyan
    powershell -ExecutionPolicy Bypass -File "$PSScriptRoot/test-python-bootstrap.ps1" -PythonPath python
    if ($LASTEXITCODE -ne 0) {
        Write-Host "--- Python bootstrap tests FAILED ---" -ForegroundColor Red
        exit $LASTEXITCODE
    }

    Write-Host "--- Building Python bootstrap package smoke test ---" -ForegroundColor Cyan
    powershell -ExecutionPolicy Bypass -File "$PSScriptRoot/build-python-bootstrap.ps1" -PythonPath python
    if ($LASTEXITCODE -ne 0) {
        Write-Host "--- Python bootstrap build FAILED ---" -ForegroundColor Red
        exit $LASTEXITCODE
    }
} else {
    Write-Host "--- Skipping Python bootstrap tests (python not found on PATH) ---" -ForegroundColor Yellow
}

Write-Host "--- Running lint checks ---" -ForegroundColor Cyan
cargo fmt --check
cargo clippy --workspace -- -D warnings

if ($LASTEXITCODE -ne 0) {
    Write-Host "--- Lint checks FAILED ---" -ForegroundColor Red
    exit $LASTEXITCODE
}

if ($IsWindows -or $env:OS -eq 'Windows_NT') {
    Write-Host "--- Running Windows shell smoke tests ---" -ForegroundColor Cyan
    cargo build -p pyenv-cli
    if ($LASTEXITCODE -ne 0) {
        Write-Host "--- Windows smoke build FAILED ---" -ForegroundColor Red
        exit $LASTEXITCODE
    }

    powershell -ExecutionPolicy Bypass -File "$PSScriptRoot/smoke-shells.ps1"
    if ($LASTEXITCODE -ne 0) {
        Write-Host "--- Windows shell smoke tests FAILED ---" -ForegroundColor Red
        exit $LASTEXITCODE
    }
}

Write-Host "--- All checks PASSED ---" -ForegroundColor Green
