# ./install.ps1
<#
Purpose: Downloads a published pyenv-native Windows bundle, verifies it, and runs the bundled portable installer without requiring a repo clone.
How to run: powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1 [-GitHubRepo <owner/repo>] [-Tag <vX.Y.Z>] [-InstallRoot <dir>] [-Yes]
Inputs: Optional GitHub repo/tag or direct release URLs, install root, shell/profile toggles, temp cache location, logging location, and overwrite/cleanup flags.
Outputs/side effects: Downloads the Windows release bundle plus checksum, verifies SHA-256, extracts the bundle into a temp directory, and installs pyenv-native into the requested portable root.
Notes: Designed for copy-paste web installs from a raw GitHub URL, defaults to the latest published GitHub release, and keeps installs registry-free by default.
#>

param(
    [string]$GitHubRepo = $(if ($env:PYENV_NATIVE_INSTALL_GITHUB_REPO) { $env:PYENV_NATIVE_INSTALL_GITHUB_REPO } else { 'imyourboyroy/pyenv-native' }),
    [string]$Tag = $env:PYENV_NATIVE_INSTALL_TAG,
    [string]$ReleaseBaseUrl = $env:PYENV_NATIVE_INSTALL_RELEASE_BASE_URL,
    [string]$BundleUrl = $env:PYENV_NATIVE_INSTALL_BUNDLE_URL,
    [string]$ChecksumUrl = $env:PYENV_NATIVE_INSTALL_CHECKSUM_URL,
    [string]$InstallRoot = (Join-Path $HOME '.pyenv'),
    [ValidateSet('pwsh', 'cmd', 'none')]
    [string]$Shell = 'pwsh',
    [string]$AddToUserPath = 'true',
    [string]$UpdatePowerShellProfile = 'true',
    [string]$RefreshShims = 'true',
    [string]$TempRoot = $(if ($env:TEMP) { Join-Path $env:TEMP 'pyenv-native-install' } else { Join-Path $HOME '.pyenv-native-install' }),
    [string]$LogPath,
    [switch]$KeepDownloads,
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

function Get-HostArchitecture {
    if (([System.Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT) -and ($env:OS -ne 'Windows_NT')) {
        throw 'install.ps1 currently supports Windows hosts only. Use install.sh on Linux or macOS.'
    }

    $candidate = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
    $candidate = if ($candidate) { $candidate } else { '' }

    switch ($candidate.ToLowerInvariant()) {
        'amd64' { return 'x64' }
        'x86_64' { return 'x64' }
        'x64' { return 'x64' }
        'arm64' { return 'arm64' }
        default {
            if ([Environment]::Is64BitOperatingSystem) {
                return 'x64'
            }
            throw "Unsupported Windows architecture '$candidate'."
        }
    }
}

function Resolve-AssetName {
    $arch = Get-HostArchitecture
    switch ($arch) {
        'x64' { return 'pyenv-native-windows-x64.zip' }
        'arm64' { return 'pyenv-native-windows-arm64.zip' }
        default { throw "Published Windows bundles are not available yet for architecture '$arch'." }
    }
}

function Resolve-ReleaseUrls {
    param(
        [string]$ResolvedGitHubRepo,
        [string]$ResolvedTag,
        [string]$ResolvedReleaseBaseUrl,
        [string]$ResolvedBundleUrl,
        [string]$ResolvedChecksumUrl
    )

    $assetName = Resolve-AssetName

    if ($ResolvedBundleUrl) {
        return [ordered]@{
            bundle_url = $ResolvedBundleUrl
            checksum_url = $(if ($ResolvedChecksumUrl) { $ResolvedChecksumUrl } else { $ResolvedBundleUrl + '.sha256' })
            asset_name = $assetName
            source = 'explicit bundle url'
        }
    }

    $sourceLabel = ''
    if (-not $ResolvedReleaseBaseUrl) {
        if (-not $ResolvedGitHubRepo) {
            throw 'Unable to resolve a release source. Pass -GitHubRepo, -ReleaseBaseUrl, or -BundleUrl.'
        }

        $ResolvedReleaseBaseUrl = if ($ResolvedTag) {
            "https://github.com/$ResolvedGitHubRepo/releases/download/$ResolvedTag"
        } else {
            "https://github.com/$ResolvedGitHubRepo/releases/latest/download"
        }

        $sourceLabel = if ($ResolvedTag) {
            "github release $ResolvedGitHubRepo@$ResolvedTag"
        } else {
            "latest github release for $ResolvedGitHubRepo"
        }
    } else {
        $sourceLabel = "release base url $ResolvedReleaseBaseUrl"
    }

    $bundleUrl = "$($ResolvedReleaseBaseUrl.TrimEnd('/'))/$assetName"
    $checksumUrl = if ($ResolvedChecksumUrl) { $ResolvedChecksumUrl } else { $bundleUrl + '.sha256' }

    return [ordered]@{
        bundle_url = $bundleUrl
        checksum_url = $checksumUrl
        asset_name = $assetName
        source = $sourceLabel
    }
}

function Invoke-FileDownload {
    param(
        [string]$Url,
        [string]$DestinationPath
    )

    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $DestinationPath) | Out-Null

    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls13
    } catch {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    }

    $webClient = New-Object -TypeName System.Net.WebClient
    try {
        $webClient.DownloadFile($Url, $DestinationPath)
    } finally {
        $webClient.Dispose()
    }
}

