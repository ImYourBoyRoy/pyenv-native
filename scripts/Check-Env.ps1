# ./scripts/Check-Env.ps1
<#
Purpose: Verifies the local Windows development environment for pyenv-native, including optional Android cross-build prerequisites.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/Check-Env.ps1 [-RequireAndroid]
Inputs: Optional switch requiring Android build readiness in addition to the base Rust/PowerShell checks.
Outputs/side effects: Prints a readiness summary for Rust, Cargo, pyenv visibility, and Android tooling such as rustup targets, cargo-ndk, and Android NDK discovery.
Notes: Intended for contributors on Windows; Android checks are advisory by default and become required only when -RequireAndroid is supplied.
#>

param(
    [switch]$RequireAndroid
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

Write-Host "--- pyenv-native dev environment check ---" -ForegroundColor Cyan

$Success = $true
$AndroidReady = $true

function Write-Status {
    param(
        [ValidateSet('OK', 'WARN', 'FAIL', 'INFO')]
        [string]$Level,
        [string]$Message
    )

    $color = switch ($Level) {
        'OK' { 'Green' }
        'WARN' { 'Yellow' }
        'FAIL' { 'Red' }
        default { 'Gray' }
    }

    Write-Host "[$Level] $Message" -ForegroundColor $color
}

function Invoke-NativeLines {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Executable,
        [string[]]$Arguments = @()
    )

    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $Executable
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true

    $quotedArguments = @($Arguments | ForEach-Object {
            if ($_ -match '\s|"') {
                '"' + ($_ -replace '"', '\"') + '"'
            } else {
                $_
            }
        }) -join ' '
    $startInfo.Arguments = $quotedArguments

    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $startInfo
    [void]$process.Start()
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    $global:LASTEXITCODE = $process.ExitCode

    $lines = @()
    if ($stdout) {
        $lines += ($stdout -split "`r?`n" | Where-Object { $_ -ne '' })
    }
    if ($stderr) {
        $lines += ($stderr -split "`r?`n" | Where-Object { $_ -ne '' })
    }

    return $lines
}

function Find-AndroidNdkRoot {
    $candidates = @()

    foreach ($envVar in @('ANDROID_NDK_HOME', 'ANDROID_NDK_ROOT')) {
        $value = [Environment]::GetEnvironmentVariable($envVar)
        if (-not [string]::IsNullOrWhiteSpace($value)) {
            $candidates += $value
        }
    }

    $sdkRoot = [Environment]::GetEnvironmentVariable('ANDROID_HOME')
    if ([string]::IsNullOrWhiteSpace($sdkRoot)) {
        $sdkRoot = [Environment]::GetEnvironmentVariable('ANDROID_SDK_ROOT')
    }
    if (-not [string]::IsNullOrWhiteSpace($sdkRoot)) {
        $ndkFolder = Join-Path $sdkRoot 'ndk'
        if (Test-Path $ndkFolder) {
            $candidates += Get-ChildItem $ndkFolder -Directory -ErrorAction SilentlyContinue |
                Sort-Object Name -Descending |
                Select-Object -ExpandProperty FullName
        }
    }

    if ($env:LOCALAPPDATA) {
        $defaultSdkRoot = Join-Path $env:LOCALAPPDATA 'Android\Sdk'
        $defaultNdkFolder = Join-Path $defaultSdkRoot 'ndk'
        if (Test-Path $defaultNdkFolder) {
            $candidates += Get-ChildItem $defaultNdkFolder -Directory -ErrorAction SilentlyContinue |
                Sort-Object Name -Descending |
                Select-Object -ExpandProperty FullName
        }
    }

    foreach ($candidate in $candidates | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) {
        if (Test-Path $candidate) {
            return (Resolve-Path $candidate).ProviderPath
        }
    }

    return $null
}

# 1. Check for Rust / Cargo
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    $CargoVersion = cargo --version
    Write-Status -Level OK -Message "Cargo found: $CargoVersion"
} else {
    Write-Status -Level FAIL -Message 'Cargo NOT found. Please install Rust from https://rustup.rs'
    $Success = $false
}

# 2. Check for rustc
if (Get-Command rustc -ErrorAction SilentlyContinue) {
    $RustcVersion = rustc --version
    Write-Status -Level OK -Message "Rustc found: $RustcVersion"
} else {
    Write-Status -Level FAIL -Message 'Rustc NOT found.'
    $Success = $false
}

