# Claude Code

## Marketplace install (recommended)

In Claude Code:

```text
/plugin marketplace add https://github.com/imyourboyroy/pyenv-native.git
/plugin install pyenv-native@pyenv-native
```

If SSH clone fails, use the HTTPS marketplace URL above.

## Local / development

```bash
git clone https://github.com/imyourboyroy/pyenv-native.git
claude --plugin-dir /path/to/pyenv-native
```

## Agent prompt

```text
Install the agent skills from https://github.com/imyourboyroy/pyenv-native using the Claude Code plugin marketplace or --plugin-dir
```

## Skills location

Plugin metadata: `.claude-plugin/plugin.json`  
Skills: `skills/pyenv-native/SKILL.md`

Also read repo `AGENTS.md` when editing the Rust codebase.

## MCP

Add `pyenv-mcp` from `pyenv-mcp print-config` to Claude MCP config for structured Python environment tools.
