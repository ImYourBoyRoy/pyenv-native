# ./uninstall.ps1
<#
.SYNOPSIS
Remote-friendly uninstaller for pyenv-native on Windows.

.DESCRIPTION
Runs the installed `pyenv.exe self-uninstall`, which removes the program files, shims, user PATH
entries, and the PowerShell profile block it added. This script contains no uninstall logic of its
own.

.PARAMETER InstallRoot
Install to remove (default: $env:PYENV_ROOT or ~\.pyenv).

.PARAMETER RemoveRoot
Also delete installed Pythons, virtual environments, and caches. Without it those are kept.

.PARAMETER RemoveShellProfileBlock
Pass 'false' to leave shell integration (profiles, PATH) in place.

.EXAMPLE
irm https://github.com/imyourboyroy/pyenv-native/releases/latest/download/uninstall.ps1 -OutFile $env:TEMP\pyenv-uninstall.ps1; & $env:TEMP\pyenv-uninstall.ps1 -RemoveRoot
#>

param(
    [string]$InstallRoot = $(if ($env:PYENV_ROOT) { $env:PYENV_ROOT } else { Join-Path $HOME '.pyenv' }),
    [switch]$RemoveRoot,
    [string]$RemoveShellProfileBlock = 'true'
)

$ErrorActionPreference = 'Stop'

$pyenv = Join-Path $InstallRoot 'bin\pyenv.exe'
if (-not (Test-Path -LiteralPath $pyenv)) {
    throw "No pyenv-native install found at '$InstallRoot' (missing $pyenv). Pass -InstallRoot, or remove the PATH and profile entries by hand."
}

$arguments = @('self-uninstall', '--yes')
if (-not $RemoveRoot) { $arguments += '--keep-data' }
switch ($RemoveShellProfileBlock.Trim().ToLowerInvariant()) {
    { $_ -in '0', 'false', 'no', 'off' } { $arguments += '--keep-shell-integration' }
    { $_ -in '1', 'true', 'yes', 'on' } { }
    default { throw "Invalid boolean '$RemoveShellProfileBlock' for -RemoveShellProfileBlock." }
}

$env:PYENV_ROOT = $InstallRoot
& $pyenv @arguments
if ($LASTEXITCODE -ne 0) { throw "pyenv self-uninstall failed (exit code $LASTEXITCODE)." }
