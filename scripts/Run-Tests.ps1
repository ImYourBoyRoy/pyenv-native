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

Write-Host "--- Running lint checks ---" -ForegroundColor Cyan
cargo fmt --check
cargo clippy --workspace -- -D warnings

if ($LASTEXITCODE -ne 0) {
    Write-Host "--- Lint checks FAILED ---" -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host "--- All checks PASSED ---" -ForegroundColor Green
