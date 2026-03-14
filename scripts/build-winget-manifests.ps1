# ./scripts/build-winget-manifests.ps1
<#
Purpose: Generates Winget portable-package manifests for the built Windows pyenv-native bundle.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/build-winget-manifests.ps1 [-BundlePath ./dist/pyenv-native-windows-x64.zip] [-GitHubRepo imyourboyroy/pyenv-native] [-Validate]
Inputs: Built Windows bundle zip/checksum, package metadata, and either a release base URL or GitHub repo/tag for installer URL generation.
Outputs/side effects: Writes version/defaultLocale/installer YAML manifests under packaging/winget/manifests/... and can optionally run winget validate.
Notes: This prepares publish-ready Winget metadata without submitting anything to winget-pkgs.
#>

param(
    [string]$BundlePath = (Join-Path $PSScriptRoot '..\dist\pyenv-native-windows-x64.zip'),
    [string]$ChecksumPath = '',
    [string]$OutputRoot = (Join-Path $PSScriptRoot '..\packaging\winget'),
    [string]$PackageIdentifier = 'ImYourBoyRoy.pyenv-native',
    [string]$PackageLocale = 'en-US',
    [string]$PackageName = 'pyenv-native',
    [string]$Publisher = 'Roy Dawson IV',
    [string]$PublisherUrl = 'https://github.com/imyourboyroy',
    [string]$PublisherSupportUrl = 'https://github.com/imyourboyroy/pyenv-native/issues',
    [string]$PackageUrl = 'https://github.com/imyourboyroy/pyenv-native',
    [string]$Moniker = 'pyenv',
    [string]$ManifestVersion = '1.12.0',
    [string]$GitHubRepo = 'imyourboyroy/pyenv-native',
    [string]$Tag = '',
    [string]$ReleaseBaseUrl = '',
    [string]$ShortDescription = 'Native-first cross-platform Python version manager compatible with pyenv workflows.',
    [string]$Description = 'Portable, native-first Python version manager that preserves familiar pyenv workflows across Windows, Linux, and macOS.',
    [switch]$Validate
)

$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.IO.Compression.FileSystem

function Get-BundleManifest {
    param(
        [Parameter(Mandatory)]
        [string]$ArchivePath
    )

    $zip = [System.IO.Compression.ZipFile]::OpenRead($ArchivePath)
    try {
        $entry = $zip.Entries | Where-Object { $_.FullName -eq 'bundle-manifest.json' } | Select-Object -First 1
        if (-not $entry) {
            throw "bundle-manifest.json was not found in $ArchivePath"
        }

        $stream = $entry.Open()
        $reader = New-Object System.IO.StreamReader($stream)
        try {
            return ($reader.ReadToEnd() | ConvertFrom-Json)
        }
        finally {
            $reader.Dispose()
            $stream.Dispose()
        }
    }
    finally {
        $zip.Dispose()
    }
}

function Get-ChecksumValue {
    param(
        [Parameter(Mandatory)]
        [string]$Path
    )

    $content = Get-Content -LiteralPath $Path -Raw
    $match = [regex]::Match($content, '(?im)\b([0-9a-f]{64})\b')
    if (-not $match.Success) {
        throw "Failed to read a SHA-256 value from $Path"
    }
    $match.Groups[1].Value.ToLowerInvariant()
}

function Join-PackagePathSegments {
    param(
        [Parameter(Mandatory)]
        [string]$Identifier
    )

    $segments = $Identifier.Split('.')
    if ($segments.Count -lt 2) {
        throw "PackageIdentifier must include a publisher and package segment"
    }

    $firstLetter = $segments[0].Substring(0, 1).ToLowerInvariant()
    $remainingSegments = $segments[1..($segments.Count - 1)]
    $path = Join-Path $OutputRoot 'manifests'
    foreach ($segment in (@($firstLetter, $segments[0]) + $remainingSegments)) {
        $path = Join-Path $path $segment
    }
    $path
}

$resolvedBundlePath = (Resolve-Path $BundlePath).Path
if ([string]::IsNullOrWhiteSpace($ChecksumPath)) {
    $ChecksumPath = $resolvedBundlePath + '.sha256'
}
$resolvedChecksumPath = (Resolve-Path $ChecksumPath).Path

