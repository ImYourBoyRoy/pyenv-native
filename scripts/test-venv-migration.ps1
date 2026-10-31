[CmdletBinding()]
param(
    [string]$SourceRuntime = '3.14.3',
    [string]$TargetRuntime = '3.15.2',
    [string]$PyenvBinary = ''
)

# Cross-platform managed-venv migration smoke test for Windows.
# The test uses a temporary PYENV_ROOT and junctions to existing runtimes only.

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if (-not $PyenvBinary) {
    $PyenvBinary = Join-Path $repoRoot 'target\release\pyenv.exe'
}
if (-not (Test-Path -LiteralPath $PyenvBinary)) {
    throw "migration test requires an executable release CLI at $PyenvBinary"
}

$installedPyenv = (Get-Command pyenv.exe -ErrorAction SilentlyContinue).Source
if (-not $installedPyenv) {
    throw 'migration test requires an installed pyenv-native CLI on PATH'
}
$installedRoot = if ($env:INSTALLED_PYENV_ROOT) {
    $env:INSTALLED_PYENV_ROOT
} else {
    (& $installedPyenv root | Out-String).Trim()
}
$sourceInstall = Join-Path $installedRoot "versions\$SourceRuntime"
$targetInstall = Join-Path $installedRoot "versions\$TargetRuntime"
$sourcePython = Join-Path $sourceInstall 'python.exe'
$targetPython = Join-Path $targetInstall 'python.exe'

foreach ($required in @($sourceInstall, $targetInstall, $sourcePython, $targetPython)) {
    if (-not (Test-Path -LiteralPath $required)) {
        throw "migration test runtime path is missing: $required"
    }
}

$targetParts = $TargetRuntime.Split('.')
$targetPrefix = "$($targetParts[0]).$($targetParts[1])"
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ("pyenv-native-migration-" + [guid]::NewGuid().ToString('N'))
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
if (-not $testRoot.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw "refusing to use an unexpected temporary root: $testRoot"
}

$environmentNames = @(
    'PYTHONDONTWRITEBYTECODE',
    'PYTHONNOUSERSITE',
    'PIP_DISABLE_PIP_VERSION_CHECK',
    'PIP_PROGRESS_BAR',
    'PIP_CONFIG_FILE',
    'PIP_CACHE_DIR',
    'PIP_NO_INDEX',
    'PIP_FIND_LINKS',
    'PYENV_ROOT'
)
$oldEnvironment = @{}
foreach ($name in $environmentNames) {
    $oldEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}