# 3. Check for rustup and Android target availability
$rustup = Get-Command rustup -ErrorAction SilentlyContinue
if ($rustup) {
    $rustupVersion = Invoke-NativeLines -Executable $rustup.Source -Arguments @('--version') | Select-Object -First 1
    Write-Status -Level OK -Message "rustup found: $rustupVersion"

    $installedTargets = @(Invoke-NativeLines -Executable $rustup.Source -Arguments @('target', 'list', '--installed'))
    if ($installedTargets -contains 'aarch64-linux-android') {
        Write-Status -Level OK -Message 'Rust Android target installed: aarch64-linux-android'
    } else {
        Write-Status -Level WARN -Message 'Rust Android target missing: run `rustup target add aarch64-linux-android` to build Android artifacts locally.'
        $AndroidReady = $false
    }
} else {
    Write-Status -Level WARN -Message 'rustup not found; Android target installation cannot be verified.'
    $AndroidReady = $false
}

# 4. Check for PowerShell version
$PSVersion = $PSVersionTable.PSVersion
if ($PSVersion.Major -ge 5) {
    Write-Status -Level OK -Message "PowerShell version: $PSVersion"
} else {
    Write-Status -Level WARN -Message "PowerShell version is old ($PSVersion). 5.1 or 7+ recommended."
}

# 5. Check for pyenv-native on PATH (optional)
if (Get-Command pyenv -ErrorAction SilentlyContinue) {
    $PyenvCommand = Get-Command pyenv | Select-Object -First 1
    $PyenvSource = if ($PyenvCommand.Source) { $PyenvCommand.Source } else { $PyenvCommand.CommandType }
    Write-Status -Level INFO -Message "pyenv found at: $PyenvSource"
} else {
    Write-Status -Level INFO -Message "pyenv not currently on system PATH. This is expected if you haven't installed it yet."
}

# 6. Android cross-build readiness
if (Get-Command cargo-ndk -ErrorAction SilentlyContinue) {
    $cargoNdkVersion = Invoke-NativeLines -Executable 'cargo-ndk' -Arguments @('--version')
    if ($LASTEXITCODE -eq 0 -and $cargoNdkVersion) {
        Write-Status -Level OK -Message "cargo-ndk found: $($cargoNdkVersion | Select-Object -First 1)"
    } else {
        Write-Status -Level OK -Message 'cargo-ndk found on PATH.'
    }
} else {
    Write-Status -Level WARN -Message 'cargo-ndk not found. Install it with `cargo install cargo-ndk --locked` for local Android bundle builds.'
    $AndroidReady = $false
}

$AndroidNdkRoot = Find-AndroidNdkRoot
if ($AndroidNdkRoot) {
    Write-Status -Level OK -Message "Android NDK found: $AndroidNdkRoot"
} else {
    Write-Status -Level WARN -Message 'Android NDK not found. Set ANDROID_NDK_HOME/ANDROID_NDK_ROOT or install the Android NDK under %LOCALAPPDATA%\Android\Sdk\ndk.'
    $AndroidReady = $false
}

$Java = Get-Command java -ErrorAction SilentlyContinue
if ($Java) {
    $JavaVersion = Invoke-NativeLines -Executable $Java.Source -Arguments @('-version') | Select-Object -First 1
    Write-Status -Level OK -Message "Java found: $JavaVersion"
} else {
    Write-Status -Level WARN -Message 'Java not found. Some Android SDK/NDK management workflows are easier with a local JDK installed.'
}

if ($AndroidReady) {
    Write-Status -Level OK -Message 'Android cross-build prerequisites look ready.'
} elseif ($RequireAndroid) {
    Write-Status -Level FAIL -Message 'Android cross-build prerequisites are missing, and -RequireAndroid was requested.'
    $Success = $false
} else {
    Write-Status -Level INFO -Message 'Android cross-build prerequisites are incomplete locally, but GitHub Actions can still build Android artifacts.'
}

if ($Success) {
    Write-Host "--- Environment looks READY for development ---" -ForegroundColor Green
} else {
    Write-Host "--- Environment is MISSING prerequisites ---" -ForegroundColor Red
    exit 1
}
