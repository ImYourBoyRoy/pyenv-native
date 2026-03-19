# ./scripts/dev-cargo.ps1
<#
Purpose: Runs Cargo for a configurable Windows Rust target, defaulting to the native MSVC toolchain while allowing GNU builds when explicitly requested.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/dev-cargo.ps1 test [-TargetTriple x86_64-pc-windows-msvc]
Inputs: Optional Windows target triple plus cargo arguments after the script name.
Outputs/side effects: Executes cargo with the requested target toolchain and any required compiler PATH updates for this process.
Notes: Prefers the native MSVC ABI for local and CI builds; GNU builds rely on WinLibs/MinGW only when that ABI is explicitly requested.
#>

param(
    [string]$TargetTriple = $env:PYENV_WINDOWS_TARGET,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$CargoArgs
)

$ErrorActionPreference = "Stop"

if ($TargetTriple -and $TargetTriple -notmatch '^(x86_64-pc-windows-(gnu|msvc)|aarch64-pc-windows-msvc)$' -and (-not $CargoArgs -or $CargoArgs.Count -eq 0)) {
    $CargoArgs = @($TargetTriple)
    $TargetTriple = $env:PYENV_WINDOWS_TARGET
}

if (-not $TargetTriple -or [string]::IsNullOrWhiteSpace($TargetTriple)) {
    $TargetTriple = 'x86_64-pc-windows-msvc'
}

if (-not $CargoArgs -or $CargoArgs.Count -eq 0) {
    $CargoArgs = @('test')
}

$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (-not (Test-Path $cargoBin)) {
    throw "Rust cargo bin path was not found at $cargoBin"
}

$env:PATH = "$cargoBin;$env:PATH"

switch ($TargetTriple) {
    'x86_64-pc-windows-gnu' {
        $winlibsRoot = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Directory -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -like 'BrechtSanders.WinLibs.POSIX.UCRT*' } |
            Select-Object -First 1 -ExpandProperty FullName

        $mingwBin = if ($winlibsRoot) {
            Join-Path $winlibsRoot 'mingw64\bin'
        }

        if (-not $mingwBin -or -not (Test-Path (Join-Path $mingwBin 'x86_64-w64-mingw32-gcc.exe'))) {
            $gcc = Get-Command x86_64-w64-mingw32-gcc -ErrorAction SilentlyContinue
            if ($gcc) {
                $mingwBin = Split-Path -Parent $gcc.Source
            }
        }

        if (-not $mingwBin -or -not (Test-Path (Join-Path $mingwBin 'x86_64-w64-mingw32-gcc.exe'))) {
            throw 'A MinGW gcc toolchain was not found. Install WinLibs or put x86_64-w64-mingw32-gcc.exe on PATH, or rerun with -TargetTriple x86_64-pc-windows-msvc.'
        }

        $env:PATH = "$mingwBin;$env:PATH"
        $toolchain = 'stable-x86_64-pc-windows-gnu'
    }
    'x86_64-pc-windows-msvc' {
        $toolchain = 'stable-x86_64-pc-windows-msvc'
    }
    'aarch64-pc-windows-msvc' {
        $toolchain = 'stable-x86_64-pc-windows-msvc'
    }
    default {
        throw "Unsupported Windows target triple '$TargetTriple'. Supported values: x86_64-pc-windows-gnu, x86_64-pc-windows-msvc, aarch64-pc-windows-msvc."
    }
}

$cargoCommand = $CargoArgs[0]
$cargoRemainder = @()
if ($CargoArgs.Count -gt 1) {
    $cargoRemainder = $CargoArgs[1..($CargoArgs.Count - 1)]
}

$argList = @("+$toolchain", $cargoCommand, '--target', $TargetTriple) + $cargoRemainder

& cargo @argList
exit $LASTEXITCODE
