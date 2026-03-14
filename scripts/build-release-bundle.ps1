# ./scripts/build-release-bundle.ps1
<#
Purpose: Builds a release binary and assembles a portable Windows distribution bundle for pyenv-native.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/build-release-bundle.ps1 [-OutputRoot ./dist]
Inputs: Optional output root and bundle name override.
Outputs/side effects: Builds the release binary, writes a bundle directory under dist/, and creates a zip archive with installers and docs.
Notes: Intended for native Windows packaging; bundle contents stay portable and registry-free.
#>

param(
    [string]$OutputRoot = (Join-Path $PSScriptRoot '..\dist'),
    [string]$BundleName = 'pyenv-native-windows-x64'
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$resolvedOutputRoot = [System.IO.Path]::GetFullPath($OutputRoot)
$bundleDir = Join-Path $resolvedOutputRoot $BundleName
$archivePath = Join-Path $resolvedOutputRoot ($BundleName + '.zip')
$checksumPath = $archivePath + '.sha256'
$cargoTomlPath = Join-Path $repoRoot 'Cargo.toml'
$releaseExe = Join-Path $repoRoot 'target\x86_64-pc-windows-gnu\release\pyenv.exe'

& (Join-Path $PSScriptRoot 'dev-cargo.ps1') build --release
if ($LASTEXITCODE -ne 0) {
    throw "Release build failed with exit code $LASTEXITCODE"
}

if (-not (Test-Path $releaseExe)) {
    throw "Release binary was not found at $releaseExe"
}

if (Test-Path $bundleDir) {
    Remove-Item -Recurse -Force $bundleDir
}
New-Item -ItemType Directory -Force -Path $bundleDir | Out-Null

$cargoToml = Get-Content $cargoTomlPath -Raw
$versionMatch = [regex]::Match($cargoToml, '(?m)^\s*version\s*=\s*"([^"]+)"\s*$')
if (-not $versionMatch.Success) {
    throw "Failed to determine workspace version from $cargoTomlPath"
}
$bundleVersion = $versionMatch.Groups[1].Value

Copy-Item -Force $releaseExe (Join-Path $bundleDir 'pyenv.exe')
Copy-Item -Force (Join-Path $repoRoot 'README.md') (Join-Path $bundleDir 'README.md')
Copy-Item -Force (Join-Path $repoRoot 'LICENSE') (Join-Path $bundleDir 'LICENSE')
Copy-Item -Force (Join-Path $PSScriptRoot 'install-pyenv-native.ps1') (Join-Path $bundleDir 'install-pyenv-native.ps1')
Copy-Item -Force (Join-Path $PSScriptRoot 'uninstall-pyenv-native.ps1') (Join-Path $bundleDir 'uninstall-pyenv-native.ps1')

$cmdWrapper = "@echo off`r`n""%~dp0pyenv.exe"" %*`r`n"
$ps1Wrapper = "& ""$PSScriptRoot\pyenv.exe"" @args`r`nexit `$LASTEXITCODE`r`n"
Set-Content -Path (Join-Path $bundleDir 'pyenv.cmd') -Value $cmdWrapper -Encoding utf8
Set-Content -Path (Join-Path $bundleDir 'pyenv.ps1') -Value $ps1Wrapper -Encoding utf8

$bundleManifest = [ordered]@{
    bundle_name = $BundleName
    bundle_version = $bundleVersion
    platform = 'windows'
    architecture = 'x64'
    executable = 'pyenv.exe'
    install_script = 'install-pyenv-native.ps1'
    uninstall_script = 'uninstall-pyenv-native.ps1'
    command_wrappers = @('pyenv.cmd', 'pyenv.ps1')
}
$bundleManifest |
    ConvertTo-Json -Depth 4 |
    Set-Content -Path (Join-Path $bundleDir 'bundle-manifest.json') -Encoding utf8

if (Test-Path $archivePath) {
    Remove-Item -Force $archivePath
}
Compress-Archive -Path (Join-Path $bundleDir '*') -DestinationPath $archivePath
if (Test-Path $checksumPath) {
    Remove-Item -Force $checksumPath
}

$hash = Get-FileHash -Algorithm SHA256 $archivePath
'{0}  {1}' -f $hash.Hash.ToLowerInvariant(), (Split-Path -Leaf $archivePath) |
    Set-Content -Path $checksumPath -Encoding ascii

$summary = [ordered]@{
    repo_root = $repoRoot
    bundle_dir = $bundleDir
    archive_path = $archivePath
    checksum_path = $checksumPath
    release_exe = $releaseExe
}

$summary.GetEnumerator() | ForEach-Object {
    '{0}: {1}' -f $_.Key, $_.Value
}
