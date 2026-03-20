# ./scripts/build-python-bootstrap.ps1
<#
Purpose: Builds the Python install package wheel and sdist for pip/pipx distribution.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/build-python-bootstrap.ps1 -PythonPath <python.exe>
Inputs: Optional Python interpreter path and output directory override.
Outputs/side effects: Installs/updates the Python build backend for that interpreter and writes wheel/sdist artifacts under python-package/dist.
Notes: Intended for development and release packaging; requires an available Python 3.8+ interpreter.
#>

param(
    [string]$PythonPath,
    [string]$OutputRoot = (Join-Path $PSScriptRoot '..\python-package\dist')
)

$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'python-common.ps1')

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$packageRoot = Resolve-Path (Join-Path $repoRoot 'python-package')
$resolvedPython = Resolve-PythonCommandPath -ExplicitPath $PythonPath
$resolvedOutputRoot = [System.IO.Path]::GetFullPath($OutputRoot)

New-Item -ItemType Directory -Force -Path $resolvedOutputRoot | Out-Null

& $resolvedPython -m pip install --upgrade build
if ($LASTEXITCODE -ne 0) {
    throw "Failed to install Python build tooling with $resolvedPython"
}

& $resolvedPython -m build --outdir $resolvedOutputRoot $packageRoot
if ($LASTEXITCODE -ne 0) {
    throw "Python package build failed with $resolvedPython"
}

$summary = [ordered]@{
    python = $resolvedPython
    package_root = $packageRoot
    output_root = $resolvedOutputRoot
}

$summary.GetEnumerator() | ForEach-Object {
    '{0}: {1}' -f $_.Key, $_.Value
}
