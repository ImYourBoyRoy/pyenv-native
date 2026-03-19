# ./scripts/install-pyenv-native.ps1
<#
Purpose: Installs the native pyenv executables into a portable Windows root and optionally updates PATH/profile integration.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/install-pyenv-native.ps1 [-SourcePath <pyenv.exe>] [-SourceMcpPath <pyenv-mcp.exe>] [-InstallRoot <dir>] [-Yes]
Inputs: Optional source binary paths, install root, shell preference, PATH/profile toggles, logging location, and a force-overwrite flag.
Outputs/side effects: Copies pyenv.exe plus pyenv-mcp.exe into <InstallRoot>\bin, creates shims/versions/cache folders, optionally updates user PATH and PowerShell profile, and writes an install log.
Notes: Keeps the install portable under a pyenv-managed root, avoids registry-based installation flows, and performs post-install sanity checks.
#>

param(
    [string]$SourcePath,
    [string]$SourceMcpPath,
    [string]$InstallRoot = (Join-Path $HOME '.pyenv'),
    [ValidateSet('pwsh', 'cmd', 'none')]
    [string]$Shell = 'pwsh',
    [string]$AddToUserPath = 'true',
    [string]$UpdatePowerShellProfile = 'true',
    [string]$RefreshShims = 'true',
    [string]$LogPath,
    [switch]$Force,
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

function Resolve-SourceBinary {
    param(
        [string]$ExplicitPath,
        [string]$BinaryName,
        [string[]]$FallbackCandidates,
        [bool]$Required = $true
    )

    $candidates = @()
    if ($ExplicitPath) {
        $candidates += $ExplicitPath
    }
    $candidates += $FallbackCandidates

    foreach ($candidate in $candidates) {
        if (-not $candidate) {
            continue
        }

        $resolved = Resolve-Path $candidate -ErrorAction SilentlyContinue
        if ($resolved) {
            return $resolved.ProviderPath
        }
    }

    if ($Required) {
        throw "pyenv-native source binary '$BinaryName' was not found. Pass an explicit path or build the project first."
    }

    return $null
}

function Resolve-OptionalMcpBinary {
    param(
        [string]$ExplicitPath,
        [string]$ResolvedPyenvBinary
    )

    $fallbackCandidates = @()
    if ($ResolvedPyenvBinary) {
        $fallbackCandidates += Join-Path (Split-Path -Parent $ResolvedPyenvBinary) 'pyenv-mcp.exe'
    }
    $fallbackCandidates += @(
        (Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-gnu\release\pyenv-mcp.exe'),
        (Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-gnu\debug\pyenv-mcp.exe'),
        (Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-msvc\release\pyenv-mcp.exe'),
        (Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-msvc\debug\pyenv-mcp.exe'),
        (Join-Path $PSScriptRoot '..\target\aarch64-pc-windows-msvc\release\pyenv-mcp.exe'),
        (Join-Path $PSScriptRoot '..\target\aarch64-pc-windows-msvc\debug\pyenv-mcp.exe')
    )

    return Resolve-SourceBinary -ExplicitPath $ExplicitPath -BinaryName 'pyenv-mcp.exe' -FallbackCandidates $fallbackCandidates -Required:$false
}

function Get-NearestExistingDirectory {
    param(
        [string]$Path
    )

    $candidate = [System.IO.Path]::GetFullPath($Path)
    while (-not (Test-Path $candidate)) {
        $parent = Split-Path -Parent $candidate
        if (-not $parent -or $parent -eq $candidate) {
            break
        }
        $candidate = $parent
    }

    if (Test-Path $candidate -PathType Leaf) {
        return Split-Path -Parent $candidate
    }

    return $candidate
}

function Test-DirectoryWritable {
    param(
        [string]$DirectoryPath
    )

    $probePath = Join-Path $DirectoryPath ('.pyenv-native-write-test-' + [guid]::NewGuid().ToString('N'))
    try {
        Set-Content -Path $probePath -Value '' -Encoding utf8 -ErrorAction Stop
        Remove-Item $probePath -Force -ErrorAction SilentlyContinue
        return $true
    } catch {
        return $false
    }
}

function Test-IsAdministrator {
    try {
        $currentIdentity = [Security.Principal.WindowsIdentity]::GetCurrent()
        $principal = [Security.Principal.WindowsPrincipal]::new($currentIdentity)
        return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    } catch {
        return $false
    }
}

function Assert-InstallRootAccess {
    param(
        [string]$ResolvedInstallRoot
    )

    $anchor = Get-NearestExistingDirectory -Path (Split-Path -Parent $ResolvedInstallRoot)
    if (Test-DirectoryWritable -DirectoryPath $anchor) {
        return
    }

    if (Test-IsAdministrator) {
        throw "Install root '$ResolvedInstallRoot' is not writable even in the current elevated session. Choose a different -InstallRoot."
    }

    throw "Install root '$ResolvedInstallRoot' requires elevated permissions. Re-run from an elevated PowerShell session or choose a user-writable -InstallRoot."
}

function Assert-InstallRootState {
    param(
        [string]$ResolvedInstallRoot,
        [string]$InstalledExe,
        [bool]$Overwrite
    )

    if ((Test-Path $InstalledExe) -and -not $Overwrite) {
        Write-Warning "pyenv-native is already installed at $InstalledExe. Proceeding will upgrade or overwrite the installation in-place."
    }

    if ((Test-Path $ResolvedInstallRoot) -and -not (Test-Path $InstalledExe) -and -not $Overwrite) {
        $children = @(Get-ChildItem -Force $ResolvedInstallRoot -ErrorAction SilentlyContinue)
        if ($children.Count -gt 0) {
            $nonLogChildren = @($children | Where-Object { $_.Name -ne 'logs' })
            if ($nonLogChildren.Count -gt 0) {
                Write-Warning "Install root '$ResolvedInstallRoot' already exists and is not empty. Proceeding will install into this existing directory."
            }
        }
    }
}

function Test-ExistingPathCommand {
    param(
        [string]$ResolvedInstallRoot
    )

    $existing = Get-Command pyenv -ErrorAction SilentlyContinue
    if (-not $existing -or -not $existing.Source) {
        return
    }

    $expectedPrefix = (Join-Path $ResolvedInstallRoot 'bin').TrimEnd('\')
    if ($existing.Source.Trim() -notlike "$expectedPrefix*") {
        Write-Warning "A different pyenv command is already discoverable at '$($existing.Source)'. Restart shells after install and verify PATH ordering."
    }
}

function Initialize-InstallLog {
    param(
        [string]$ResolvedLogPath
    )

    $directory = Split-Path -Parent $ResolvedLogPath
    if ($directory) {
        New-Item -ItemType Directory -Force -Path $directory | Out-Null
    }
    Set-Content -Path $ResolvedLogPath -Value '' -Encoding utf8
}

function Write-InstallLog {
    param(
        [string]$Level,
        [string]$Message,
        [string]$ResolvedLogPath
    )

    $line = "[pyenv-native][$Level] $Message"
    Write-Host $line
    Add-Content -Path $ResolvedLogPath -Value $line -Encoding utf8
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
    $escapedInstalledExePath = $InstalledExePath.Replace("'", "''")
    $beginMarker = '# >>> pyenv-native init >>>'
    $endMarker = '# <<< pyenv-native init <<<'
    $block = @(
        $beginMarker,
        "if (Test-Path '$escapedInstalledExePath') {",
        "  iex ((& '$escapedInstalledExePath' init - pwsh) -join ""`n"")",
        '}',
        $endMarker
    ) -join [Environment]::NewLine

    if (Test-Path $profilePath) {
        $existing = Get-Content $profilePath -Raw
    } else {
        $existing = ''
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
    return $profilePath
}

function Emit-Summary {
    param(
        [hashtable]$Summary
    )

    Write-Host ''
    Write-Host 'pyenv-native install summary'
    Write-Host '============================'
    foreach ($entry in $Summary.GetEnumerator()) {
        Write-Host ('{0,-18}: {1}' -f $entry.Key, $entry.Value)
    }
    Write-Host ''
    Write-Host 'This will create or update a portable pyenv-native installation under the selected root.'
    Write-Host 'It installs pyenv plus the agent-friendly pyenv-mcp server when available, writes an install log, and runs basic sanity checks.'
    if ($Summary.update_profile -eq $true) {
        Write-Host 'Your PowerShell profile will be updated so future sessions can find pyenv-native automatically.'
    } else {
        Write-Host 'No PowerShell profile changes will be made.'
    }
    Write-Host ''
}

function Confirm-Install {
    if ($Yes -or $Force.IsPresent) {
        return
    }

    try {
        $answer = Read-Host 'Continue with install? [y/N]'
    } catch {
        throw 'Confirmation is required for interactive installs. Re-run with -Yes for non-interactive use.'
    }

    if ($null -eq $answer -or $answer.Trim().Length -eq 0) {
        throw 'Install cancelled.'
    }

    switch ($answer.Trim().ToLowerInvariant()) {
        'y' { return }
        'yes' { return }
        default { throw 'Install cancelled.' }
    }
}

function Invoke-BinarySanityCheck {
    param(
        [string]$CommandPath,
        [string]$ResolvedInstallRoot,
        [string]$Name,
        [string[]]$Arguments,
        [string]$ResolvedLogPath
    )

    $previousRoot = $env:PYENV_ROOT
    $output = $null
    $exitCode = 0

    try {
        $env:PYENV_ROOT = $ResolvedInstallRoot
        $output = & $CommandPath @Arguments 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        if ($null -eq $previousRoot) {
            Remove-Item Env:PYENV_ROOT -ErrorAction SilentlyContinue
        } else {
            $env:PYENV_ROOT = $previousRoot
        }
    }

    if ($exitCode -ne 0) {
        $text = ($output | Out-String).Trim()
        Write-InstallLog -Level 'ERROR' -Message "Sanity check failed: $Name" -ResolvedLogPath $ResolvedLogPath
        if ($text) {
            Add-Content -Path $ResolvedLogPath -Value $text -Encoding utf8
            Write-Host $text
        }
        throw "Sanity check failed: $Name"
    }

    $firstLine = (($output | Out-String).Trim() -split "`r?`n" | Select-Object -First 1)
    if ($firstLine) {
        Write-InstallLog -Level 'INFO' -Message "Sanity check passed: $Name -> $firstLine" -ResolvedLogPath $ResolvedLogPath
    } else {
        Write-InstallLog -Level 'INFO' -Message "Sanity check passed: $Name" -ResolvedLogPath $ResolvedLogPath
    }
}

$resolvedSource = Resolve-SourceBinary -ExplicitPath $SourcePath -BinaryName 'pyenv.exe' -FallbackCandidates @(
    (Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-gnu\release\pyenv.exe'),
    (Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-gnu\debug\pyenv.exe'),
    (Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-msvc\release\pyenv.exe'),
    (Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-msvc\debug\pyenv.exe'),
    (Join-Path $PSScriptRoot '..\target\aarch64-pc-windows-msvc\release\pyenv.exe'),
    (Join-Path $PSScriptRoot '..\target\aarch64-pc-windows-msvc\debug\pyenv.exe')
)
$resolvedMcpSource = Resolve-OptionalMcpBinary -ExplicitPath $SourceMcpPath -ResolvedPyenvBinary $resolvedSource
$resolvedInstallRoot = [System.IO.Path]::GetFullPath($InstallRoot)
$installBin = Join-Path $resolvedInstallRoot 'bin'
$installedExe = Join-Path $installBin 'pyenv.exe'
$installedMcpExe = Join-Path $installBin 'pyenv-mcp.exe'
$addToUserPathValue = Convert-ToBoolean -Value $AddToUserPath -ParameterName 'AddToUserPath'
$updatePowerShellProfileValue = Convert-ToBoolean -Value $UpdatePowerShellProfile -ParameterName 'UpdatePowerShellProfile'
$refreshShimsValue = Convert-ToBoolean -Value $RefreshShims -ParameterName 'RefreshShims'
$updateProfileEffective = ($Shell -eq 'pwsh' -and $updatePowerShellProfileValue)

if ([string]::IsNullOrWhiteSpace($LogPath)) {
    $timestamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $LogPath = Join-Path (Join-Path $resolvedInstallRoot 'logs') "install-$timestamp.log"
}
$resolvedLogPath = [System.IO.Path]::GetFullPath($LogPath)

Test-ExistingPathCommand -ResolvedInstallRoot $resolvedInstallRoot
Assert-InstallRootAccess -ResolvedInstallRoot $resolvedInstallRoot
Assert-InstallRootState -ResolvedInstallRoot $resolvedInstallRoot -InstalledExe $installedExe -Overwrite $Force.IsPresent

$summary = [ordered]@{
    source_binary = $resolvedSource
    source_mcp = $(if ($resolvedMcpSource) { $resolvedMcpSource } else { '<not found>' })
    install_root = $resolvedInstallRoot
    installed_exe = $installedExe
    installed_mcp = $installedMcpExe
    shell = $Shell
    add_to_path = $addToUserPathValue
    update_profile = $updateProfileEffective
    refresh_shims = $refreshShimsValue
    force = $Force.IsPresent
    log_path = $resolvedLogPath
}
Emit-Summary -Summary $summary
Confirm-Install
Initialize-InstallLog -ResolvedLogPath $resolvedLogPath
Write-InstallLog -Level 'INFO' -Message 'Starting portable pyenv-native install.' -ResolvedLogPath $resolvedLogPath

New-Item -ItemType Directory -Force -Path $installBin | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $resolvedInstallRoot 'shims') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $resolvedInstallRoot 'versions') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $resolvedInstallRoot 'cache') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $resolvedInstallRoot 'logs') | Out-Null

Copy-Item -Force -Path $resolvedSource -Destination $installedExe
if ($resolvedMcpSource) {
    Copy-Item -Force -Path $resolvedMcpSource -Destination $installedMcpExe
    Write-InstallLog -Level 'INFO' -Message "Installed MCP server binary into $installedMcpExe" -ResolvedLogPath $resolvedLogPath
} else {
    Write-InstallLog -Level 'WARN' -Message 'pyenv-mcp source binary was not found; installing pyenv CLI only.' -ResolvedLogPath $resolvedLogPath
}

$cmdWrapper = '@echo off' + "`r`n" + '"%~dp0pyenv.exe" %*' + "`r`n"
$ps1Wrapper = '& "$PSScriptRoot\pyenv.exe" @args' + "`r`n" + 'exit $LASTEXITCODE' + "`r`n"
$cmdInitHelper = '@echo off' + "`r`n" + 'for /f "delims=" %%i in (''"%~dp0pyenv.exe" init - cmd'') do %%i' + "`r`n"
$mcpCmdWrapper = '@echo off' + "`r`n" + '"%~dp0pyenv-mcp.exe" %*' + "`r`n"
$mcpPs1Wrapper = '& "$PSScriptRoot\pyenv-mcp.exe" @args' + "`r`n" + 'exit $LASTEXITCODE' + "`r`n"

Write-TextFile -Path (Join-Path $installBin 'pyenv.cmd') -Contents $cmdWrapper
Write-TextFile -Path (Join-Path $installBin 'pyenv.ps1') -Contents $ps1Wrapper
Write-TextFile -Path (Join-Path $installBin 'pyenv-init.cmd') -Contents $cmdInitHelper
if ($resolvedMcpSource) {
    Write-TextFile -Path (Join-Path $installBin 'pyenv-mcp.cmd') -Contents $mcpCmdWrapper
    Write-TextFile -Path (Join-Path $installBin 'pyenv-mcp.ps1') -Contents $mcpPs1Wrapper
}
Write-InstallLog -Level 'INFO' -Message "Installed core binaries into $installBin" -ResolvedLogPath $resolvedLogPath

if ($addToUserPathValue) {
    Ensure-LineInUserPath -Entry $installBin
    Write-InstallLog -Level 'INFO' -Message 'Updated user PATH to include the install bin directory.' -ResolvedLogPath $resolvedLogPath
}

if ($updateProfileEffective) {
    $profilePath = Update-PowerShellProfileBlock -InstalledExePath $installedExe
    Write-InstallLog -Level 'INFO' -Message "Updated PowerShell profile at $profilePath" -ResolvedLogPath $resolvedLogPath
}

if ($refreshShimsValue) {
    & $installedExe rehash | Out-Null
    Write-InstallLog -Level 'INFO' -Message 'Refreshed shims.' -ResolvedLogPath $resolvedLogPath
}

Invoke-BinarySanityCheck -CommandPath $installedExe -ResolvedInstallRoot $resolvedInstallRoot -Name 'pyenv --version' -Arguments @('--version') -ResolvedLogPath $resolvedLogPath
Invoke-BinarySanityCheck -CommandPath $installedExe -ResolvedInstallRoot $resolvedInstallRoot -Name 'pyenv root' -Arguments @('root') -ResolvedLogPath $resolvedLogPath
Invoke-BinarySanityCheck -CommandPath $installedExe -ResolvedInstallRoot $resolvedInstallRoot -Name 'pyenv commands' -Arguments @('commands') -ResolvedLogPath $resolvedLogPath
if ($resolvedMcpSource) {
    Invoke-BinarySanityCheck -CommandPath $installedMcpExe -ResolvedInstallRoot $resolvedInstallRoot -Name 'pyenv-mcp guide' -Arguments @('guide') -ResolvedLogPath $resolvedLogPath
}

Write-InstallLog -Level 'INFO' -Message 'Install completed successfully.' -ResolvedLogPath $resolvedLogPath
Write-Host ''
Write-Host "Installed pyenv-native to $resolvedInstallRoot"
Write-Host "Installed command: $installedExe"
if ($resolvedMcpSource) {
    Write-Host "Installed MCP server: $installedMcpExe"
    Write-Host "MCP config helper: & '$installedMcpExe' print-config"
}
Write-Host "Log file: $resolvedLogPath"
if ($Shell -eq 'cmd') {
    Write-Host "CMD note: run '$installBin\pyenv-init.cmd' in each interactive CMD session, or wire it into your preferred startup flow."
}
if ($addToUserPathValue) {
    Write-Host 'PATH note: restart your shell to pick up the updated user PATH.'
}
