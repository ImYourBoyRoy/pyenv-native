# ./scripts/Check-Env.ps1
# Environment check script to verify developer prerequisites.
# Checks for Rust, Cargo, and essential environmental variables.

Write-Host "--- pyenv-native dev environment check ---" -ForegroundColor Cyan

$Success = $true

# 1. Check for Rust / Cargo
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    $CargoVersion = cargo --version
    Write-Host "[OK] Cargo found: $CargoVersion" -ForegroundColor Green
} else {
    Write-Host "[FAIL] Cargo NOT found. Please install Rust from https://rustup.rs" -ForegroundColor Red
    $Success = $false
}

# 2. Check for rustc
if (Get-Command rustc -ErrorAction SilentlyContinue) {
    $RustcVersion = rustc --version
    Write-Host "[OK] Rustc found: $RustcVersion" -ForegroundColor Green
} else {
    Write-Host "[FAIL] Rustc NOT found." -ForegroundColor Red
    $Success = $false
}

# 3. Check for PowerShell version
$PSVersion = $PSVersionTable.PSVersion
if ($PSVersion.Major -ge 5) {
    Write-Host "[OK] PowerShell version: $PSVersion" -ForegroundColor Green
} else {
    Write-Host "[WARN] PowerShell version is old ($PSVersion). 5.1 or 7+ recommended." -ForegroundColor Yellow
}

# 4. Check for pyenv-native shims/bin on path (optional/nice to have)
if (Get-Command pyenv -ErrorAction SilentlyContinue) {
    $PyenvPath = (Get-Command pyenv).Source
    Write-Host "[INFO] pyenv found at: $PyenvPath" -ForegroundColor Gray
} else {
    Write-Host "[INFO] pyenv not currently on system PATH. This is expected if you haven't installed it yet." -ForegroundColor Gray
}

if ($Success) {
    Write-Host "--- Environment look READY for development ---" -ForegroundColor Green
} else {
    Write-Host "--- Environment is MISSING prerequisites ---" -ForegroundColor Red
    exit 1
}
