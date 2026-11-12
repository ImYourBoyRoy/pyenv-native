# ./install.ps1
<#
.SYNOPSIS
Bootstrap installer for pyenv-native on Windows.

.DESCRIPTION
Downloads the published Windows release bundle, verifies its SHA-256, and hands over to the bundled
`pyenv.exe self-install`, which does the actual install (binaries, PATH, PowerShell profile, shims,
sanity checks). This script only fetches and verifies; all install logic lives in the Rust binary.

.EXAMPLE
irm https://github.com/imyourboyroy/pyenv-native/releases/latest/download/install.ps1 | iex

.EXAMPLE
.\install.ps1 -Tag v0.6.0 -InstallRoot "$HOME\.pyenv" -Yes

.NOTES
-BundleUrl accepts file:// URLs for offline installs; pair it with -ChecksumUrl.
Run `pyenv self-install --help` for the options these parameters map onto.
#>

param(
    [string]$GitHubRepo = $(if ($env:PYENV_NATIVE_INSTALL_GITHUB_REPO) { $env:PYENV_NATIVE_INSTALL_GITHUB_REPO } else { 'imyourboyroy/pyenv-native' }),
    [string]$Tag = $env:PYENV_NATIVE_INSTALL_TAG,
    [string]$ReleaseBaseUrl = $env:PYENV_NATIVE_INSTALL_RELEASE_BASE_URL,
    [string]$BundleUrl = $env:PYENV_NATIVE_INSTALL_BUNDLE_URL,
    [string]$ChecksumUrl = $env:PYENV_NATIVE_INSTALL_CHECKSUM_URL,
    [string]$InstallRoot,
    [ValidateSet('pwsh', 'cmd', 'none')]
    [string]$Shell = 'pwsh',
    [string]$AddToUserPath = 'true',
    [string]$UpdatePowerShellProfile = 'true',
    [string]$RefreshShims = 'true',
    [string]$TempRoot = $(if ($env:TEMP) { Join-Path $env:TEMP 'pyenv-native-install' } else { Join-Path $HOME '.pyenv-native-install' }),
    [string]$LogPath,
    [switch]$KeepDownloads,
    [switch]$Force,
    [switch]$Yes,
    [string]$Lang,
    [ValidateSet('auto', 'true', 'false')]
    [string]$InstallPowerShell7 = $(if ($env:PYENV_NATIVE_INSTALL_POWERSHELL7) { $env:PYENV_NATIVE_INSTALL_POWERSHELL7 } else { 'auto' })
)

$ErrorActionPreference = 'Stop'

function Get-AssetName {
    if (([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) -and ($env:OS -ne 'Windows_NT')) {
        throw 'install.ps1 supports Windows hosts only. Use install.sh on Linux, macOS, or Termux.'
    }
    $candidate = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
    switch (([string]$candidate).ToLowerInvariant()) {
        { $_ -in 'amd64', 'x86_64', 'x64' } { return 'pyenv-native-windows-x64.zip' }
        'arm64' { return 'pyenv-native-windows-arm64.zip' }
        default {
            if ([Environment]::Is64BitOperatingSystem) { return 'pyenv-native-windows-x64.zip' }
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

$assetName = Get-AssetName
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
}
if (-not $ChecksumUrl) { $ChecksumUrl = "$BundleUrl.sha256" }

$work = Join-Path $TempRoot ('run-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $work | Out-Null

try {
    $archive = Join-Path $work $assetName
    $checksumFile = "$archive.sha256"
    Write-Host "[pyenv-native] Downloading $BundleUrl"
    Get-FileFromUrl -Url $BundleUrl -Destination $archive
    Get-FileFromUrl -Url $ChecksumUrl -Destination $checksumFile

    $line = (Get-Content -Path $checksumFile -TotalCount 1)
    $match = [regex]::Match(([string]$line).Trim(), '^(?<sha>[A-Fa-f0-9]{64})\b')
    if (-not $match.Success) { throw "The checksum file did not contain a valid SHA-256 digest." }
    $expected = $match.Groups['sha'].Value.ToLowerInvariant()
    $actual = (Get-FileHash -Algorithm SHA256 -Path $archive).Hash.ToLowerInvariant()
    if ($actual -ne $expected) {
        throw "SHA-256 verification failed (expected $expected, got $actual)."
    }
    Write-Host "[pyenv-native] Verified SHA-256 for $assetName"

    $bundle = Join-Path $work 'bundle'
    Expand-Archive -Path $archive -DestinationPath $bundle -Force
    $exe = Join-Path $bundle 'pyenv.exe'
    if (-not (Test-Path -LiteralPath $exe)) {
        $nested = Get-ChildItem -Path $bundle -Directory | Where-Object { Test-Path (Join-Path $_.FullName 'pyenv.exe') } | Select-Object -First 1
        if (-not $nested) { throw 'The downloaded bundle did not contain pyenv.exe.' }
        $bundle = $nested.FullName
        $exe = Join-Path $bundle 'pyenv.exe'
    }

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

    Write-Host '[pyenv-native] Running pyenv self-install'
    & $exe @arguments
    if ($LASTEXITCODE -ne 0) { throw "pyenv self-install failed (exit code $LASTEXITCODE)." }
} finally {
    if (-not $KeepDownloads) {
        Remove-Item -Recurse -Force -LiteralPath $work -ErrorAction SilentlyContinue
    }
}
