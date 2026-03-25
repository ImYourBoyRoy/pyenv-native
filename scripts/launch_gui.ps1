# ./scripts/launch_gui.ps1
# Convenience script to compile and launch the pyenv-native GUI for local testing.
#
# Usage:
#   .\scripts\launch_gui.ps1
#
# This will:
#   1. Stop any running pyenv-gui process
#   2. Build the pyenv-gui crate in debug mode
#   3. Launch the GUI application

param(
    [switch]$Release
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

# Resolve to the repo root (handle running from scripts/ or repo root)
if (Test-Path (Join-Path $PSScriptRoot "..\Cargo.toml")) {
    $ProjectRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
} elseif (Test-Path (Join-Path $PSScriptRoot "Cargo.toml")) {
    $ProjectRoot = Resolve-Path $PSScriptRoot
} else {
    $ProjectRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
}

Write-Host ""
Write-Host "  Pyenv-Native GUI Launcher" -ForegroundColor Cyan
Write-Host "  =========================" -ForegroundColor DarkCyan
Write-Host ""

# Kill any existing instance
$existing = Get-Process -Name "pyenv-gui" -ErrorAction SilentlyContinue
if ($existing) {
    Write-Host "  [*] Stopping existing pyenv-gui process..." -ForegroundColor Yellow
    Stop-Process -Name "pyenv-gui" -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500
}

# Ensure cargo is available
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH

# Build
$buildMode = if ($Release) { "--release" } else { "" }
$modeLabel = if ($Release) { "release" } else { "debug" }

Write-Host "  [1/2] Building pyenv-gui ($modeLabel)..." -ForegroundColor Cyan

if ($Release) {
    cargo build -p pyenv-gui --release
} else {
    cargo build -p pyenv-gui
}

if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "  [!] Build failed. See errors above." -ForegroundColor Red
    exit 1
}

Write-Host "  [2/2] Launching..." -ForegroundColor Green

$exePath = if ($Release) {
    Join-Path $ProjectRoot "target\release\pyenv-gui.exe"
} else {
    Join-Path $ProjectRoot "target\debug\pyenv-gui.exe"
}

Start-Process -FilePath $exePath
Write-Host ""
Write-Host "  GUI is running. Close the window to exit." -ForegroundColor Green
Write-Host ""
