# ./scripts/apply-main-branch-protection.ps1
# Purpose: Apply the repository ruleset that protects main from deletion and force-push.
# How to run: pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File .\scripts\apply-main-branch-protection.ps1
# Inputs: Requires GitHub CLI (gh) authenticated as a repo admin.
# Outputs/side effects: Creates or updates the "Protect main" ruleset via GitHub REST API.
# Notes: Cloud agents cannot set branch protection; run this locally with admin credentials.

$ErrorActionPreference = 'Stop'

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Split-Path -Parent $ScriptDir
$RulesetFile = Join-Path $RepoRoot '.github/rulesets/protect-main.json'
$RulesetName = 'Protect main'

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    throw 'GitHub CLI (gh) is required.'
}

if (-not (Test-Path $RulesetFile)) {
    throw "Ruleset file not found: $RulesetFile"
}

$Repo = gh repo view --json nameWithOwner --jq .nameWithOwner
Write-Host "Applying ruleset to $Repo..."

$ExistingId = gh api "repos/$Repo/rulesets" --jq ".[] | select(.name == `"$RulesetName`") | .id" 2>$null
if ($LASTEXITCODE -ne 0) {
    $ExistingId = $null
}

if ($ExistingId) {
    Write-Host "Updating existing ruleset id=$ExistingId"
    gh api -X PUT "repos/$Repo/rulesets/$ExistingId" --input $RulesetFile
} else {
    Write-Host 'Creating new ruleset'
    gh api -X POST "repos/$Repo/rulesets" --input $RulesetFile
}

if ($LASTEXITCODE -ne 0) {
    throw 'Failed to apply branch protection ruleset. Ensure gh is authenticated as a repository admin.'
}

Write-Host "Done. Verify at: https://github.com/$Repo/settings/rules"
