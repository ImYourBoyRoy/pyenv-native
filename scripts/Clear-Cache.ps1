# ./scripts/Clear-Cache.ps1
# Maintenance script to clear project caches and temporary files.
# Use this to reclaim disk space (e.g. from /target/) and ensure a clean environment.

$ErrorActionPreference = 'SilentlyContinue'

Write-Host "--- pyenv-native cache cleanup ---" -ForegroundColor Cyan

$Targets = @(
    "target",
    "dist",
    "python-package/dist",
    "test-install",
    ".tmp*",
    "**/__pycache__",
    "*.pdb",
    "repro_*.rs",
    "temp_*.rs"
)

foreach ($Pattern in $Targets) {
    $Items = Get-ChildItem -Path $PSScriptRoot\.. -Filter $Pattern -Recurse -File -Directory
    if ($Items) {
        foreach ($Item in $Items) {
            Write-Host "Removing: $($Item.FullName)" -ForegroundColor Gray
            Remove-Item -Path $Item.FullName -Recurse -Force
        }
    }
}

Write-Host "--- Cleanup complete ---" -ForegroundColor Green
