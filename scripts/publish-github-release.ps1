# ./scripts/publish-github-release.ps1
<#
Purpose: Orchestrates the local preflight, tagging, and optional push steps that trigger the workflow-driven GitHub release flow for pyenv-native.
How to run: powershell -ExecutionPolicy Bypass -File ./scripts/publish-github-release.ps1 -Version <semver> [-PushCurrentBranch] [-PushTag] [-WatchWorkflow]
Inputs: Release version, optional repo/remote info, Python path for bootstrap validation, and switches controlling version sync, local validation/build steps, and git pushes.
Outputs/side effects: Optionally syncs versions, runs validation/build scripts, creates a git tag, pushes the branch/tag, and can watch the GitHub Actions release workflow.
Notes: The actual public GitHub Release is produced by .github/workflows/release.yml after the pushed tag lands on GitHub.
#>

param(
    [Parameter(Mandatory = $true)]
    [string]$Version,
    [string]$Remote = 'origin',
    [string]$PythonPath = 'python',
    [string]$GitHubRepo,
    [switch]$SyncVersion,
    [switch]$SkipValidation,
    [switch]$SkipBuild,
    [switch]$PushCurrentBranch,
    [switch]$PushTag,
    [switch]$WatchWorkflow,
    [switch]$AllowDirty,
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'

function Write-Step {
    param([string]$Message)
    Write-Host "[publish-github-release] $Message"
}

function Format-CommandArgument {
    param([string]$Value)

    if ($null -eq $Value) {
        return "''"
    }

    if ($Value -match '[\s`"\$]') {
        return "'" + ($Value -replace "'", "''") + "'"
    }

    return $Value
}

function Assert-CommandAvailable {
    param([string]$CommandName)
    if (-not (Get-Command $CommandName -ErrorAction SilentlyContinue)) {
        throw "Required command '$CommandName' was not found on PATH."
    }
}

function Assert-GitRepository {
    $repoMarker = Join-Path (Resolve-Path (Join-Path $PSScriptRoot '..')) '.git'
    if (-not (Test-Path $repoMarker)) {
        throw 'This directory is not inside a git repository yet. Initialize the repo or clone the final GitHub remote before using publish-github-release.ps1.'
    }
}

function Invoke-OrReport {
    param(
        [string[]]$Command,
        [switch]$AllowedInDryRun
    )

    $rendered = ($Command | ForEach-Object { Format-CommandArgument -Value $_ }) -join ' '

    if ($DryRun -and -not $AllowedInDryRun) {
        Write-Host "DRY-RUN: $rendered"
        return
    }

    & $Command[0] @($Command | Select-Object -Skip 1)
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed with exit code ${LASTEXITCODE}: $rendered"
    }
}

function Get-GitHubRepoFromRemote {
    param([string]$RemoteName)

    $remoteUrl = (& git remote get-url $RemoteName 2>$null)
    if (-not $remoteUrl) {
        return $null
    }

    if ($remoteUrl -match 'github\.com[:/](?<repo>[^/]+/[^/.]+)(?:\.git)?$') {
        return $Matches.repo
    }

    return $null
}

function Assert-CleanGitState {
    $status = & git status --porcelain 2>$null
    if ($LASTEXITCODE -ne 0) {
        throw 'Failed to inspect git status.'
    }
    if ($status) {
        throw 'Git working tree is not clean. Commit or stash local changes before publishing.'
    }
}

function Resolve-ReleaseTag {
    param([string]$SemanticVersion)
    if ($SemanticVersion.StartsWith('v')) {
        return $SemanticVersion
    }
    return 'v' + $SemanticVersion
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$tag = Resolve-ReleaseTag -SemanticVersion $Version

Assert-CommandAvailable -CommandName git
Assert-GitRepository
if (-not $GitHubRepo) {
    $GitHubRepo = Get-GitHubRepoFromRemote -RemoteName $Remote
}
if (-not $AllowDirty) {
    Assert-CleanGitState
}

if ($SyncVersion) {
    Write-Step "Syncing workspace/package version to $Version"
    Invoke-OrReport -Command @('powershell', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $PSScriptRoot 'set-version.ps1'), '-Version', $Version)
}

if (-not $SkipValidation) {
    Write-Step 'Running release validation checks'
    Invoke-OrReport -Command @('powershell', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $PSScriptRoot 'dev-cargo.ps1'), 'test')
    Invoke-OrReport -Command @('powershell', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $PSScriptRoot 'test-python-bootstrap.ps1'), '-PythonPath', $PythonPath)
}

if (-not $SkipBuild) {
    Write-Step 'Building local release artifacts for a final pre-publish sanity check'
    Invoke-OrReport -Command @('powershell', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $PSScriptRoot 'build-release-bundle.ps1'), '-OutputRoot', (Join-Path $repoRoot 'dist'))
    Invoke-OrReport -Command @('powershell', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $PSScriptRoot 'build-python-bootstrap.ps1'), '-PythonPath', $PythonPath)
    if ($GitHubRepo) {
        Invoke-OrReport -Command @('powershell', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $PSScriptRoot 'build-winget-manifests.ps1'), '-GitHubRepo', $GitHubRepo, '-Tag', $tag, '-OutputRoot', (Join-Path $repoRoot 'packaging\winget'), '-Validate')
    }
}

$existingTag = (& git tag --list $tag 2>$null)
if (-not $existingTag) {
    Write-Step "Creating git tag $tag"
    Invoke-OrReport -Command @('git', 'tag', '-a', $tag, '-m', "Release $tag")
} else {
    Write-Step "Git tag $tag already exists; leaving it in place."
}

if ($PushCurrentBranch) {
    $branch = (& git branch --show-current 2>$null).Trim()
    if (-not $branch) {
        throw 'Unable to determine the current git branch for push.'
    }
    Write-Step "Pushing branch $branch to $Remote"
    Invoke-OrReport -Command @('git', 'push', $Remote, $branch)
}

if ($PushTag) {
    Write-Step "Pushing tag $tag to $Remote"
    Invoke-OrReport -Command @('git', 'push', $Remote, $tag)
}

if ($WatchWorkflow) {
    Assert-CommandAvailable -CommandName gh
    Write-Step 'Watching the latest GitHub Actions run for this repository'
    if ($GitHubRepo) {
        Invoke-OrReport -Command @('gh', 'run', 'watch', '--repo', $GitHubRepo)
    } else {
        Invoke-OrReport -Command @('gh', 'run', 'watch')
    }
}

$summary = [ordered]@{
    version = $Version
    tag = $tag
    github_repo = $GitHubRepo
    sync_version = [bool]$SyncVersion
    skip_validation = [bool]$SkipValidation
    skip_build = [bool]$SkipBuild
    push_current_branch = [bool]$PushCurrentBranch
    push_tag = [bool]$PushTag
    watch_workflow = [bool]$WatchWorkflow
    allow_dirty = [bool]$AllowDirty
    dry_run = [bool]$DryRun
}

$summary.GetEnumerator() | ForEach-Object {
    '{0}: {1}' -f $_.Key, $_.Value
}
