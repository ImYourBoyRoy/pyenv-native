# ./scripts/Clear-Cache.ps1
# Maintenance script to clear project caches and temporary files.
# Use this to reclaim disk space (e.g. from /target/) and ensure a clean environment.

$ErrorActionPreference = 'SilentlyContinue'

Write-Host "--- pyenv-native cache cleanup ---" -ForegroundColor Cyan

# 1. Use cargo clean if available (most reliable for /target)
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Write-Host "Running cargo clean..." -ForegroundColor Gray
    cargo clean
}

# 2. Define targets relative to project root
$ProjectRoot = Resolve-Path "$PSScriptRoot\.."
$Targets = @(
    "target",
    "dist",
    "python-package/dist",
    "test-install",
    "temp",
    ".tmp*"
)

foreach ($Target in $Targets) {
    $Path = Join-Path $ProjectRoot $Target
    if (Test-Path $Path) {
        Write-Host "Removing: $Path" -ForegroundColor Gray
        Remove-Item -Path $Path -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# 3. Recursive cleanup for common junk
$JunkPatterns = @(
    "**/__pycache__",
    "*.pdb",
    "repro_*.rs",
    "temp_*.rs",
    "clippy_errors.txt"
)

foreach ($Pattern in $JunkPatterns) {
    $Items = Get-ChildItem -Path $ProjectRoot -Filter $Pattern -Recurse -File -Directory -ErrorAction SilentlyContinue
    if ($Items) {
        foreach ($Item in $Items) {
            Write-Host "Removing: $($Item.FullName)" -ForegroundColor Gray
            Remove-Item -Path $Item.FullName -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

Write-Host "--- Cleanup complete ---" -ForegroundColor Green
