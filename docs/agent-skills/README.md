# Agent skills — pyenv-native

Install structured agent workflows for **pyenv-native** across mainstream coding agents (Cursor, Claude Code, Gemini CLI, Antigravity, GitHub Copilot, Windsurf, OpenCode, Kiro).

## Tell your agent (copy-paste)

Use this prompt in **any** supported agent:

```text
Install the agent skills from https://github.com/imyourboyroy/pyenv-native
```

The agent should clone the repo and run the cross-platform installer for your platform.

## Quick install

### Windows (PowerShell 7+)

```powershell
git clone --depth 1 https://github.com/imyourboyroy/pyenv-native.git
cd pyenv-native
./scripts/install-agent-skills.ps1 -Agent all
```

### macOS / Linux

```bash
git clone --depth 1 https://github.com/imyourboyroy/pyenv-native.git
cd pyenv-native
chmod +x ./scripts/install-agent-skills.sh
./scripts/install-agent-skills.sh --agent all
```

### From GitHub URL only (no local clone)

**Windows:**

```powershell
./scripts/install-agent-skills.ps1 -RepoUrl "https://github.com/imyourboyroy/pyenv-native" -Agent all
```

**macOS / Linux:**

```bash
./scripts/install-agent-skills.sh --repo-url "https://github.com/imyourboyroy/pyenv-native" --agent all
```

### Project-scoped (single repo)

Install skills into the **current project** (`.cursor/skills`, `.github/skills`, etc.):

```powershell
./scripts/install-agent-skills.ps1 -Agent cursor -Scope project
```

```bash
./scripts/install-agent-skills.sh --agent cursor --scope project
```

## Skills included

| Skill | Purpose |
|-------|---------|
| `pyenv-native` | MCP-first Python/venv workflows, CLI fallback, Windows shim guidance |

Read `AGENTS.md` for full repo rules. Pair with **`pyenv-mcp`** MCP server (`pyenv-mcp print-config`).

## Per-agent guides

| Agent | Guide |
|-------|-------|
| Cursor | [cursor.md](./cursor.md) |
| Claude Code | [claude-code.md](./claude-code.md) |
| Gemini CLI | [gemini-cli.md](./gemini-cli.md) |
| Antigravity CLI | [antigravity.md](./antigravity.md) |
| GitHub Copilot | [copilot.md](./copilot.md) |
| Windsurf | [windsurf.md](./windsurf.md) |
| OpenCode | [opencode.md](./opencode.md) |
| Kiro | [kiro.md](./kiro.md) |

## Update

```powershell
git pull
./scripts/install-agent-skills.ps1 -Agent all
```

```bash
git pull
./scripts/install-agent-skills.sh --agent all
```

## Repo layout

```text
skills/pyenv-native/SKILL.md   # skill entry point
AGENTS.md                        # repo agent rules
docs/agent-skills/               # install guides (this folder)
scripts/install-agent-skills.*   # cross-platform installer
plugin.json                      # Antigravity / plugin manifest
.claude-plugin/plugin.json       # Claude Code plugin metadata
```
