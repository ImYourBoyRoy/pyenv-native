# Agent skills — Enchi

Install structured agent workflows for **Enchi** across mainstream coding agents (Cursor, Claude Code, Gemini CLI, Antigravity, GitHub Copilot, Windsurf, OpenCode, Kiro).

**Start here:** [getting-started.md](./getting-started.md) (all platforms + one-shot install)
## Tell your agent (copy-paste)

Use this prompt in **any** supported agent:

```text
Install the agent skills from https://github.com/imyourboyroy/pyenv-native
```

The agent should run `enchi agent-skills --agent all` (install Enchi first if `enchi` is missing).

## Quick install

The skill is built into `pyenv`, so there is nothing to clone and no Rust toolchain is needed. It works the same on Windows, macOS, and Linux:

```bash
enchi agent-skills --agent all
```

Don't have `enchi` yet? Install Enchi first (see the [README](../../README.md)); the skill is for using it.

### Project-scoped (single repo)

Install skills into the **current project** (`.cursor/skills`, `.github/skills`, etc.). Run this from the project root:

```bash
enchi agent-skills --agent cursor --scope project
```

### Agents that install from the repository

Gemini CLI, Antigravity, and Claude Code install from the public repository (`https://github.com/imyourboyroy/pyenv-native`), not from files on disk. `enchi agent-skills` runs `gemini` or `agy` for you when they are on `PATH`, and prints the exact commands when they are not. Pass `--repo-url` to use a fork.

Run `enchi agent-skills --help` for every option.

## Skills included

| Skill | Purpose |
|-------|---------|
| Enchi | MCP-first Python/venv workflows, CLI fallback, Windows shim guidance |

Read `AGENTS.md` for full repo rules. Pair with **`enchi-mcp`** MCP server (`enchi-mcp print-config`).

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

Update Enchi (`enchi self-update`), then run the same command again. It replaces the installed copy of the skill.

```bash
enchi self-update
enchi agent-skills --agent all
```

## Repo layout

```text
skills/enchi/SKILL.md   # skill entry point
AGENTS.md                        # repo agent rules
docs/agent-skills/               # install guides (this folder)
crates/enchi-core/src/agent_skills.rs   # installer (`enchi agent-skills`); embeds the skill
plugin.json                      # Antigravity / plugin manifest
.claude-plugin/plugin.json       # Claude Code plugin metadata
```