$bundleManifest = Get-BundleManifest -ArchivePath $resolvedBundlePath
if ($bundleManifest.platform -ne 'windows') {
    throw "Winget manifests can only be generated from the Windows bundle. Found platform '$($bundleManifest.platform)'."
}

$bundleFileName = Split-Path -Leaf $resolvedBundlePath
$bundleVersion = [string]$bundleManifest.bundle_version
$effectiveTag = if ([string]::IsNullOrWhiteSpace($Tag)) { "v$bundleVersion" } else { $Tag }
$effectiveReleaseBaseUrl = if (-not [string]::IsNullOrWhiteSpace($ReleaseBaseUrl)) {
    $ReleaseBaseUrl.TrimEnd('/')
}
else {
    "https://github.com/$GitHubRepo/releases/download/$effectiveTag"
}
$installerUrl = "$effectiveReleaseBaseUrl/$bundleFileName"
$installerSha256 = Get-ChecksumValue -Path $resolvedChecksumPath

$packagePath = Join-PackagePathSegments -Identifier $PackageIdentifier
$manifestDirectory = Join-Path $packagePath $bundleVersion
New-Item -ItemType Directory -Force -Path $manifestDirectory | Out-Null

$versionManifestPath = Join-Path $manifestDirectory "$PackageIdentifier.yaml"
$localeManifestPath = Join-Path $manifestDirectory "$PackageIdentifier.locale.$PackageLocale.yaml"
$installerManifestPath = Join-Path $manifestDirectory "$PackageIdentifier.installer.yaml"

$licenseUrl = "$PackageUrl/blob/$effectiveTag/LICENSE"
$releaseNotesUrl = "$PackageUrl/releases/tag/$effectiveTag"

$versionManifest = @"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.version.$ManifestVersion.schema.json
PackageIdentifier: $PackageIdentifier
PackageVersion: $bundleVersion
DefaultLocale: $PackageLocale
ManifestType: version
ManifestVersion: $ManifestVersion
"@

$localeManifest = @"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.defaultLocale.$ManifestVersion.schema.json
PackageIdentifier: $PackageIdentifier
PackageVersion: $bundleVersion
PackageLocale: $PackageLocale
Publisher: $Publisher
PublisherUrl: $PublisherUrl
PublisherSupportUrl: $PublisherSupportUrl
Author: $Publisher
PackageName: $PackageName
PackageUrl: $PackageUrl
License: MIT
LicenseUrl: $licenseUrl
ShortDescription: $ShortDescription
Description: $Description
Moniker: $Moniker
Tags:
  - pyenv
  - python
  - version-manager
  - virtualenv
ReleaseNotesUrl: $releaseNotesUrl
ManifestType: defaultLocale
ManifestVersion: $ManifestVersion
"@

$installerManifest = @"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.installer.$ManifestVersion.schema.json
PackageIdentifier: $PackageIdentifier
PackageVersion: $bundleVersion
InstallerType: zip
NestedInstallerType: portable
NestedInstallerFiles:
  - RelativeFilePath: $($bundleManifest.executable)
    PortableCommandAlias: pyenv
ArchiveBinariesDependOnPath: true
Installers:
  - Architecture: $($bundleManifest.architecture)
    InstallerUrl: $installerUrl
    InstallerSha256: $installerSha256
ManifestType: installer
ManifestVersion: $ManifestVersion
"@

Set-Content -LiteralPath $versionManifestPath -Value $versionManifest -Encoding utf8
Set-Content -LiteralPath $localeManifestPath -Value $localeManifest -Encoding utf8
Set-Content -LiteralPath $installerManifestPath -Value $installerManifest -Encoding utf8

if ($Validate) {
    $winget = Get-Command winget -ErrorAction SilentlyContinue
    if (-not $winget) {
        throw "Winget validation was requested, but winget is not available on PATH."
    }
    & $winget.Source validate --manifest $manifestDirectory --disable-interactivity
    if ($LASTEXITCODE -ne 0) {
        throw "winget validate failed with exit code $LASTEXITCODE"
    }
}

[ordered]@{
    bundle_path = $resolvedBundlePath
    checksum_path = $resolvedChecksumPath
    manifest_directory = $manifestDirectory
    version_manifest = $versionManifestPath
    locale_manifest = $localeManifestPath
    installer_manifest = $installerManifestPath
    installer_url = $installerUrl
    package_identifier = $PackageIdentifier
} | ConvertTo-Json -Depth 4
