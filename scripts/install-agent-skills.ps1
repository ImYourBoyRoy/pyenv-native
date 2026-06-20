# ./scripts/install-agent-skills.ps1
<#
.SYNOPSIS
  Install this repo's agent skills for Cursor, Claude Code, Gemini CLI, Antigravity, Copilot, and more.

.PARAMETER Agent
  Target agent: cursor, claude, gemini, antigravity, copilot, kiro, windsurf, opencode, all (default: all)

.PARAMETER Scope
  user (global) or project (current directory). Default: user

.PARAMETER RepoUrl
  Optional GitHub URL. Clones to a cache dir before installing (for agents that only have a URL).

.PARAMETER RepoRoot
  Path to repo root. Defaults to parent of scripts/.

.EXAMPLE
  ./scripts/install-agent-skills.ps1 -Agent all
  ./scripts/install-agent-skills.ps1 -Agent cursor -Scope project
  ./scripts/install-agent-skills.ps1 -RepoUrl https://github.com/imyourboyroy/pyenv-native
#>
[CmdletBinding(DefaultParameterSetName = 'Local')]
param(
    [ValidateSet('cursor', 'claude', 'gemini', 'antigravity', 'copilot', 'kiro', 'windsurf', 'opencode', 'all')]
    [string]$Agent = 'all',

    [ValidateSet('user', 'project')]
    [string]$Scope = 'user',

    [Parameter(ParameterSetName = 'Remote')]
    [string]$RepoUrl,

    [string]$RepoRoot = '',
    [string]$Branch = 'main'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-RepoSlug([string]$Url) {
    $clean = ($Url -replace '\.git$', '').TrimEnd('/')
    return ($clean -split '/')[-1]
}

function Copy-SkillTree {
    param([string]$SourceSkillsDir, [string]$DestDir)
    if (-not (Test-Path $DestDir)) {
        New-Item -ItemType Directory -Path $DestDir -Force | Out-Null
    }
    Get-ChildItem $SourceSkillsDir -Directory | ForEach-Object {
        $target = Join-Path $DestDir $_.Name
        if (Test-Path $target) { Remove-Item $target -Recurse -Force }
        Copy-Item $_.FullName $target -Recurse -Force
        Write-Host "    + $($_.Name)"
    }
}

function Install-CursorSkills {
    param([string]$SkillsSource, [string]$Scope)
    $dest = if ($Scope -eq 'project') {
        Join-Path (Get-Location) '.cursor\skills'
    } else {
        Join-Path $env:USERPROFILE '.cursor\skills'
    }
    Write-Host "  Cursor -> $dest"
    Copy-SkillTree -SourceSkillsDir $SkillsSource -DestDir $dest
}

function Install-CopilotSkills {
    param([string]$SkillsSource, [string]$Scope)
    if ($Scope -ne 'project') {
        Write-Host "  Copilot: project scope only (.github/skills). Re-run with -Scope project from a repo root."
        return
    }
    $dest = Join-Path (Get-Location) '.github\skills'
    Write-Host "  Copilot -> $dest"
    Copy-SkillTree -SourceSkillsDir $SkillsSource -DestDir $dest
}

function Install-KiroSkills {
    param([string]$SkillsSource, [string]$Scope)
    $dest = if ($Scope -eq 'project') {
        Join-Path (Get-Location) '.kiro\skills'
    } else {
        Join-Path $env:USERPROFILE '.kiro\skills'
    }
    Write-Host "  Kiro -> $dest"
    Copy-SkillTree -SourceSkillsDir $SkillsSource -DestDir $dest
}

function Install-AgentsFolder {
    param([string]$RepoRoot, [string]$DestSkillsDir)
    $agents = Join-Path $RepoRoot 'agents'
    if (-not (Test-Path $agents)) { return }
    Get-ChildItem $agents -Filter '*.md' | ForEach-Object {
        $name = $_.BaseName
        $target = Join-Path $DestSkillsDir $name
        New-Item -ItemType Directory -Path $target -Force | Out-Null
        Copy-Item $_.FullName (Join-Path $target 'SKILL.md') -Force
        Write-Host "    + $name (agent persona)"
    }
}

function Install-GeminiSkills {
    param([string]$RepoRoot, [string]$Scope)
    if (-not (Get-Command gemini -ErrorAction SilentlyContinue)) {
        Write-Host "  Gemini CLI: 'gemini' not in PATH — manual:"
        Write-Host "    gemini skills install $RepoRoot/skills/ $(if ($Scope -eq 'project') { '--scope workspace' })"
        return
    }
    $scopeArg = if ($Scope -eq 'project') { '--scope', 'workspace' } else { @() }
    & gemini skills install (Join-Path $RepoRoot 'skills') @scopeArg
}

function Install-AntigravityPlugin {
    param([string]$RepoRoot)
    if (-not (Get-Command agy -ErrorAction SilentlyContinue)) {
        Write-Host "  Antigravity: 'agy' not in PATH — manual:"
        Write-Host "    agy plugin install $RepoRoot"
        return
    }
    & agy plugin install $RepoRoot
}

function Show-ClaudeInstructions {
    param([string]$RepoUrl, [string]$RepoRoot)
    Write-Host "  Claude Code:"
    if ($RepoUrl) {
        Write-Host "    /plugin marketplace add $RepoUrl"
        Write-Host "    /plugin install $(Split-Path $RepoRoot -Leaf)@$(Split-Path $RepoRoot -Leaf)"
    }
    Write-Host "    Or: claude --plugin-dir `"$RepoRoot`""
    Write-Host "    Docs: docs/agent-skills/claude-code.md"
}

function Show-WindsurfInstructions {
    Write-Host "  Windsurf: copy skill content to .windsurfrules or Global Rules"
    Write-Host "    Docs: docs/agent-skills/windsurf.md"
}

function Show-OpenCodeInstructions {
    param([string]$RepoRoot)
    Write-Host "  OpenCode: open workspace with AGENTS.md + skills/ at $RepoRoot"
    Write-Host "    Docs: docs/agent-skills/opencode.md"
}

# Resolve repo root
if ($RepoUrl) {
    $slug = Get-RepoSlug $RepoUrl
    $cache = Join-Path $env:USERPROFILE ".agent-skills-cache\$slug"
    if (Test-Path $cache) {
        git -C $cache pull --ff-only
    } else {
        New-Item -ItemType Directory -Path (Split-Path $cache) -Force | Out-Null
        git clone --depth 1 --branch $Branch $RepoUrl $cache
    }
    $RepoRoot = $cache
} elseif (-not $RepoRoot) {
    $RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
} else {
    $RepoRoot = (Resolve-Path $RepoRoot).Path
}

$skillsSource = Join-Path $RepoRoot 'skills'
if (-not (Test-Path $skillsSource)) {
    throw "No skills/ directory at $skillsSource"
}

$repoUrlDisplay = if ($RepoUrl) { $RepoUrl } else { "(local) $RepoRoot" }
Write-Host "Installing agent skills from $repoUrlDisplay"
Write-Host "Agent: $Agent | Scope: $Scope"
Write-Host ""

$targets = if ($Agent -eq 'all') {
    @('cursor', 'copilot', 'kiro', 'gemini', 'antigravity', 'claude', 'windsurf', 'opencode')
} else {
    @($Agent)
}

foreach ($t in $targets) {
    Write-Host "[$t]"
    switch ($t) {
        'cursor' {
            Install-CursorSkills -SkillsSource $skillsSource -Scope $Scope
            if ($Scope -eq 'user') {
                Install-AgentsFolder -RepoRoot $RepoRoot -DestSkillsDir (Join-Path $env:USERPROFILE '.cursor\skills')
            }
        }
        'copilot' { Install-CopilotSkills -SkillsSource $skillsSource -Scope $Scope }
        'kiro' { Install-KiroSkills -SkillsSource $skillsSource -Scope $Scope }
        'gemini' { Install-GeminiSkills -RepoRoot $RepoRoot -Scope $Scope }
        'antigravity' { Install-AntigravityPlugin -RepoRoot $RepoRoot }
        'claude' { Show-ClaudeInstructions -RepoUrl $RepoUrl -RepoRoot $RepoRoot }
        'windsurf' { Show-WindsurfInstructions }
        'opencode' { Show-OpenCodeInstructions -RepoRoot $RepoRoot }
    }
    Write-Host ""
}

Write-Host "Done. See docs/agent-skills/README.md for per-agent details."
