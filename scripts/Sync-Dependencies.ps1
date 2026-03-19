# ./scripts/Sync-Dependencies.ps1
# Maintenance script to find and optionally update Cargo.toml dependencies 
# based on the latest resolved (or registry) versions.

Param(
    [Parameter(Mandatory=$false)]
    [switch]$Fix,

    [Parameter(Mandatory=$false)]
    [switch]$DryRun,

    [Parameter(Mandatory=$false)]
    [switch]$CheckLatest
)

$ErrorActionPreference = 'Stop'

Write-Host "--- pyenv-native Dependency Sync ---" -ForegroundColor Cyan

# 1. Collect Target Versions
$TargetVersionsCount = 0
$TargetVersions = @{}

# Metadata keys to skip in discovery to avoid false positives (like package.version)
$MetadataKeysToSkip = @(
    "name", "version", "edition", "license", "resolver", "authors", 
    "description", "homepage", "repository", "documentation", 
    "keywords", "categories", "readme", "publish", "default-run"
)

if ($CheckLatest) {
    Write-Host "Fetching latest registry versions from crates.io (this may take a moment)..." -ForegroundColor Gray
    
    # Discovery Pass: Find all unique dependencies across the workspace
    $WorkspaceRoot = Resolve-Path "$PSScriptRoot\.."
    $AllTomlFiles = Get-ChildItem -Path $WorkspaceRoot -Filter "Cargo.toml" -Recurse
    $UniqueDependencyNames = @{}
    
    foreach ($TomlFile in $AllTomlFiles) {
        $RawTomlContent = Get-Content $TomlFile.FullName -Raw
        $DependencyIdentRegex = '(?m)^(\s*)([a-zA-Z0-9_-]+)\s*=\s*(?:\"([0-9.]+)\"|\{\s*version\s*=\s*\"([0-9.]+)\")'
        $DiscoveryMatchesResult = [regex]::Matches($RawTomlContent, $DependencyIdentRegex)
        foreach ($DiscMatchObj in $DiscoveryMatchesResult) { 
            $FoundNameVarStr = $DiscMatchObj.Groups[2].Value
            if ($FoundNameVarStr -notin $MetadataKeysToSkip -and $FoundNameVarStr -ne "pyenv-core") { 
                $UniqueDependencyNames[$FoundNameVarStr] = $true 
            }
        }
    }

    # Registry Query Pass
    $CurrentDepTrackerIndex = 0
    foreach ($NameReqItem in $UniqueDependencyNames.Keys) {
        $CurrentDepTrackerIndex++
        Write-Progress -Activity "Fetching registry versions" -Status "Checking $NameReqItem" -PercentComplete (($CurrentDepTrackerIndex / $UniqueDependencyNames.Count) * 100)
        
        # Suppress noise by ignoring note: lines and stderr
        $RawSearchOutputLines = cargo search $NameReqItem --limit 1 2>$null
        foreach ($LineOutput in $RawSearchOutputLines) {
            $EscapedSearchPkgNameStr = [regex]::Escape($NameReqItem)
            if ($LineOutput -match "^$EscapedSearchPkgNameStr = `"([^`"]+)`"") {
                $TargetVersions[$NameReqItem] = $matches[1].Split('+')[0]
                $TargetVersionsCount++
                break
            }
        }
    }
    Write-Progress -Activity "Fetching registry versions" -Completed
} else {
    Write-Host "Fetching local resolved versions from cargo metadata..." -ForegroundColor Gray
    $RawMetadataJsonObj = cargo metadata --format-version 1 | ConvertFrom-Json
    foreach ($PackageInfoItem in $RawMetadataJsonObj.packages) {
        if ($null -ne $PackageInfoItem.source) {
            # Strip metadata from version (e.g. 0.9.12+spec-1.1.0 -> 0.9.12)
            $TargetVersions[$PackageInfoItem.name] = $PackageInfoItem.version.Split('+')[0]
            $TargetVersionsCount++
        }
    }
}

# 2. Update Pass
$WorkspaceRoot = Resolve-Path "$PSScriptRoot\.."
$TomlFilesToProcessLoop = Get-ChildItem -Path $WorkspaceRoot -Filter "Cargo.toml" -Recurse
$TotalUpdatesFoundAcrossAllFiles = 0
$TotalFilesUpdated = 0

foreach ($TargetFileProcessItem in $TomlFilesToProcessLoop) {
    $DisplayPathString = $TargetFileProcessItem.FullName.Replace($WorkspaceRoot.Path, ".")
    Write-Host "`nChecking: $DisplayPathString" -ForegroundColor Yellow
    
    $OriginalFileContentRawStr = Get-Content $TargetFileProcessItem.FullName -Raw
    $UpdatedContentResultStr = $OriginalFileContentRawStr
    $FileLevelUpdateCountTracker = 0

    $FetchPatternsListArray = @(
        '(?m)^(\s*)([a-zA-Z0-9_-]+)\s*=\s*\"([0-9.]+)\"',
        '(?m)^(\s*)([a-zA-Z0-9_-]+)\s*=\s*\{\s*version\s*=\s*\"([0-9.]+)\"'
    )

    foreach ($PatternStrItemValue in $FetchPatternsListArray) {
        $UpdateMatchesFoundInFileResult = [regex]::Matches($OriginalFileContentRawStr, $PatternStrItemValue)
        foreach ($UpdMatchObject in $UpdateMatchesFoundInFileResult) {
            $KeyNameFoundStr = $UpdMatchObject.Groups[2].Value
            $DeclVerFoundStr = $UpdMatchObject.Groups[3].Value

            if ($KeyNameFoundStr -notin $MetadataKeysToSkip -and $TargetVersions.ContainsKey($KeyNameFoundStr)) {
                $NewerVerFoundInMapStr = $TargetVersions[$KeyNameFoundStr]
                
                if ($DeclVerFoundStr -ne $NewerVerFoundInMapStr -and $DeclVerFoundStr -notlike "*workspace*") {
                    Write-Host "  [UPDATE] $KeyNameFoundStr : $DeclVerFoundStr -> $NewerVerFoundInMapStr" -ForegroundColor Green
                    $TotalUpdatesFoundAcrossAllFiles++
                    $FileLevelUpdateCountTracker++

                    if ($Fix) {
                        $MatchLineTextStr = $UpdMatchObject.Value
                        $NewLineTextResolvedStr = $MatchLineTextStr.Replace($DeclVerFoundStr, $NewerVerFoundInMapStr)
                        $UpdatedContentResultStr = $UpdatedContentResultStr.Replace($MatchLineTextStr, $NewLineTextResolvedStr)
                    }
                }
            }
        }
    }

    if ($FileLevelUpdateCountTracker -eq 0) {
        Write-Host "  No updates required." -ForegroundColor Gray
    } elseif ($Fix) {
        if ($DryRun) {
            Write-Host "  [DRY RUN] Would update $DisplayPathString" -ForegroundColor Magenta
        } else {
            Write-Host "  Updating $DisplayPathString..." -ForegroundColor Cyan
            $UpdatedContentResultStr | Set-Content $TargetFileProcessItem.FullName
            $TotalFilesUpdated++
        }
    }
}

Write-Host "`n--- Summary ---" -ForegroundColor Cyan
if ($TotalUpdatesFoundAcrossAllFiles -eq 0) {
    Write-Host "All dependencies are current." -ForegroundColor Green
} else {
    Write-Host "Found $TotalUpdatesFoundAcrossAllFiles pending updates." -ForegroundColor Yellow
    if ($Fix -and -not $DryRun) {
        Write-Host "Refreshing Cargo.lock..." -ForegroundColor Gray
        cargo update
        Write-Host "Applied $TotalUpdatesFoundAcrossAllFiles updates across $TotalFilesUpdated file(s)." -ForegroundColor Green
        Write-Host "All changes saved and Cargo.lock refreshed." -ForegroundColor Green
    } elseif (-not $Fix) {
        Write-Host "Run with -Fix to apply these changes." -ForegroundColor Gray
    }
}
Write-Host "--- Done ---" -ForegroundColor Cyan
