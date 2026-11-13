# Agent skills — pyenv-native

Install structured agent workflows for **pyenv-native** across mainstream coding agents (Cursor, Claude Code, Gemini CLI, Antigravity, GitHub Copilot, Windsurf, OpenCode, Kiro).

**Start here:** [getting-started.md](./getting-started.md) (all platforms + one-shot install)
## Tell your agent (copy-paste)

Use this prompt in **any** supported agent:

```text
Install the agent skills from https://github.com/imyourboyroy/pyenv-native
```

The agent should run `pyenv agent-skills --agent all` (install pyenv-native first if `pyenv` is missing).

## Quick install

The skill is built into `pyenv`, so there is nothing to clone and no Rust toolchain is needed. It works the same on Windows, macOS, and Linux:

```bash
pyenv agent-skills --agent all
```

Don't have `pyenv` yet? Install pyenv-native first (see the [README](../../README.md)); the skill is for using it.

### Project-scoped (single repo)

Install skills into the **current project** (`.cursor/skills`, `.github/skills`, etc.). Run this from the project root:

```bash
pyenv agent-skills --agent cursor --scope project
```

### Agents that install from the repository

Gemini CLI, Antigravity, and Claude Code install from the public repository (`https://github.com/imyourboyroy/pyenv-native`), not from files on disk. `pyenv agent-skills` runs `gemini` or `agy` for you when they are on `PATH`, and prints the exact commands when they are not. Pass `--repo-url` to use a fork.

Run `pyenv agent-skills --help` for every option.

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

Update pyenv-native (`pyenv self-update`), then run the same command again. It replaces the installed copy of the skill.

```bash
pyenv self-update
pyenv agent-skills --agent all
```

## Repo layout

```text
skills/pyenv-native/SKILL.md   # skill entry point
AGENTS.md                        # repo agent rules
docs/agent-skills/               # install guides (this folder)
crates/pyenv-core/src/agent_skills.rs   # installer (`pyenv agent-skills`); embeds the skill
plugin.json                      # Antigravity / plugin manifest
.claude-plugin/plugin.json       # Claude Code plugin metadata
```
