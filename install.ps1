# ./install.ps1
<#
Purpose: Downloads a published pyenv-native Windows bundle, verifies it, and runs the bundled portable installer without requiring a repo clone.
How to run: powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1 [-GitHubRepo <owner/repo>] [-Tag <vX.Y.Z>] [-InstallRoot <dir>]
Inputs: Optional GitHub repo/tag or direct release URLs, install root, shell/profile toggles, temp cache location, and overwrite/cleanup flags.
Outputs/side effects: Downloads the Windows release bundle plus checksum, verifies SHA-256, extracts the bundle into a temp directory, and installs pyenv-native into the requested portable root.
Notes: Designed for copy-paste web installs from a raw GitHub URL and keeps installs registry-free by default.
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
    [switch]$KeepDownloads,
    [switch]$Force
)

$ErrorActionPreference = 'Stop'

function Write-Step {
    param(
        [string]$Message
    )

    Write-Host "[pyenv-native] $Message"
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

        $ResolvedReleaseBaseUrl =
            if ($ResolvedTag) {
                "https://github.com/$ResolvedGitHubRepo/releases/download/$ResolvedTag"
            } else {
                "https://github.com/$ResolvedGitHubRepo/releases/latest/download"
            }

        $sourceLabel =
            if ($ResolvedTag) {
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

    if ($PSVersionTable.PSVersion.Major -lt 6) {
        Invoke-WebRequest -UseBasicParsing -Uri $Url -OutFile $DestinationPath
    } else {
        Invoke-WebRequest -Uri $Url -OutFile $DestinationPath
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

function Assert-InstallRootState {
    param(
        [string]$ResolvedInstallRoot,
        [bool]$Overwrite
    )

    $installBin = Join-Path $ResolvedInstallRoot 'bin'
    $installedExe = Join-Path $installBin 'pyenv.exe'

    if (Test-Path $installedExe) {
        if (-not $Overwrite) {
            throw "pyenv-native is already installed at $installedExe. Re-run with -Force to upgrade in place or run uninstall.ps1 first."
        }

        Write-Step "Existing pyenv-native install detected at $installedExe; continuing because -Force was supplied."
        return
    }

    if ((Test-Path $ResolvedInstallRoot) -and (Get-ChildItem -Force $ResolvedInstallRoot | Select-Object -First 1) -and -not $Overwrite) {
        throw "Install root '$ResolvedInstallRoot' already exists and is not empty. Re-run with -Force or choose a different -InstallRoot."
    }
}

function Test-ExistingPathCommand {
    param(
        [string]$ResolvedInstallRoot
    )

    $existing = Get-Command pyenv -ErrorAction SilentlyContinue
    if (-not $existing) {
        return
    }

    $expectedBin = (Join-Path $ResolvedInstallRoot 'bin').TrimEnd('\')
    $actualSource = if ($existing.Source) { $existing.Source.Trim() } else { '' }
    if (-not $actualSource) {
        return
    }

    if ($actualSource -notlike "$expectedBin*") {
        Write-Warning "A different pyenv command is already discoverable at '$actualSource'. Restart shells after install and verify PATH ordering."
    }
}

function Invoke-BundledInstaller {
    param(
        [string]$ExtractedDir,
        [string]$ResolvedInstallRoot,
        [bool]$AddToUserPathValue,
        [bool]$UpdateProfileValue,
        [bool]$RefreshShimsValue,
        [switch]$Overwrite
    )

    $installerPath = Join-Path $ExtractedDir 'install-pyenv-native.ps1'
    $executablePath = Join-Path $ExtractedDir 'pyenv.exe'
    $manifestPath = Join-Path $ExtractedDir 'bundle-manifest.json'

    foreach ($requiredPath in @($installerPath, $executablePath, $manifestPath)) {
        if (-not (Test-Path $requiredPath)) {
            throw "Downloaded bundle was missing required file '$requiredPath'."
        }
    }

    $manifest = Get-Content -Raw $manifestPath | ConvertFrom-Json
    if ($manifest.platform -ne 'windows') {
        throw "Downloaded bundle platform '$($manifest.platform)' does not match this Windows installer."
    }

    Write-Step "Running bundled installer from $installerPath"
    & $installerPath `
        -SourcePath $executablePath `
        -InstallRoot $ResolvedInstallRoot `
        -Shell $Shell `
        -AddToUserPath $AddToUserPathValue.ToString().ToLowerInvariant() `
        -UpdatePowerShellProfile $UpdateProfileValue.ToString().ToLowerInvariant() `
        -RefreshShims $RefreshShimsValue.ToString().ToLowerInvariant() `
        @($(if ($Overwrite) { '-Force' }))
}

$resolvedInstallRoot = [System.IO.Path]::GetFullPath($InstallRoot)
$addToUserPathValue = Convert-ToBoolean -Value $AddToUserPath -ParameterName 'AddToUserPath'
$updateProfileValue = Convert-ToBoolean -Value $UpdatePowerShellProfile -ParameterName 'UpdatePowerShellProfile'
$refreshShimsValue = Convert-ToBoolean -Value $RefreshShims -ParameterName 'RefreshShims'
$resolvedTempRoot = [System.IO.Path]::GetFullPath($TempRoot)

Write-Step "Preparing install for $resolvedInstallRoot"
Test-ExistingPathCommand -ResolvedInstallRoot $resolvedInstallRoot
Assert-InstallRootState -ResolvedInstallRoot $resolvedInstallRoot -Overwrite $Force.IsPresent

$urls = Resolve-ReleaseUrls `
    -ResolvedGitHubRepo $GitHubRepo `
    -ResolvedTag $Tag `
    -ResolvedReleaseBaseUrl $ReleaseBaseUrl `
    -ResolvedBundleUrl $BundleUrl `
    -ResolvedChecksumUrl $ChecksumUrl

$downloadRoot = Join-Path $resolvedTempRoot ("downloads-" + [guid]::NewGuid().ToString('N'))
$extractRoot = Join-Path $resolvedTempRoot ("extract-" + [guid]::NewGuid().ToString('N'))
$bundlePath = Join-Path $downloadRoot $urls.asset_name
$checksumPath = $bundlePath + '.sha256'

try {
    Write-Step "Downloading $($urls.source)"
    Invoke-FileDownload -Url $urls.bundle_url -DestinationPath $bundlePath
    Invoke-FileDownload -Url $urls.checksum_url -DestinationPath $checksumPath

    $expectedHash = Read-ExpectedChecksum -ChecksumPath $checksumPath
    $actualHash = (Get-FileHash -Algorithm SHA256 $bundlePath).Hash.ToLowerInvariant()
    if ($actualHash -ne $expectedHash) {
        throw "SHA-256 verification failed for '$bundlePath'. Expected $expectedHash but found $actualHash."
    }
    Write-Step "Verified SHA-256 for $($urls.asset_name)"

    if (Test-Path $extractRoot) {
        Remove-Item -Recurse -Force $extractRoot
    }
    New-Item -ItemType Directory -Force -Path $extractRoot | Out-Null
    Expand-Archive -Path $bundlePath -DestinationPath $extractRoot -Force

    Invoke-BundledInstaller `
        -ExtractedDir $extractRoot `
        -ResolvedInstallRoot $resolvedInstallRoot `
        -AddToUserPathValue $addToUserPathValue `
        -UpdateProfileValue $updateProfileValue `
        -RefreshShimsValue $refreshShimsValue `
        -Overwrite:$Force

    Write-Host ''
    Write-Host "Installed pyenv-native to $resolvedInstallRoot"
    Write-Host "Bundle source: $($urls.source)"
    Write-Host "Installed command: $(Join-Path $resolvedInstallRoot 'bin\pyenv.exe')"
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





