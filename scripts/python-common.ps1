# ./scripts/python-common.ps1
<#
Purpose: Shared Python interpreter discovery helpers for local validation and packaging scripts.
How to run: Dot-source from another PowerShell script: `. "$PSScriptRoot/python-common.ps1"`.
Inputs: Optional explicit interpreter path or command name.
Outputs/side effects: Returns a concrete Python executable path, skipping broken Windows Store aliases when possible.
Notes: Prefers explicit interpreters, then a working `python`, then `py -3`, then `pyenv which python`.
#>

$ErrorActionPreference = 'Stop'

function Get-PythonExecutableFromResolvedCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CommandPath,

        [string[]]$PrefixArguments = @()
    )

    if ($CommandPath -match '[\\/]WindowsApps[\\/]') {
        return $null
    }

    try {
        $output = & $CommandPath @PrefixArguments -c "import os, sys; print(os.path.realpath(sys.executable))" 2>$null
        if ($LASTEXITCODE -ne 0) {
            return $null
        }

        $reportedPath = ($output | Select-Object -Last 1).Trim()
        if ($reportedPath -and (Test-Path $reportedPath)) {
            return (Resolve-Path $reportedPath).ProviderPath
        }

        if (Test-Path $CommandPath) {
            return (Resolve-Path $CommandPath).ProviderPath
        }
    }
    catch {
        return $null
    }

    return $null
}

function Resolve-PythonCommandPath {
    param(
        [string]$ExplicitPath
    )

    if ($ExplicitPath) {
        $resolved = Resolve-Path $ExplicitPath -ErrorAction SilentlyContinue
        if ($resolved) {
            $candidate = Get-PythonExecutableFromResolvedCommand -CommandPath $resolved.ProviderPath
            if ($candidate) {
                return $candidate
            }

            throw "Python interpreter at $ExplicitPath is not executable."
        }

        $command = Get-Command $ExplicitPath -ErrorAction SilentlyContinue
        if ($command) {
            $prefixArgs = @()
            if ($command.Name -eq 'py' -or $command.Source -like '*\py.exe') {
                $prefixArgs = @('-3')
            }

            $candidate = Get-PythonExecutableFromResolvedCommand -CommandPath $command.Source -PrefixArguments $prefixArgs
            if ($candidate) {
                return $candidate
            }

            throw "Python command `$ExplicitPath` was found at $($command.Source) but is not a working interpreter."
        }

        throw "Python interpreter was not found at $ExplicitPath"
    }

    $python = Get-Command python -ErrorAction SilentlyContinue
    if ($python) {
        $candidate = Get-PythonExecutableFromResolvedCommand -CommandPath $python.Source
        if ($candidate) {
            return $candidate
        }
    }

    $pyLauncher = Get-Command py -ErrorAction SilentlyContinue
    if ($pyLauncher) {
        $candidate = Get-PythonExecutableFromResolvedCommand -CommandPath $pyLauncher.Source -PrefixArguments @('-3')
        if ($candidate) {
            return $candidate
        }
    }

    $pyenv = Get-Command pyenv -ErrorAction SilentlyContinue
    if ($pyenv) {
        try {
            $reportedPath = (& $pyenv.Source which python 2>$null | Select-Object -Last 1).Trim()
            if ($LASTEXITCODE -eq 0 -and $reportedPath -and (Test-Path $reportedPath)) {
                $candidate = Get-PythonExecutableFromResolvedCommand -CommandPath $reportedPath
                if ($candidate) {
                    return $candidate
                }
            }
        }
        catch {
            # Ignore pyenv lookup failures and continue to the final error.
        }
    }

    throw 'No usable Python interpreter was found. Install Python, disable the broken Windows Store alias, or pass -PythonPath <python.exe>.'
}
