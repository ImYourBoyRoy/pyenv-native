# ./scripts/build-homebrew-formula.ps1
<#
Purpose: Generates a Homebrew formula from release asset checksum files so Homebrew support can be prepared before public tap publishing.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/build-homebrew-formula.ps1 -GitHubRepo <owner/repo> -Tag <vX.Y.Z>
Inputs: GitHub repo/tag metadata, one or more asset roots containing .tar.gz.sha256 files, and an optional output path.
Outputs/side effects: Writes a formula file under packaging/homebrew/Formula/ with URL/sha blocks for every detected Linux/macOS bundle.
Notes: The formula is generator-backed and can be committed later or submitted to a tap after the release assets are public.
#>

param(
    [string]$GitHubRepo = 'imyourboyroy/pyenv-native',
    [string]$Tag,
    [string[]]$AssetRoots = @(
        (Join-Path $PSScriptRoot '..\dist'),
        (Join-Path $PSScriptRoot '..\dist\linux'),
        (Join-Path $PSScriptRoot '..\dist\macos')
    ),
    [string]$OutputPath = (Join-Path $PSScriptRoot '..\packaging\homebrew\Formula\pyenv-native.rb'),
    [string]$FormulaName = 'pyenv-native',
    [string]$Description = 'Cross-platform, native-first reimplementation of pyenv',
    [string]$Homepage
)

$ErrorActionPreference = 'Stop'

function Get-WorkspaceVersion {
    $cargoTomlPath = Join-Path $PSScriptRoot '..\Cargo.toml'
    $cargoToml = Get-Content -Raw $cargoTomlPath
    $versionMatch = [regex]::Match($cargoToml, '(?m)^\s*version\s*=\s*"([^"]+)"\s*$')
    if (-not $versionMatch.Success) {
        throw "Failed to determine workspace version from $cargoTomlPath"
    }

    return $versionMatch.Groups[1].Value
}

function Convert-ToFormulaClassName {
    param(
        [string]$Name
    )

    return (($Name -split '[^A-Za-z0-9]+') | Where-Object { $_ } | ForEach-Object {
        $_.Substring(0, 1).ToUpperInvariant() + $_.Substring(1)
    }) -join ''
}

function Read-ChecksumValue {
    param(
        [string]$Path
    )

    $line = Get-Content -Path $Path -TotalCount 1
    $lineValue = if ($line) { $line } else { '' }
    $match = [regex]::Match($lineValue.Trim(), '^(?<sha>[A-Fa-f0-9]{64})\b')
    if (-not $match.Success) {
        throw "Checksum file '$Path' did not contain a valid SHA-256 digest."
    }

    return $match.Groups['sha'].Value.ToLowerInvariant()
}

function Get-AssetRecordMap {
    param(
        [string[]]$Roots,
        [string]$Repository,
        [string]$ReleaseTag
    )

    $records = [ordered]@{}
    foreach ($root in $Roots) {
        if (-not (Test-Path $root)) {
            continue
        }

        Get-ChildItem -Path $root -Recurse -File -Filter '*.sha256' | ForEach-Object {
            $checksumPath = $_.FullName
            $assetName = [System.IO.Path]::GetFileNameWithoutExtension($_.Name)
            if ($assetName -notmatch '^pyenv-native-(linux|macos)-(x64|arm64)\.tar\.gz$') {
                return
            }

            if (-not $records.Contains($assetName)) {
                $records[$assetName] = [pscustomobject]@{
                    asset_name = $assetName
                    operating_system = $Matches[1]
                    architecture = $Matches[2]
                    sha256 = Read-ChecksumValue -Path $checksumPath
                    url = "https://github.com/$Repository/releases/download/$ReleaseTag/$assetName"
                }
            }
        }
    }

    return $records
}

