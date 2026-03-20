# ./scripts/test-python-bootstrap.ps1
<#
Purpose: Runs the Python install package unit tests with a chosen Python interpreter.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/test-python-bootstrap.ps1 -PythonPath <python.exe>
Inputs: Optional Python interpreter path.
Outputs/side effects: Executes stdlib unittest discovery against python-package/tests with PYTHONPATH set to python-package/src.
Notes: Intended for local validation and CI; requires Python 3.8+.
#>

param(
    [string]$PythonPath
)

$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'python-common.ps1')

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$packageRoot = Resolve-Path (Join-Path $repoRoot 'python-package')
$resolvedPython = Resolve-PythonCommandPath -ExplicitPath $PythonPath
$previousPythonPath = $env:PYTHONPATH
$env:PYTHONPATH = (Resolve-Path (Join-Path $packageRoot 'src')).ProviderPath

Get-ChildItem -Path $packageRoot -Recurse -Directory -Force |
    Where-Object { $_.Name -in @('__pycache__', '.pytest_cache', '.mypy_cache') } |
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
Get-ChildItem -Path $packageRoot -Recurse -File -Force |
    Where-Object { $_.Extension -eq '.pyc' } |
    Remove-Item -Force -ErrorAction SilentlyContinue

Push-Location $packageRoot
try {
    & $resolvedPython -m unittest discover -s tests -v
    exit $LASTEXITCODE
}
finally {
    Pop-Location
    $env:PYTHONPATH = $previousPythonPath
}
