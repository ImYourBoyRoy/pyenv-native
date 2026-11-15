# ./uninstall.ps1
<#
.SYNOPSIS
Remote-friendly uninstaller for Enchi (formerly pyenv-native) on Windows.

.DESCRIPTION
Runs the installed `enchi.exe self-uninstall` (or `pyenv.exe`, for installs made before the rename),
which removes the program files, shims, user PATH entries, and the PowerShell profile block it
added. This script contains no uninstall logic of its own.

.PARAMETER InstallRoot
Install to remove (default: $env:ENCHI_ROOT, else $env:PYENV_ROOT, else ~\.enchi, else an existing
pyenv-native install at ~\.pyenv).

.PARAMETER RemoveRoot
Also delete installed Pythons, virtual environments, and caches. Without it those are kept.

.PARAMETER RemoveShellProfileBlock
Pass 'false' to leave shell integration (profiles, PATH) in place.

.EXAMPLE
irm https://github.com/imyourboyroy/pyenv-native/releases/latest/download/uninstall.ps1 -OutFile $env:TEMP\enchi-uninstall.ps1; & $env:TEMP\enchi-uninstall.ps1 -RemoveRoot
#>

param(
    [string]$InstallRoot,
    [switch]$RemoveRoot,
    [string]$RemoveShellProfileBlock = 'true'
)

$ErrorActionPreference = 'Stop'

function Find-Binary {
    param([string]$Root)
    # Installs made before the rename only have pyenv.exe; upgraded ones have both.
    foreach ($name in 'enchi.exe', 'pyenv.exe') {
        $candidate = Join-Path $Root "bin\$name"
        if (Test-Path -LiteralPath $candidate) { return $candidate }
    }
    return $null
}

if (-not $InstallRoot) {
    $InstallRoot = if ($env:ENCHI_ROOT) { $env:ENCHI_ROOT } elseif ($env:PYENV_ROOT) { $env:PYENV_ROOT } else { $null }
}
if (-not $InstallRoot) {
    # The first of ~\.enchi and ~\.pyenv that holds a binary; ~\.enchi when neither does.
    $InstallRoot = Join-Path $HOME '.enchi'
    foreach ($candidate in (Join-Path $HOME '.enchi'), (Join-Path $HOME '.pyenv')) {
        if (Find-Binary $candidate) { $InstallRoot = $candidate; break }
    }
}

$enchi = Find-Binary $InstallRoot
if (-not $enchi) {
    throw "No Enchi install found at '$InstallRoot' (missing $(Join-Path $InstallRoot 'bin\enchi.exe')). Pass -InstallRoot, or remove the PATH and profile entries by hand."
}

$arguments = @('self-uninstall', '--yes')
if (-not $RemoveRoot) { $arguments += '--keep-data' }
switch ($RemoveShellProfileBlock.Trim().ToLowerInvariant()) {
    { $_ -in '0', 'false', 'no', 'off' } { $arguments += '--keep-shell-integration' }
    { $_ -in '1', 'true', 'yes', 'on' } { }
    default { throw "Invalid boolean '$RemoveShellProfileBlock' for -RemoveShellProfileBlock." }
}

# Both variables, so whichever generation of the binary runs honours the root.
$env:ENCHI_ROOT = $InstallRoot
$env:PYENV_ROOT = $InstallRoot
& $enchi @arguments
if ($LASTEXITCODE -ne 0) { throw "self-uninstall failed (exit code $LASTEXITCODE)." }