function Get-ConditionalBody {
    param(
        [object[]]$Records,
        [string]$Indent = '    ',
        [string]$FormulaLabel
    )

    $orderedRecords = @($Records | Sort-Object architecture)
    $lines = New-Object System.Collections.Generic.List[string]
    for ($index = 0; $index -lt $orderedRecords.Count; $index++) {
        $record = $orderedRecords[$index]
        $condition = switch ($record.architecture) {
            'arm64' { 'Hardware::CPU.arm?' }
            'x64' { 'Hardware::CPU.intel?' }
            default { throw "Unsupported formula architecture '$($record.architecture)'" }
        }

        if ($index -eq 0) {
            $lines.Add("${Indent}if $condition")
        } else {
            $lines.Add("${Indent}elsif $condition")
        }
        $lines.Add("${Indent}  url `"$($record.url)`"")
        $lines.Add("${Indent}  sha256 `"$($record.sha256)`"")
    }
    $lines.Add("${Indent}else")
    $lines.Add("${Indent}  odie `"Unsupported architecture for $FormulaLabel`"")
    $lines.Add("${Indent}end")

    return $lines
}

$workspaceVersion = Get-WorkspaceVersion
if (-not $Tag) {
    $Tag = 'v' + $workspaceVersion
}
if (-not $Homepage) {
    $Homepage = "https://github.com/$GitHubRepo"
}

$assetRecordMap = Get-AssetRecordMap -Roots $AssetRoots -Repository $GitHubRepo -ReleaseTag $Tag
if ($assetRecordMap.Count -eq 0) {
    throw 'No Linux/macOS bundle checksum files were found. Build or download release assets before generating the Homebrew formula.'
}

$macosRecords = @($assetRecordMap.Values | Where-Object { $_.operating_system -eq 'macos' })
$linuxRecords = @($assetRecordMap.Values | Where-Object { $_.operating_system -eq 'linux' })
$formulaClassName = Convert-ToFormulaClassName -Name $FormulaName
$outputDirectory = Split-Path -Parent $OutputPath
New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null

$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("# ./packaging/homebrew/Formula/$FormulaName.rb")
$lines.Add("class $formulaClassName < Formula")
$lines.Add("  desc `"$Description`"")
$lines.Add("  homepage `"$Homepage`"")
$lines.Add('  license "MIT"')
$lines.Add("  version `"$workspaceVersion`"")
$lines.Add('')
if ($macosRecords.Count -gt 0) {
    $lines.Add('  on_macos do')
    foreach ($line in Get-ConditionalBody -Records $macosRecords -Indent '    ' -FormulaLabel $FormulaName) {
        $lines.Add($line)
    }
    $lines.Add('  end')
    $lines.Add('')
}
if ($linuxRecords.Count -gt 0) {
    $lines.Add('  on_linux do')
    foreach ($line in Get-ConditionalBody -Records $linuxRecords -Indent '    ' -FormulaLabel $FormulaName) {
        $lines.Add($line)
    }
    $lines.Add('  end')
    $lines.Add('')
}
$lines.Add('  def install')
$lines.Add('    bin.install "pyenv"')
$lines.Add('    prefix.install "LICENSE", "README.md"')
$lines.Add('    libexec.install "install-pyenv-native.sh", "uninstall-pyenv-native.sh"')
$lines.Add('  end')
$lines.Add('')
$lines.Add('  def caveats')
$lines.Add('    <<~EOS')
$lines.Add('      pyenv-native keeps managed Python runtimes under ~/.pyenv by default.')
$lines.Add('')
$lines.Add('      Add it to your shell manually:')
$lines.Add('        eval "$(&quot;#{bin}/pyenv&quot; init - bash)"')
$lines.Add('')
$lines.Add('      Or run the bundled profile-aware installer:')
$lines.Add('        #{libexec}/install-pyenv-native.sh --source-path #{bin}/pyenv --install-root ~/.pyenv --shell bash')
$lines.Add('    EOS')
$lines.Add('  end')
$lines.Add('')
$lines.Add('  test do')
$lines.Add('    ENV["PYENV_ROOT"] = testpath/".pyenv"')
$lines.Add('    assert_equal ENV["PYENV_ROOT"], shell_output("#{bin}/pyenv root").strip')
$lines.Add('  end')
$lines.Add('end')
$lines.Add('')

($lines -join [Environment]::NewLine).Replace('&quot;', '"') | Set-Content -Path $OutputPath -Encoding utf8

$summary = [ordered]@{
    output_path = [System.IO.Path]::GetFullPath($OutputPath)
    github_repo = $GitHubRepo
    tag = $Tag
    macos_assets = $macosRecords.Count
    linux_assets = $linuxRecords.Count
}

$summary.GetEnumerator() | ForEach-Object {
    '{0}: {1}' -f $_.Key, $_.Value
}


