# ./scripts/uninstall-pyenv-native.ps1
<#
Purpose: Removes a portable pyenv-native Windows installation and optionally cleans PATH/profile integration.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/uninstall-pyenv-native.ps1 [-InstallRoot <dir>] [-RemoveRoot]
Inputs: Optional install root plus booleans controlling user PATH cleanup, profile cleanup, and root deletion.
Outputs/side effects: Removes installed wrappers/binary and optionally removes PATH entries, profile blocks, and the entire install root.
Notes: Avoids registry cleanup because the installer keeps pyenv-native portable and registry-free by default.
#>

param(
    [string]$InstallRoot = (Join-Path $HOME '.pyenv'),
    [string]$RemoveFromUserPath = 'true',
    [string]$RemovePowerShellProfileBlock = 'true',
    [switch]$RemoveRoot,
    [switch]$Yes
)

$ErrorActionPreference = 'Stop'

function Convert-ToBoolean {
    param(
        [string]$Value,
        [string]$ParameterName
    )

    switch ($Value.Trim().ToLowerInvariant()) {
        '1' { return $true }
        'true' { return $true }
        'yes' { return $true }
        'on' { return $true }
        '0' { return $false }
        'false' { return $false }
        'no' { return $false }
        'off' { return $false }
        default { throw "Invalid boolean value '$Value' for -$ParameterName. Use true/false, yes/no, on/off, or 1/0." }
    }
}

function Remove-LineFromUserPath {
    param(
        [string]$Entry
    )

    $existing = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not $existing) {
        return
    }

    $segments = $existing -split ';' |
        Where-Object { $_.Trim() } |
        Where-Object { $_.TrimEnd('\') -ine $Entry.TrimEnd('\') }

    [Environment]::SetEnvironmentVariable('Path', ($segments -join ';'), 'User')
}

function Remove-PowerShellProfileBlock {
    param(
        [string]$ProfilePath
    )

    if (-not (Test-Path $ProfilePath)) {
        return
    }

    $beginMarker = '# >>> pyenv-native init >>>'
    $endMarker = '# <<< pyenv-native init <<<'
    $pattern = [regex]::Escape($beginMarker) + '.*?' + [regex]::Escape($endMarker)
    $existing = Get-Content $ProfilePath -Raw
    $updated = [regex]::Replace($existing, $pattern, '', 'Singleline').Trim()

    if ([string]::IsNullOrWhiteSpace($updated)) {
        Remove-Item -Force $ProfilePath
    } else {
        Set-Content -Path $ProfilePath -Value ($updated + [Environment]::NewLine) -Encoding utf8
    }
}

$resolvedInstallRoot = [System.IO.Path]::GetFullPath($InstallRoot)
$installBin = Join-Path $resolvedInstallRoot 'bin'
$removeFromUserPathValue = Convert-ToBoolean -Value $RemoveFromUserPath -ParameterName 'RemoveFromUserPath'
$removeProfileValue = Convert-ToBoolean -Value $RemovePowerShellProfileBlock -ParameterName 'RemovePowerShellProfileBlock'

$shouldRemoveRoot = $RemoveRoot.IsPresent

if (-not $shouldRemoveRoot -and -not $Yes -and (Test-Path $resolvedInstallRoot)) {
    try {
        $answer = Read-Host "Do you want to completely wipe the pyenv root directory ('$resolvedInstallRoot') including all installed Python versions? [y/N]"
        if ($answer -and $answer.Trim() -match '^(y|yes)$') {
            $shouldRemoveRoot = $true
        }
    } catch {
        # Non-interactive or cancelled, default to false
    }
}

foreach ($path in @(
    (Join-Path $installBin 'pyenv.exe'),
    (Join-Path $installBin 'pyenv.cmd'),
    (Join-Path $installBin 'pyenv.ps1'),
    (Join-Path $installBin 'pyenv-init.cmd'),
    (Join-Path $installBin 'pyenv-mcp.exe'),
    (Join-Path $installBin 'pyenv-mcp.cmd'),
    (Join-Path $installBin 'pyenv-mcp.ps1')
)) {
    if (Test-Path $path) {
        Remove-Item -Force $path
    }
}

if ($removeFromUserPathValue) {
    Remove-LineFromUserPath -Entry $installBin
}

if ($removeProfileValue) {
    Remove-PowerShellProfileBlock -ProfilePath $PROFILE.CurrentUserCurrentHost
}

if ($shouldRemoveRoot -and (Test-Path $resolvedInstallRoot)) {
    Remove-Item -Recurse -Force $resolvedInstallRoot
}

$summary = [ordered]@{
    install_root = $resolvedInstallRoot
    install_bin = $installBin
    remove_from_user_path = $removeFromUserPathValue
    remove_powershell_profile_block = $removeProfileValue
    remove_root = $shouldRemoveRoot
}

$summary.GetEnumerator() | ForEach-Object {
    '{0}: {1}' -f $_.Key, $_.Value
}
