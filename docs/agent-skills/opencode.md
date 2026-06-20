# OpenCode

OpenCode uses **agent-driven** skill execution via `AGENTS.md` and the built-in `skill` tool.

## Install

1. Clone or open the repo as your workspace:

```bash
git clone https://github.com/imyourboyroy/pyenv-native.git
```

2. Ensure present:

- `AGENTS.md` (root)
- `skills/pyenv-native/SKILL.md`

No separate install step — the agent discovers skills from the workspace.

## Agent prompt

```text
Use the pyenv-native workspace skills from https://github.com/imyourboyroy/pyenv-native — read AGENTS.md and invoke the pyenv-native skill when managing Python environments
```

## Expected behavior

- Python version / venv tasks → load `pyenv-native` skill
- Prefer MCP tool order from skill before raw shell

## MCP

Register `pyenv-mcp` in OpenCode MCP config when available.
