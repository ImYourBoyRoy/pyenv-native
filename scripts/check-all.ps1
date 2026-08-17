# ./scripts/check-all.ps1
# Local + pre-push quality gate for Windows: format, clippy, tests, Python package, GUI a11y, optional cargo-audit.
param()

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

function Assert-LastExitCode {
    param([string]$Message)
    if ($LASTEXITCODE -ne 0) { throw $Message }
}

Write-Host '--- Format ---' -ForegroundColor Cyan
cargo fmt --check
Assert-LastExitCode 'cargo fmt failed'

Write-Host '--- Clippy (workspace) ---' -ForegroundColor Cyan
cargo clippy --workspace -- -D warnings
Assert-LastExitCode 'clippy failed'

Write-Host '--- Tests (workspace) ---' -ForegroundColor Cyan
cargo test --workspace
Assert-LastExitCode 'cargo test failed'

Write-Host '--- Version sync ---' -ForegroundColor Cyan
pwsh -NoLogo -NoProfile -File "$PSScriptRoot/check-version-sync.ps1"
Assert-LastExitCode 'version sync failed'

Write-Host '--- GUI patterns ---' -ForegroundColor Cyan
if (Get-Command sh -ErrorAction SilentlyContinue) {
    sh "$PSScriptRoot/check-gui-patterns.sh"
} elseif (Get-Command bash -ErrorAction SilentlyContinue) {
    bash "$PSScriptRoot/check-gui-patterns.sh"
} else {
    throw 'sh or bash is required for GUI pattern checks'
}
Assert-LastExitCode 'GUI pattern check failed'

Write-Host '--- Python bootstrap tests ---' -ForegroundColor Cyan
pwsh -NoLogo -NoProfile -File "$PSScriptRoot/test-python-bootstrap.ps1"
Assert-LastExitCode 'python bootstrap tests failed'

if (Get-Command pnpm -ErrorAction SilentlyContinue) {
    Write-Host '--- GUI a11y (pnpm) ---' -ForegroundColor Cyan
    Push-Location "$PSScriptRoot/gui-a11y"
    pnpm install --frozen-lockfile
    Assert-LastExitCode 'pnpm install failed'
    pnpm audit --prod
    Assert-LastExitCode 'pnpm audit failed'
    pnpm run check
    Assert-LastExitCode 'a11y check failed'
    Pop-Location
} else {
    throw 'pnpm is required for GUI a11y checks (corepack enable; corepack prepare pnpm@latest --activate)'
}

if (Get-Command cargo-audit -ErrorAction SilentlyContinue) {
    Write-Host '--- cargo audit ---' -ForegroundColor Cyan
    cargo audit
    Assert-LastExitCode 'cargo audit failed'
} else {
    Write-Host '--- Skipping cargo audit (cargo install cargo-audit --locked) ---' -ForegroundColor Yellow
}

Write-Host '--- All checks PASSED ---' -ForegroundColor Green
