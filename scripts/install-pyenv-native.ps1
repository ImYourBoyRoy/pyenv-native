# ./scripts/install-pyenv-native.ps1
<#
Purpose: Installs the native pyenv executable into a portable Windows root and optionally updates PATH/profile integration.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/install-pyenv-native.ps1 [-SourcePath <pyenv.exe>] [-InstallRoot <dir>]
Inputs: Optional source binary path, install root, shell preference, PATH/profile toggles, and a force-overwrite flag.
Outputs/side effects: Copies pyenv.exe and wrappers into <InstallRoot>\bin, creates shims/versions/cache folders, optionally updates user PATH and PowerShell profile.
Notes: Keeps the install portable under a pyenv-managed root and avoids registry-based installation flows.
#>

param(
    [string]$SourcePath,
    [string]$InstallRoot = (Join-Path $HOME '.pyenv'),
    [ValidateSet('pwsh', 'cmd', 'none')]
    [string]$Shell = 'pwsh',
    [string]$AddToUserPath = 'true',
    [string]$UpdatePowerShellProfile = 'true',
    [string]$RefreshShims = 'true',
    [switch]$Force
)

$ErrorActionPreference = 'Stop'

function Resolve-SourceBinary {
    param(
        [string]$ExplicitPath
    )

    $candidates = @()
    if ($ExplicitPath) {
        $candidates += $ExplicitPath
    }

    $candidates += @(
        (Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-gnu\release\pyenv.exe'),
        (Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-gnu\debug\pyenv.exe')
    )

    foreach ($candidate in $candidates) {
        if (-not $candidate) {
            continue
        }

        $resolved = Resolve-Path $candidate -ErrorAction SilentlyContinue
        if ($resolved) {
            return $resolved.ProviderPath
        }
    }

    throw "pyenv-native source binary was not found. Pass -SourcePath <pyenv.exe> or build the project first."
}

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

function Ensure-LineInUserPath {
    param(
        [string]$Entry
    )

    $existing = [Environment]::GetEnvironmentVariable('Path', 'User')
    $segments = @()
    if ($existing) {
        $segments = $existing -split ';' | Where-Object { $_.Trim() }
    }

    if (-not ($segments | Where-Object { $_.TrimEnd('\') -ieq $Entry.TrimEnd('\') })) {
        $newPath = @($segments + $Entry) -join ';'
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
    }
}

function Write-TextFile {
    param(
        [string]$Path,
        [string]$Contents
    )

    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Path) | Out-Null
    Set-Content -Path $Path -Value $Contents -Encoding utf8
}

function Update-PowerShellProfileBlock {
    param(
        [string]$InstalledExePath
    )

    $profilePath = $PROFILE.CurrentUserCurrentHost
    $beginMarker = '# >>> pyenv-native init >>>'
    $endMarker = '# <<< pyenv-native init <<<'
    $block = @(
        $beginMarker,
        "if (Test-Path '$($InstalledExePath.Replace("'", "''"))') {",
        "  iex ((& '$($InstalledExePath.Replace("'", "''"))' init - pwsh) -join ""``n"")",
        '}',
        $endMarker
    ) -join [Environment]::NewLine

    $existing = ''
    if (Test-Path $profilePath) {
        $existing = Get-Content $profilePath -Raw
    } else {
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $profilePath) | Out-Null
    }

    $pattern = [regex]::Escape($beginMarker) + '.*?' + [regex]::Escape($endMarker)
    if ($existing -match $pattern) {
        $updated = [regex]::Replace($existing, $pattern, [System.Text.RegularExpressions.MatchEvaluator]{ param($m) $block }, 'Singleline')
    } elseif ([string]::IsNullOrWhiteSpace($existing)) {
        $updated = $block + [Environment]::NewLine
    } else {
        $updated = $existing.TrimEnd() + [Environment]::NewLine + [Environment]::NewLine + $block + [Environment]::NewLine
    }

    Set-Content -Path $profilePath -Value $updated -Encoding utf8
}

$resolvedSource = Resolve-SourceBinary -ExplicitPath $SourcePath
$resolvedInstallRoot = [System.IO.Path]::GetFullPath($InstallRoot)
$installBin = Join-Path $resolvedInstallRoot 'bin'
$installedExe = Join-Path $installBin 'pyenv.exe'
$addToUserPathValue = Convert-ToBoolean -Value $AddToUserPath -ParameterName 'AddToUserPath'
$updatePowerShellProfileValue = Convert-ToBoolean -Value $UpdatePowerShellProfile -ParameterName 'UpdatePowerShellProfile'
$refreshShimsValue = Convert-ToBoolean -Value $RefreshShims -ParameterName 'RefreshShims'

New-Item -ItemType Directory -Force -Path $installBin | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $resolvedInstallRoot 'shims') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $resolvedInstallRoot 'versions') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $resolvedInstallRoot 'cache') | Out-Null

if ((Test-Path $installedExe) -and -not $Force) {
    throw "pyenv-native is already installed at $installedExe. Re-run with -Force to overwrite."
}

Copy-Item -Force -Path $resolvedSource -Destination $installedExe

$cmdWrapper = "@echo off`r`n""%~dp0pyenv.exe"" %*`r`n"
$ps1Wrapper = "& ""$PSScriptRoot\pyenv.exe"" @args`r`nexit `$LASTEXITCODE`r`n"
$cmdInitHelper = "@echo off`r`nfor /f ""delims="" %%i in ('""%~dp0pyenv.exe"" init - cmd') do %%i`r`n"

Write-TextFile -Path (Join-Path $installBin 'pyenv.cmd') -Contents $cmdWrapper
Write-TextFile -Path (Join-Path $installBin 'pyenv.ps1') -Contents $ps1Wrapper
Write-TextFile -Path (Join-Path $installBin 'pyenv-init.cmd') -Contents $cmdInitHelper

if ($addToUserPathValue) {
    Ensure-LineInUserPath -Entry $installBin
}

if ($Shell -eq 'pwsh' -and $updatePowerShellProfileValue) {
    Update-PowerShellProfileBlock -InstalledExePath $installedExe
}

if ($refreshShimsValue) {
    & $installedExe rehash | Out-Null
}

$summary = [ordered]@{
    source_binary = $resolvedSource
    install_root = $resolvedInstallRoot
    installed_exe = $installedExe
    install_bin = $installBin
    add_to_user_path = $addToUserPathValue
    update_powershell_profile = ($Shell -eq 'pwsh' -and $updatePowerShellProfileValue)
    shell = $Shell
    refresh_shims = $refreshShimsValue
    cmd_init_helper = (Join-Path $installBin 'pyenv-init.cmd')
}

$summary.GetEnumerator() | ForEach-Object {
    '{0}: {1}' -f $_.Key, $_.Value
}

if ($Shell -eq 'cmd') {
    Write-Host ''
    Write-Host "CMD note: run `"$installBin\pyenv-init.cmd`" in each interactive CMD session, or wire it into your preferred startup flow."
}

if ($addToUserPathValue) {
    Write-Host ''
    Write-Host 'PATH note: restart your shell to pick up the updated user PATH.'
}
