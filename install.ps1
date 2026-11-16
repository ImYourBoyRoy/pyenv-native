# ./install.ps1
<#
.SYNOPSIS
Bootstrap installer for Enchi (formerly pyenv-native) on Windows.

.DESCRIPTION
Downloads the published Windows release bundle, verifies its SHA-256, and hands over to the bundled
`enchi.exe self-install`, which does the actual install (binaries, PATH, PowerShell profile, shims,
sanity checks) and decides the install root (`~\.enchi`, or an existing `~\.pyenv` it upgrades).
This script only fetches and verifies; all install logic lives in the Rust binary.

.EXAMPLE
irm https://github.com/imyourboyroy/pyenv-native/releases/latest/download/install.ps1 | iex

.EXAMPLE
.\install.ps1 -Tag v0.6.0 -InstallRoot "$HOME\.enchi" -Yes

.NOTES
-BundleUrl accepts file:// URLs for offline installs; pair it with -ChecksumUrl.
Run `enchi self-install --help` for the options these parameters map onto.
Environment: ENCHI_INSTALL_GITHUB_REPO, ENCHI_INSTALL_TAG, ENCHI_INSTALL_RELEASE_BASE_URL,
ENCHI_INSTALL_BUNDLE_URL, ENCHI_INSTALL_CHECKSUM_URL, ENCHI_INSTALL_POWERSHELL7 (the
PYENV_NATIVE_INSTALL_* names are still read as a fallback).
#>

param(
    # The default repository is the GitHub project that publishes releases. It keeps its pre-rename
    # name until the repository is renamed on GitHub; that rename is then a one-line edit here.
    [string]$GitHubRepo = $(if ($env:ENCHI_INSTALL_GITHUB_REPO) { $env:ENCHI_INSTALL_GITHUB_REPO } elseif ($env:PYENV_NATIVE_INSTALL_GITHUB_REPO) { $env:PYENV_NATIVE_INSTALL_GITHUB_REPO } else { 'imyourboyroy/pyenv-native' }),
    [string]$Tag = $(if ($env:ENCHI_INSTALL_TAG) { $env:ENCHI_INSTALL_TAG } else { $env:PYENV_NATIVE_INSTALL_TAG }),
    [string]$ReleaseBaseUrl = $(if ($env:ENCHI_INSTALL_RELEASE_BASE_URL) { $env:ENCHI_INSTALL_RELEASE_BASE_URL } else { $env:PYENV_NATIVE_INSTALL_RELEASE_BASE_URL }),
    [string]$BundleUrl = $(if ($env:ENCHI_INSTALL_BUNDLE_URL) { $env:ENCHI_INSTALL_BUNDLE_URL } else { $env:PYENV_NATIVE_INSTALL_BUNDLE_URL }),
    [string]$ChecksumUrl = $(if ($env:ENCHI_INSTALL_CHECKSUM_URL) { $env:ENCHI_INSTALL_CHECKSUM_URL } else { $env:PYENV_NATIVE_INSTALL_CHECKSUM_URL }),
    [string]$InstallRoot,
    [ValidateSet('pwsh', 'cmd', 'none')]
    [string]$Shell = 'pwsh',
    [string]$AddToUserPath = 'true',
    [string]$UpdatePowerShellProfile = 'true',
    [string]$RefreshShims = 'true',
    [string]$TempRoot = $(if ($env:TEMP) { Join-Path $env:TEMP 'enchi-install' } else { Join-Path $HOME '.enchi-install' }),
    [string]$LogPath,
    [switch]$KeepDownloads,
    [switch]$Force,
    [switch]$Yes,
    [string]$Lang,
    [ValidateSet('auto', 'true', 'false')]
    [string]$InstallPowerShell7 = $(if ($env:ENCHI_INSTALL_POWERSHELL7) { $env:ENCHI_INSTALL_POWERSHELL7 } elseif ($env:PYENV_NATIVE_INSTALL_POWERSHELL7) { $env:PYENV_NATIVE_INSTALL_POWERSHELL7 } else { 'auto' })
)

$ErrorActionPreference = 'Stop'

function Get-ArchitectureToken {
    if (([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) -and ($env:OS -ne 'Windows_NT')) {
        throw 'install.ps1 supports Windows hosts only. Use install.sh on Linux, macOS, or Termux.'
    }
    $candidate = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
    switch (([string]$candidate).ToLowerInvariant()) {
        { $_ -in 'amd64', 'x86_64', 'x64' } { return 'x64' }
        'arm64' { return 'arm64' }
        default {
            if ([Environment]::Is64BitOperatingSystem) { return 'x64' }
            throw "No published Windows bundle for architecture '$candidate'."
        }
    }
}

function Get-FileFromUrl {
    param([string]$Url, [string]$Destination)
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls13
    } catch {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    }
    $client = New-Object System.Net.WebClient
    try { $client.DownloadFile($Url, $Destination) } finally { $client.Dispose() }
}

