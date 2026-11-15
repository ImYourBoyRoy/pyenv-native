# ./scripts/smoke-shells.ps1
<#
Purpose: Smoke-test Windows shell integration for Enchi across PowerShell 7, Windows PowerShell 5.1, and CMD using a temporary portable root.
How to run: pwsh -NoLogo -NoProfile -File .\scripts\smoke-shells.ps1 [-EnchiExe <path>]
Inputs: Optional path to a built enchi.exe binary (-PyenvExe is accepted as an alias). Defaults to ../target/debug/enchi.exe.
Outputs/side effects: Launches each target shell in a temporary workspace, evaluates generated init code, and verifies that the shell wrapper (`enchi`, or the `pyenv` compatibility alias, whichever the init code defines) resolves `shell 3.13.12` correctly.
Notes: Designed for CI smoke coverage of shell wrappers and dotted-version forwarding behavior on Windows.
#>

param(
    [Alias('PyenvExe')]
    [string]$EnchiExe = (Join-Path $PSScriptRoot '..\target\debug\enchi.exe')
)

$ErrorActionPreference = 'Stop'

$resolvedEnchiExe = (Resolve-Path $EnchiExe).ProviderPath
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
    $layoutInstallRoot = Join-Path $layoutRoot '.enchi'
    $layoutWorkDir = Join-Path $layoutRoot 'work'
    New-Item -ItemType Directory -Force -Path (Join-Path $layoutInstallRoot 'versions\3.13.12') | Out-Null
    New-Item -ItemType Directory -Force -Path $layoutWorkDir | Out-Null
    return @{
        Root = $layoutRoot
        EnchiRoot = $layoutInstallRoot
        WorkDir = $layoutWorkDir
    }
}

$pwshLayout = New-SmokeLayout -Name 'pwsh'
$ps5Layout = New-SmokeLayout -Name 'powershell'
$cmdLayout = New-SmokeLayout -Name 'cmd'

$pwshCommand = @"
`$env:ENCHI_ROOT = '$($pwshLayout.EnchiRoot.Replace("'", "''"))'
Set-Location '$($pwshLayout.WorkDir.Replace("'", "''"))'
`$__init = (& '$($resolvedEnchiExe.Replace("'", "''"))' init --no-rehash - pwsh) -join [Environment]::NewLine
if (-not `$__init) { throw 'Expected non-empty init output' }
Invoke-Expression `$__init
# The init code defines the `enchi` wrapper function (and possibly the `pyenv` alias); test whichever exists.
`$__wrapper = if (Get-Command enchi -CommandType Function -ErrorAction SilentlyContinue) { 'enchi' } else { 'pyenv' }
# Regression: init must still return capturable stdout after the wrapper function shadows the exe.
`$__init2 = (& `$__wrapper init --no-rehash - pwsh) -join [Environment]::NewLine
if (-not `$__init2) { throw "`$__wrapper init returned empty after function load (stdout not pipelined)" }
if (`$__init2 -notmatch ('function ' + `$__wrapper)) { throw "Captured init output missing the `$__wrapper function" }
& `$__wrapper shell 3.13.12
if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE }
`$result = & '$($resolvedEnchiExe.Replace("'", "''"))' version-name
if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE }
if ((`$result | Select-Object -Last 1).Trim() -ne '3.13.12') { throw 'Expected version-name to resolve to 3.13.12' }
"@

$ps5Command = @"
`$env:ENCHI_ROOT = '$($ps5Layout.EnchiRoot.Replace("'", "''"))'
Set-Location '$($ps5Layout.WorkDir.Replace("'", "''"))'
`$__init = (& '$($resolvedEnchiExe.Replace("'", "''"))' init --no-rehash - pwsh) -join [Environment]::NewLine
if (-not `$__init) { throw 'Expected non-empty init output' }
Invoke-Expression `$__init
`$__wrapper = if (Get-Command enchi -CommandType Function -ErrorAction SilentlyContinue) { 'enchi' } else { 'pyenv' }
`$__init2 = (& `$__wrapper init --no-rehash - pwsh) -join [Environment]::NewLine
if (-not `$__init2) { throw "`$__wrapper init returned empty after function load (stdout not pipelined)" }
if (`$__init2 -notmatch ('function ' + `$__wrapper)) { throw "Captured init output missing the `$__wrapper function" }
& `$__wrapper shell 3.13.12
if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE }
`$result = & '$($resolvedEnchiExe.Replace("'", "''"))' version-name
if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE }
if ((`$result | Select-Object -Last 1).Trim() -ne '3.13.12') { throw 'Expected version-name to resolve to 3.13.12' }
"@

Invoke-Smoke -Name 'PowerShell 7' -Executable 'pwsh' -Command $pwshCommand
Invoke-Smoke -Name 'Windows PowerShell 5.1' -Executable 'powershell' -Command $ps5Command

$cmdScript = @"
@echo off
setlocal
set "ENCHI_ROOT=$($cmdLayout.EnchiRoot)"
cd /d "$($cmdLayout.WorkDir)"
findstr /c:"doskey enchi=" "$($cmdLayout.Root)\init.cmd" >nul || findstr /c:"doskey pyenv=" "$($cmdLayout.Root)\init.cmd" >nul || exit /b 1
set "PYENV_SHELL=cmd"
for /f "delims=" %%i in ('"$resolvedEnchiExe" sh-cmd shell 3.13.12') do %%i
if errorlevel 1 exit /b 1
for /f "delims=" %%i in ('"$resolvedEnchiExe" version-name') do set "RESULT=%%i"
if /I not "%RESULT%"=="3.13.12" (
  echo Expected version-name to resolve to 3.13.12 but found %RESULT%
  exit /b 1
)
"@
$cmdInitScriptPath = Join-Path $cmdLayout.Root 'init.cmd'
& $resolvedEnchiExe init --no-rehash - cmd | Set-Content -Path $cmdInitScriptPath -Encoding ascii
$cmdScriptPath = Join-Path $cmdLayout.Root 'smoke.cmd'
Set-Content -Path $cmdScriptPath -Value $cmdScript -Encoding ascii
Write-Host 'Smoke testing CMD...' -ForegroundColor Cyan
cmd /c $cmdScriptPath
if ($LASTEXITCODE -ne 0) {
    throw "CMD smoke test failed with exit code $LASTEXITCODE"
}

Write-Host 'All Windows shell smoke tests passed.' -ForegroundColor Green