try {
    New-Item -ItemType Directory -Force -Path $testRoot | Out-Null
    $env:PYENV_ROOT = $testRoot
    New-Item -ItemType Directory -Force -Path (Join-Path $testRoot 'versions') | Out-Null
    New-Item -ItemType Junction -Path (Join-Path $testRoot "versions\$SourceRuntime") -Target $sourceInstall | Out-Null
    New-Item -ItemType Junction -Path (Join-Path $testRoot "versions\$TargetRuntime") -Target $targetInstall | Out-Null

    $env:PYTHONDONTWRITEBYTECODE = '1'
    $env:PYTHONNOUSERSITE = '1'
    $env:PIP_DISABLE_PIP_VERSION_CHECK = '1'
    $env:PIP_PROGRESS_BAR = 'off'

    $fixtureDir = Join-Path $testRoot 'fixtures'
    & $sourcePython (Join-Path $repoRoot 'scripts\venv_migration_fixture.py') $fixtureDir $TargetRuntime | Out-Null

    $sourceEnv = Join-Path $testRoot "venvs\$SourceRuntime\migration-test"
    $targetEnv = Join-Path $testRoot "venvs\$TargetRuntime\migration-test"
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $sourceEnv) | Out-Null
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $targetEnv) | Out-Null

    $env:PIP_CONFIG_FILE = Join-Path $testRoot 'pip.conf'
    $env:PIP_CACHE_DIR = Join-Path $testRoot 'pip-cache'
    New-Item -ItemType File -Force -Path $env:PIP_CONFIG_FILE | Out-Null
    & $sourcePython -m venv $sourceEnv
    & $targetPython -m venv $targetEnv

    $sourceVenvPython = Join-Path $sourceEnv 'Scripts\python.exe'
    $targetVenvPython = Join-Path $targetEnv 'Scripts\python.exe'
    $env:PIP_NO_INDEX = '1'
    $env:PIP_FIND_LINKS = $fixtureDir
    & $sourceVenvPython -m pip install --no-index --find-links $fixtureDir 'migration-compat-demo==1.0.0' | Out-Null

    Set-Content -LiteralPath (Join-Path $testRoot '.python-version') -Value "$SourceRuntime/envs/migration-test" -NoNewline
    Push-Location $testRoot
    try {
        $upgradeOutput = & $PyenvBinary venv upgrade --force --set-local "$SourceRuntime/envs/migration-test" $targetPrefix 2>&1 | Out-String
        $upgradeStatus = $LASTEXITCODE
    } finally {
        Pop-Location
    }
    Write-Output $upgradeOutput.TrimEnd()
    if ($upgradeStatus -ne 0) {
        throw "managed-venv migration failed with exit code $upgradeStatus"
    }

    $probeCode = "import migration_compat_demo,sys; print('python=' + '.'.join(str(value) for value in sys.version_info[:3])); print('fixture=' + migration_compat_demo.__version__)"
    $targetProbe = (& $targetVenvPython -c $probeCode | Out-String).Trim()
    Write-Output $targetProbe.TrimEnd()
    if ($targetProbe -notmatch '(?m)^fixture=2\.0\.0$') {
        throw "compatibility package was not upgraded to 2.0.0: $targetProbe"
    }

    $localSpec = (Get-Content -LiteralPath (Join-Path $testRoot '.python-version') -Raw).Trim()
    if ($localSpec -ne "$TargetRuntime/envs/migration-test") {
        throw "local selection was not retargeted: $localSpec"
    }
    if (Test-Path -LiteralPath $sourceEnv) {
        throw "source managed venv still exists: $sourceEnv"
    }
    if (-not (Test-Path -LiteralPath $targetVenvPython)) {
        throw "target interpreter is missing: $targetVenvPython"
    }
    $previous = Get-ChildItem -LiteralPath (Split-Path -Parent $targetEnv) -Directory -Filter 'migration-test-previous*' -ErrorAction SilentlyContinue
    if ($previous) {
        throw "rollback copy was not cleaned up: $($previous.FullName -join ', ')"
    }

    $incompatibleFixtureDir = Join-Path $testRoot 'fixtures-incompatible'
    & $sourcePython (Join-Path $repoRoot 'scripts\venv_migration_fixture.py') $incompatibleFixtureDir $TargetRuntime | Out-Null
    Remove-Item -LiteralPath (Join-Path $incompatibleFixtureDir 'migration_compat_demo-2.0.0-py3-none-any.whl') -Force
    $failureEnv = Join-Path $testRoot "venvs\$SourceRuntime\failure-test"
    $failureTarget = Join-Path $testRoot "venvs\$TargetRuntime\failure-test"
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $failureEnv) | Out-Null
    & $sourcePython -m venv $failureEnv
    $failureVenvPython = Join-Path $failureEnv 'Scripts\python.exe'
    $env:PIP_FIND_LINKS = $incompatibleFixtureDir
    & $failureVenvPython -m pip install --no-index --find-links $incompatibleFixtureDir 'migration-compat-demo==1.0.0' | Out-Null

    $failureStderrPath = Join-Path $testRoot 'migration-failure.stderr'
    $failureErrorAction = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    Push-Location $testRoot
    try {
        $failureOutput = & $PyenvBinary venv upgrade --force "$SourceRuntime/envs/failure-test" $targetPrefix 2> $failureStderrPath | Out-String
        $failureStatus = $LASTEXITCODE
        if (Test-Path -LiteralPath $failureStderrPath) {
            $failureOutput += Get-Content -LiteralPath $failureStderrPath -Raw
        }
    } finally {
        Pop-Location
        $ErrorActionPreference = $failureErrorAction
    }
    if ($failureStatus -eq 0) {
        throw 'incompatible migration unexpectedly succeeded'
    }
    Write-Output "expected incompatible migration failure observed (exit code $failureStatus); source remained intact"
    if (-not (Test-Path -LiteralPath $failureEnv)) {
        throw "failed migration removed the source venv: $failureEnv"
    }
    if (Test-Path -LiteralPath $failureTarget) {
        throw "failed migration left a target venv: $failureTarget"
    }
    $failureStaging = Get-ChildItem -LiteralPath (Split-Path -Parent $failureTarget) -Directory -Filter 'failure-test-migrating*' -ErrorAction SilentlyContinue
    if ($failureStaging) {
        throw "failed migration left staging venvs: $($failureStaging.FullName -join ', ')"
    }

    Write-Output "managed-venv migration smoke test passed: $SourceRuntime -> $TargetRuntime"
    exit 0
} finally {
    foreach ($name in $environmentNames) {
        [Environment]::SetEnvironmentVariable($name, $oldEnvironment[$name], 'Process')
    }
    if ((Test-Path -LiteralPath $testRoot) -and
        $testRoot.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -and
        (Split-Path -Leaf $testRoot) -match '^pyenv-native-migration-[0-9a-f-]+$') {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