function Read-ExpectedChecksum {
    param(
        [string]$ChecksumPath
    )

    $line = Get-Content -Path $ChecksumPath -TotalCount 1
    if (-not $line) {
        throw "Checksum file '$ChecksumPath' was empty."
    }

    $match = [regex]::Match($line.Trim(), '^(?<sha>[A-Fa-f0-9]{64})\b')
    if (-not $match.Success) {
        throw "Checksum file '$ChecksumPath' did not contain a valid SHA-256 digest."
    }

    return $match.Groups['sha'].Value.ToLowerInvariant()
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
        [bool]$Overwrite
    )

    $installedExe = Join-Path (Join-Path $ResolvedInstallRoot 'bin') 'pyenv.exe'
    if (Test-Path $installedExe) {
        if (-not $Overwrite) {
            Write-Warning "pyenv-native is already installed at $installedExe. Proceeding will upgrade or overwrite the installation in-place."
        }
        return
    }

    if ((Test-Path $ResolvedInstallRoot) -and -not $Overwrite) {
        $firstChild = Get-ChildItem -Force $ResolvedInstallRoot -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($firstChild) {
            Write-Warning "Install root '$ResolvedInstallRoot' already exists and is not empty. Proceeding will install into this existing directory."
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

function Write-InstallSummary {
    param(
        [hashtable]$Summary
    )

    Write-Host ''
    Write-Host 'pyenv-native network install summary'
    Write-Host '===================================='
    foreach ($entry in $Summary.GetEnumerator()) {
        Write-Host ('{0,-16}: {1}' -f $entry.Key, $entry.Value)
    }
    Write-Host ''
    Write-Host 'This will download a published pyenv-native bundle, verify its SHA-256 checksum, and install it into the selected portable root.'
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

function Invoke-BundledInstaller {
    param(
        [string]$ExtractedDir,
        [string]$ResolvedInstallRoot,
        [bool]$AddToUserPathValue,
        [bool]$UpdateProfileValue,
        [bool]$RefreshShimsValue,
        [string]$ResolvedLogPath,
        [switch]$Overwrite
    )

    $installerPath = Join-Path $ExtractedDir 'install-pyenv-native.ps1'
    $executablePath = Join-Path $ExtractedDir 'pyenv.exe'
    $manifestPath = Join-Path $ExtractedDir 'bundle-manifest.json'
    $mcpExecutablePath = Join-Path $ExtractedDir 'pyenv-mcp.exe'
    $guiExecutablePath = Join-Path $ExtractedDir 'pyenv-gui.exe'

    foreach ($requiredPath in @($installerPath, $executablePath, $manifestPath)) {
        if (-not (Test-Path $requiredPath)) {
            throw "Downloaded bundle was missing required file '$requiredPath'."
        }
    }

    $manifest = Get-Content -Raw $manifestPath | ConvertFrom-Json
    if ($manifest.platform -ne 'windows') {
        throw "Downloaded bundle platform '$($manifest.platform)' does not match this Windows installer."
    }

    if ($manifest.mcp_executable -and -not (Test-Path $mcpExecutablePath)) {
        throw "Downloaded bundle declared an MCP server binary but '$mcpExecutablePath' was missing."
    }

    if ($manifest.gui_executable -and -not (Test-Path $guiExecutablePath)) {
        throw "Downloaded bundle declared a GUI companion binary but '$guiExecutablePath' was missing."
    }

    $installerArgs = @{
        SourcePath = $executablePath
        InstallRoot = $ResolvedInstallRoot
        Shell = $Shell
        AddToUserPath = $AddToUserPathValue.ToString().ToLowerInvariant()
        UpdatePowerShellProfile = $UpdateProfileValue.ToString().ToLowerInvariant()
        RefreshShims = $RefreshShimsValue.ToString().ToLowerInvariant()
        LogPath = $ResolvedLogPath
        Yes = $true
    }

    if ($Overwrite) {
        $installerArgs['Force'] = $true
    }

    if (Test-Path $mcpExecutablePath) {
        $installerArgs['SourceMcpPath'] = $mcpExecutablePath
    }

    if (Test-Path $guiExecutablePath) {
        $installerArgs['SourceGuiPath'] = $guiExecutablePath
    }

    & $installerPath @installerArgs
}

$resolvedInstallRoot = [System.IO.Path]::GetFullPath($InstallRoot)
$resolvedTempRoot = [System.IO.Path]::GetFullPath($TempRoot)
$addToUserPathValue = Convert-ToBoolean -Value $AddToUserPath -ParameterName 'AddToUserPath'
$updateProfileValue = Convert-ToBoolean -Value $UpdatePowerShellProfile -ParameterName 'UpdatePowerShellProfile'
$refreshShimsValue = Convert-ToBoolean -Value $RefreshShims -ParameterName 'RefreshShims'
$updateProfileEffective = ($Shell -eq 'pwsh' -and $updateProfileValue)
if ([string]::IsNullOrWhiteSpace($LogPath)) {
    $timestamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $LogPath = Join-Path (Join-Path $resolvedInstallRoot 'logs') "network-install-$timestamp.log"
}
$resolvedLogPath = [System.IO.Path]::GetFullPath($LogPath)

Test-ExistingPathCommand -ResolvedInstallRoot $resolvedInstallRoot
Assert-InstallRootAccess -ResolvedInstallRoot $resolvedInstallRoot
Assert-InstallRootState -ResolvedInstallRoot $resolvedInstallRoot -Overwrite $Force.IsPresent

$urls = Resolve-ReleaseUrls -ResolvedGitHubRepo $GitHubRepo -ResolvedTag $Tag -ResolvedReleaseBaseUrl $ReleaseBaseUrl -ResolvedBundleUrl $BundleUrl -ResolvedChecksumUrl $ChecksumUrl
$summary = [ordered]@{
    release_source = $urls.source
    bundle_url = $urls.bundle_url
    checksum_url = $urls.checksum_url
    install_root = $resolvedInstallRoot
    shell = $Shell
    add_to_path = $addToUserPathValue
    update_profile = $updateProfileEffective
    refresh_shims = $refreshShimsValue
    temp_root = $resolvedTempRoot
    force = $Force.IsPresent
    log_path = $resolvedLogPath
}
Write-InstallSummary -Summary $summary
Confirm-Install
Initialize-InstallLog -ResolvedLogPath $resolvedLogPath
Write-InstallLog -Level 'INFO' -Message "Downloading $($urls.source)" -ResolvedLogPath $resolvedLogPath

$downloadRoot = Join-Path $resolvedTempRoot ('downloads-' + [guid]::NewGuid().ToString('N'))
$extractRoot = Join-Path $resolvedTempRoot ('extract-' + [guid]::NewGuid().ToString('N'))
$bundlePath = Join-Path $downloadRoot $urls.asset_name
$checksumPath = $bundlePath + '.sha256'

try {
    Invoke-FileDownload -Url $urls.bundle_url -DestinationPath $bundlePath
    Invoke-FileDownload -Url $urls.checksum_url -DestinationPath $checksumPath

    $expectedHash = Read-ExpectedChecksum -ChecksumPath $checksumPath
    $actualHash = (Get-FileHash -Algorithm SHA256 $bundlePath).Hash.ToLowerInvariant()
    if ($actualHash -ne $expectedHash) {
        throw "SHA-256 verification failed for '$bundlePath'. Expected $expectedHash but found $actualHash."
    }
    Write-InstallLog -Level 'INFO' -Message "Verified SHA-256 for $($urls.asset_name)" -ResolvedLogPath $resolvedLogPath

    New-Item -ItemType Directory -Force -Path $extractRoot | Out-Null
    Expand-Archive -Path $bundlePath -DestinationPath $extractRoot -Force

    Invoke-BundledInstaller -ExtractedDir $extractRoot -ResolvedInstallRoot $resolvedInstallRoot -AddToUserPathValue $addToUserPathValue -UpdateProfileValue $updateProfileValue -RefreshShimsValue $refreshShimsValue -ResolvedLogPath $resolvedLogPath -Overwrite:$Force.IsPresent

    Write-InstallLog -Level 'INFO' -Message 'Network install completed successfully.' -ResolvedLogPath $resolvedLogPath
    Write-Host ''
    Write-Host "Installed pyenv-native to $resolvedInstallRoot"
    Write-Host "Installed command: $(Join-Path $resolvedInstallRoot 'bin\pyenv.exe')"
    Write-Host "Log file: $resolvedLogPath"
    if ($GitHubRepo) {
        Write-Host "Remote uninstall helper: https://raw.githubusercontent.com/$GitHubRepo/main/uninstall.ps1"
    }
} finally {
    if (-not $KeepDownloads) {
        foreach ($cleanupPath in @($downloadRoot, $extractRoot)) {
            if (Test-Path $cleanupPath) {
                Remove-Item -Recurse -Force $cleanupPath -ErrorAction SilentlyContinue
            }
        }
    }
}
