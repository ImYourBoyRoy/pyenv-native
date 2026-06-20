# Antigravity CLI (agy)

## Install

```bash
agy plugin install https://github.com/imyourboyroy/pyenv-native.git
```

Local clone:

```bash
git clone https://github.com/imyourboyroy/pyenv-native.git
agy plugin install /path/to/pyenv-native
```

Or:

```bash
./scripts/install-agent-skills.sh --agent antigravity
```

## Validate

```bash
agy plugin validate /path/to/pyenv-native
agy plugin list
```

## Agent prompt

```text
Install the agent skills from https://github.com/imyourboyroy/pyenv-native as an Antigravity plugin (agy plugin install)
```

## Workspace rules

Copy or symlink `AGENTS.md` into project roots where strict Python env discipline is required.

## MCP

Use `pyenv-mcp` alongside the plugin for structured runtime management.
