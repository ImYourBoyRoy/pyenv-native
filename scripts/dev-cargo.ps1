# ./scripts/dev-cargo.ps1
<#
Purpose: Runs Cargo with the GNU Windows Rust toolchain and a discovered MinGW toolchain.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/dev-cargo.ps1 test
Inputs: cargo arguments after the script name.
Outputs/side effects: Executes cargo with a MinGW-enabled PATH for this process.
Notes: Prefers a Winget-installed WinLibs MinGW toolchain; falls back to PATH if available.
#>

param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$CargoArgs
)

$ErrorActionPreference = "Stop"

if (-not $CargoArgs -or $CargoArgs.Count -eq 0) {
    $CargoArgs = @("test")
}

$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (-not (Test-Path $cargoBin)) {
    throw "Rust cargo bin path was not found at $cargoBin"
}

$winlibsRoot = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Directory -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -like "BrechtSanders.WinLibs.POSIX.UCRT*" } |
    Select-Object -First 1 -ExpandProperty FullName

$mingwBin = if ($winlibsRoot) {
    Join-Path $winlibsRoot "mingw64\bin"
}

if (-not $mingwBin -or -not (Test-Path (Join-Path $mingwBin "x86_64-w64-mingw32-gcc.exe"))) {
    $gcc = Get-Command x86_64-w64-mingw32-gcc -ErrorAction SilentlyContinue
    if ($gcc) {
        $mingwBin = Split-Path -Parent $gcc.Source
    }
}

if (-not $mingwBin -or -not (Test-Path (Join-Path $mingwBin "x86_64-w64-mingw32-gcc.exe"))) {
    throw "A MinGW gcc toolchain was not found. Install WinLibs or put x86_64-w64-mingw32-gcc.exe on PATH."
}

$env:PATH = "$cargoBin;$mingwBin;$env:PATH"

$cargoCommand = $CargoArgs[0]
$cargoRemainder = @()
if ($CargoArgs.Count -gt 1) {
    $cargoRemainder = $CargoArgs[1..($CargoArgs.Count - 1)]
}

$argList = @("+stable-x86_64-pc-windows-gnu", $cargoCommand, "--target", "x86_64-pc-windows-gnu") + $cargoRemainder

& cargo @argList
exit $LASTEXITCODE
