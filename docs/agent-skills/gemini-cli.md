# Gemini CLI

## Install (recommended)

```bash
gemini skills install https://github.com/imyourboyroy/pyenv-native.git --path skills
```

From a local clone:

```bash
git clone https://github.com/imyourboyroy/pyenv-native.git
gemini skills install /path/to/pyenv-native/skills/
```

Workspace-only (project `.gemini/skills/`):

```bash
gemini skills install /path/to/pyenv-native/skills/ --scope workspace
```

Or use the installer:

```bash
./scripts/install-agent-skills.sh --agent gemini
```

## Verify

```
/skills list
```

## Agent prompt

```text
Install the agent skills from https://github.com/imyourboyroy/pyenv-native using gemini skills install
```

## Persistent context (optional)

For always-on rules, add `@skills/pyenv-native/SKILL.md` to project `GEMINI.md`. Prefer on-demand skills for most workflows.

## MCP

Configure `pyenv-mcp` in `~/.gemini/config.json` when managing Python environments from Gemini.
