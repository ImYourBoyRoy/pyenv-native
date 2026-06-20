# Cursor

## Install

```powershell
# Windows
./scripts/install-agent-skills.ps1 -Agent cursor
```

```bash
# macOS / Linux
./scripts/install-agent-skills.sh --agent cursor
```

Skills copy to:

- **User (default):** `~/.cursor/skills/<skill-name>/SKILL.md`
- **Project:** `.cursor/skills/` in the current directory (`-Scope project`)

## Agent prompt

```text
Install the agent skills from https://github.com/imyourboyroy/pyenv-native
```

## MCP (recommended)

Register **pyenv-mcp** in Cursor MCP settings:

```powershell
pyenv-mcp print-config
```

Then say: **Follow the pyenv-native skill** when managing Python versions or venvs.

## Update

Re-run the install script after `git pull`.
