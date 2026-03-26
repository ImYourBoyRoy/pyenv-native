# ./scripts/build-release-bundle.ps1
<#
Purpose: Builds release binaries and assembles a portable Windows distribution bundle for pyenv-native.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/build-release-bundle.ps1 [-OutputRoot ./dist] [-TargetTriple x86_64-pc-windows-msvc]
Inputs: Optional output root, bundle name override, and Windows target triple.
Outputs/side effects: Builds the release binaries, writes a bundle directory under dist/, and creates a zip archive with installers, MCP server, and user-facing docs.
Notes: Intended for native Windows packaging; defaults to the MSVC ABI and derives the bundle architecture from the requested target triple.
#>

param(
    [string]$OutputRoot = (Join-Path $PSScriptRoot '..\dist'),
    [string]$BundleName = '',
    [string]$TargetTriple = $env:PYENV_WINDOWS_TARGET
)

$ErrorActionPreference = 'Stop'

function Get-TargetArchitecture {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Target
    )

    switch ($Target) {
        'x86_64-pc-windows-gnu' { return 'x64' }
        'x86_64-pc-windows-msvc' { return 'x64' }
        'aarch64-pc-windows-msvc' { return 'arm64' }
        default { throw "Unsupported Windows target triple '$Target'." }
    }
}

if (-not $TargetTriple -or [string]::IsNullOrWhiteSpace($TargetTriple)) {
    $TargetTriple = 'x86_64-pc-windows-msvc'
}

$architecture = Get-TargetArchitecture -Target $TargetTriple
if (-not $BundleName -or [string]::IsNullOrWhiteSpace($BundleName)) {
    $BundleName = "pyenv-native-windows-$architecture"
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$resolvedOutputRoot = [System.IO.Path]::GetFullPath($OutputRoot)
$bundleDir = Join-Path $resolvedOutputRoot $BundleName
$archivePath = Join-Path $resolvedOutputRoot ($BundleName + '.zip')
$checksumPath = $archivePath + '.sha256'
$cargoTomlPath = Join-Path $repoRoot 'Cargo.toml'
$releaseExe = Join-Path $repoRoot ("target\$TargetTriple\release\pyenv.exe")
$releaseMcpExe = Join-Path $repoRoot ("target\$TargetTriple\release\pyenv-mcp.exe")
$releaseGuiExe = Join-Path $repoRoot ("target\$TargetTriple\release\pyenv-gui.exe")


& (Join-Path $PSScriptRoot 'dev-cargo.ps1') -TargetTriple $TargetTriple build --release -p pyenv-cli -p pyenv-mcp -p pyenv-gui

if ($LASTEXITCODE -ne 0) {
    throw "Release build failed with exit code $LASTEXITCODE"
}

foreach ($requiredBinary in @($releaseExe, $releaseMcpExe, $releaseGuiExe)) {

    if (-not (Test-Path $requiredBinary)) {
        throw "Release binary was not found at $requiredBinary"
    }
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
Copy-Item -Force $releaseMcpExe (Join-Path $bundleDir 'pyenv-mcp.exe')
Copy-Item -Force $releaseGuiExe (Join-Path $bundleDir 'pyenv-gui.exe')

Copy-Item -Force (Join-Path $repoRoot 'README.md') (Join-Path $bundleDir 'README.md')
Copy-Item -Force (Join-Path $repoRoot 'INSTRUCTIONS.md') (Join-Path $bundleDir 'INSTRUCTIONS.md')
if (Test-Path (Join-Path $repoRoot 'MCP.md')) {
    Copy-Item -Force (Join-Path $repoRoot 'MCP.md') (Join-Path $bundleDir 'MCP.md')
}
Copy-Item -Force (Join-Path $repoRoot 'LICENSE') (Join-Path $bundleDir 'LICENSE')
Copy-Item -Force (Join-Path $PSScriptRoot 'install-pyenv-native.ps1') (Join-Path $bundleDir 'install-pyenv-native.ps1')
Copy-Item -Force (Join-Path $PSScriptRoot 'uninstall-pyenv-native.ps1') (Join-Path $bundleDir 'uninstall-pyenv-native.ps1')

$cmdWrapper = "@echo off`r`n""%~dp0pyenv.exe"" %*`r`n"
$ps1Wrapper = '& "$PSScriptRoot\pyenv.exe" @args' + "`r`n" + 'exit $LASTEXITCODE' + "`r`n"
$mcpCmdWrapper = "@echo off`r`n""%~dp0pyenv-mcp.exe"" %*`r`n"
$mcpPs1Wrapper = '& "$PSScriptRoot\pyenv-mcp.exe" @args' + "`r`n" + 'exit $LASTEXITCODE' + "`r`n"
Set-Content -Path (Join-Path $bundleDir 'pyenv.cmd') -Value $cmdWrapper -Encoding utf8
Set-Content -Path (Join-Path $bundleDir 'pyenv.ps1') -Value $ps1Wrapper -Encoding utf8
Set-Content -Path (Join-Path $bundleDir 'pyenv-mcp.cmd') -Value $mcpCmdWrapper -Encoding utf8
Set-Content -Path (Join-Path $bundleDir 'pyenv-mcp.ps1') -Value $mcpPs1Wrapper -Encoding utf8

$bundleManifest = [ordered]@{
    bundle_name = $BundleName
    bundle_version = $bundleVersion
    platform = 'windows'
    architecture = $architecture
    target_triple = $TargetTriple
    executable = 'pyenv.exe'
    mcp_executable = 'pyenv-mcp.exe'
    gui_executable = 'pyenv-gui.exe'
    install_script = 'install-pyenv-native.ps1'
    uninstall_script = 'uninstall-pyenv-native.ps1'
    command_wrappers = @('pyenv.cmd', 'pyenv.ps1', 'pyenv-mcp.cmd', 'pyenv-mcp.ps1')
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
    release_mcp_exe = $releaseMcpExe
    target_triple = $TargetTriple
}

$summary.GetEnumerator() | ForEach-Object {
    '{0}: {1}' -f $_.Key, $_.Value
}
