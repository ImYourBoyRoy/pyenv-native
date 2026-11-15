# OpenCode

OpenCode uses **agent-driven** skill execution via `AGENTS.md` and the built-in `skill` tool.

## Install

1. Clone or open the repo as your workspace:

```bash
git clone https://github.com/imyourboyroy/pyenv-native.git
```

2. Ensure present:

- `AGENTS.md` (root)
- `skills/enchi/SKILL.md`

No separate install step — the agent discovers skills from the workspace.

## Agent prompt

```text
Use the Enchi workspace skills from https://github.com/imyourboyroy/pyenv-native — read AGENTS.md and invoke the Enchi skill when managing Python environments
```

## Expected behavior

- Python version / venv tasks → load the `enchi` skill
- Prefer MCP tool order from skill before raw shell

## MCP

Register `enchi-mcp` in OpenCode MCP config when available.
