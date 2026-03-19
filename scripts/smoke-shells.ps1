# ./scripts/smoke-shells.ps1
<#
Purpose: Smoke-test Windows shell integration for pyenv-native across PowerShell 7, Windows PowerShell 5.1, and CMD using a temporary portable root.
How to run: powershell -ExecutionPolicy Bypass -File .\scripts\smoke-shells.ps1 [-PyenvExe <path>]
Inputs: Optional path to a built pyenv.exe binary. Defaults to ../target/debug/pyenv.exe.
Outputs/side effects: Launches each target shell in a temporary workspace, evaluates generated init code, and verifies that `pyenv shell 3.13.12` resolves correctly.
Notes: Designed for CI smoke coverage of shell wrappers and dotted-version forwarding behavior on Windows.
#>

param(
    [string]$PyenvExe = (Join-Path $PSScriptRoot '..\target\debug\pyenv.exe')
)

$ErrorActionPreference = 'Stop'

$resolvedPyenvExe = (Resolve-Path $PyenvExe).ProviderPath
$smokeRoot = Join-Path (Resolve-Path (Join-Path $PSScriptRoot '..')).ProviderPath '.tmp-shell-smoke-windows'
if (Test-Path $smokeRoot) {
    Remove-Item -Recurse -Force $smokeRoot
}

function Invoke-Smoke {
    param(
        [string]$Name,
        [string]$Executable,
        [string]$Command
    )

    Write-Host "Smoke testing $Name..." -ForegroundColor Cyan
    & $Executable -NoLogo -NoProfile -Command $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Name smoke test failed with exit code $LASTEXITCODE"
    }
}

function New-SmokeLayout {
    param(
        [string]$Name
    )

    $layoutRoot = Join-Path $smokeRoot $Name
    $layoutPyenvRoot = Join-Path $layoutRoot '.pyenv'
    $layoutWorkDir = Join-Path $layoutRoot 'work'
    New-Item -ItemType Directory -Force -Path (Join-Path $layoutPyenvRoot 'versions\3.13.12') | Out-Null
    New-Item -ItemType Directory -Force -Path $layoutWorkDir | Out-Null
    return @{
        Root = $layoutRoot
        PyenvRoot = $layoutPyenvRoot
        WorkDir = $layoutWorkDir
    }
}

$pwshLayout = New-SmokeLayout -Name 'pwsh'
$ps5Layout = New-SmokeLayout -Name 'powershell'
$cmdLayout = New-SmokeLayout -Name 'cmd'

$pwshCommand = @"
`$env:PYENV_ROOT = '$($pwshLayout.PyenvRoot.Replace("'", "''"))'
Set-Location '$($pwshLayout.WorkDir.Replace("'", "''"))'
iex ((& '$($resolvedPyenvExe.Replace("'", "''"))' init --no-rehash - pwsh) -join [Environment]::NewLine)
pyenv shell 3.13.12
if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE }
`$result = & '$($resolvedPyenvExe.Replace("'", "''"))' version-name
if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE }
if ((`$result | Select-Object -Last 1).Trim() -ne '3.13.12') { throw 'Expected version-name to resolve to 3.13.12' }
"@

$ps5Command = @"
`$env:PYENV_ROOT = '$($ps5Layout.PyenvRoot.Replace("'", "''"))'
Set-Location '$($ps5Layout.WorkDir.Replace("'", "''"))'
iex ((& '$($resolvedPyenvExe.Replace("'", "''"))' init --no-rehash - pwsh) -join [Environment]::NewLine)
pyenv shell 3.13.12
if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE }
`$result = & '$($resolvedPyenvExe.Replace("'", "''"))' version-name
if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE }
if ((`$result | Select-Object -Last 1).Trim() -ne '3.13.12') { throw 'Expected version-name to resolve to 3.13.12' }
"@

Invoke-Smoke -Name 'PowerShell 7' -Executable 'pwsh' -Command $pwshCommand
Invoke-Smoke -Name 'Windows PowerShell 5.1' -Executable 'powershell' -Command $ps5Command

$cmdScript = @"
@echo off
setlocal
set "PYENV_ROOT=$($cmdLayout.PyenvRoot)"
cd /d "$($cmdLayout.WorkDir)"
findstr /c:"doskey pyenv=" "$($cmdLayout.Root)\init.cmd" >nul || exit /b 1
set "PYENV_SHELL=cmd"
for /f "delims=" %%i in ('"$resolvedPyenvExe" sh-cmd shell 3.13.12') do %%i
if errorlevel 1 exit /b 1
for /f "delims=" %%i in ('"$resolvedPyenvExe" version-name') do set "RESULT=%%i"
if /I not "%RESULT%"=="3.13.12" (
  echo Expected version-name to resolve to 3.13.12 but found %RESULT%
  exit /b 1
)
"@
$cmdInitScriptPath = Join-Path $cmdLayout.Root 'init.cmd'
& $resolvedPyenvExe init --no-rehash - cmd | Set-Content -Path $cmdInitScriptPath -Encoding ascii
$cmdScriptPath = Join-Path $cmdLayout.Root 'smoke.cmd'
Set-Content -Path $cmdScriptPath -Value $cmdScript -Encoding ascii
Write-Host 'Smoke testing CMD...' -ForegroundColor Cyan
cmd /c $cmdScriptPath
if ($LASTEXITCODE -ne 0) {
    throw "CMD smoke test failed with exit code $LASTEXITCODE"
}

Write-Host 'All Windows shell smoke tests passed.' -ForegroundColor Green