$arch = Get-ArchitectureToken
$assetName = "enchi-windows-$arch.zip"
# Releases cut before the rename only publish this name (same layout, with `pyenv.exe` as the CLI).
$legacyAssetName = "pyenv-native-windows-$arch.zip"
$derivedUrl = $false
$explicitChecksum = [bool]$ChecksumUrl
if (-not $BundleUrl) {
    if (-not $ReleaseBaseUrl) {
        if (-not $GitHubRepo) { throw 'No release source: pass -GitHubRepo, -ReleaseBaseUrl, or -BundleUrl.' }
        $ReleaseBaseUrl = if ($Tag) {
            "https://github.com/$GitHubRepo/releases/download/$Tag"
        } else {
            "https://github.com/$GitHubRepo/releases/latest/download"
        }
    }
    $BundleUrl = "$($ReleaseBaseUrl.TrimEnd('/'))/$assetName"
    $derivedUrl = $true
}
if (-not $ChecksumUrl) { $ChecksumUrl = "$BundleUrl.sha256" }

$work = Join-Path $TempRoot ('run-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $work | Out-Null

try {
    $archive = Join-Path $work $assetName
    $checksumFile = "$archive.sha256"
    Write-Host "[enchi] Downloading $BundleUrl"
    try {
        Get-FileFromUrl -Url $BundleUrl -Destination $archive
    } catch {
        if (-not ($derivedUrl -and -not $explicitChecksum)) { throw }
        Write-Host "[enchi] That release has no $assetName; trying the pre-rename asset $legacyAssetName"
        $assetName = $legacyAssetName
        $BundleUrl = "$($ReleaseBaseUrl.TrimEnd('/'))/$assetName"
        $ChecksumUrl = "$BundleUrl.sha256"
        $archive = Join-Path $work $assetName
        $checksumFile = "$archive.sha256"
        Write-Host "[enchi] Downloading $BundleUrl"
        Get-FileFromUrl -Url $BundleUrl -Destination $archive
    }
    Get-FileFromUrl -Url $ChecksumUrl -Destination $checksumFile

    $line = (Get-Content -Path $checksumFile -TotalCount 1)
    $match = [regex]::Match(([string]$line).Trim(), '^(?<sha>[A-Fa-f0-9]{64})\b')
    if (-not $match.Success) { throw "The checksum file did not contain a valid SHA-256 digest." }
    $expected = $match.Groups['sha'].Value.ToLowerInvariant()
    $actual = (Get-FileHash -Algorithm SHA256 -Path $archive).Hash.ToLowerInvariant()
    if ($actual -ne $expected) {
        throw "SHA-256 verification failed (expected $expected, got $actual)."
    }
    Write-Host "[enchi] Verified SHA-256 for $assetName"

    $bundle = Join-Path $work 'bundle'
    Expand-Archive -Path $archive -DestinationPath $bundle -Force
    # The bundle holds enchi.exe; bundles from before the rename hold pyenv.exe instead.
    $exeName = @('enchi.exe', 'pyenv.exe') | Where-Object { Test-Path -LiteralPath (Join-Path $bundle $_) } | Select-Object -First 1
    if (-not $exeName) {
        $nested = Get-ChildItem -Path $bundle -Directory | Where-Object {
            (Test-Path (Join-Path $_.FullName 'enchi.exe')) -or (Test-Path (Join-Path $_.FullName 'pyenv.exe'))
        } | Select-Object -First 1
        if (-not $nested) { throw 'The downloaded bundle did not contain enchi.exe.' }
        $bundle = $nested.FullName
        $exeName = @('enchi.exe', 'pyenv.exe') | Where-Object { Test-Path -LiteralPath (Join-Path $bundle $_) } | Select-Object -First 1
    }
    $exe = Join-Path $bundle $exeName

    $arguments = @(
        'self-install',
        '--source-dir', $bundle,
        '--shell', $Shell,
        '--add-to-user-path', $AddToUserPath,
        '--update-shell-profile', $UpdatePowerShellProfile,
        '--refresh-shims', $RefreshShims,
        '--install-powershell7', $InstallPowerShell7
    )
    if ($InstallRoot) { $arguments += @('--install-root', $InstallRoot) }
    if ($LogPath) { $arguments += @('--log-path', $LogPath) }
    if ($Lang) { $arguments += @('--lang', $Lang) }
    if ($Force) { $arguments += '--force' }
    if ($Yes) { $arguments += '--yes' }

    Write-Host "[enchi] Running $exeName self-install"
    & $exe @arguments
    if ($LASTEXITCODE -ne 0) { throw "$exeName self-install failed (exit code $LASTEXITCODE)." }
} finally {
    if (-not $KeepDownloads) {
        Remove-Item -Recurse -Force -LiteralPath $work -ErrorAction SilentlyContinue
    }
}
