# Getting started with agent skills

Each Roy toolkit repo ships a **`skills/`** folder with structured workflows agents follow instead of guessing CLI/API behavior.

## Supported agents

| Agent | Install method |
|-------|----------------|
| **Cursor** | Copy to `~/.cursor/skills/` or project `.cursor/skills/` |
| **Claude Code** | Plugin marketplace or `claude --plugin-dir` |
| **Gemini CLI** | `gemini skills install <repo-url> --path skills` |
| **Antigravity CLI** | `agy plugin install <repo-path-or-url>` |
| **GitHub Copilot** | Project `.github/skills/` (project scope) |
| **Kiro** | `~/.kiro/skills/` or project `.kiro/skills/` |
| **Windsurf** | `.windsurfrules` or Global Rules (paste skill content) |
| **OpenCode** | Workspace `AGENTS.md` + `skills/` |

One cross-platform installer: `pyenv agent-skills`. The skill is built into `pyenv`, so it needs no clone and no Rust toolchain.

## Tell your agent (works in any tool)

```text
Install the agent skills from https://github.com/imyourboyroy/pyenv-native. If `pyenv` is not installed, install pyenv-native first. Then run `pyenv agent-skills --agent all`.
```

## Install

```bash
pyenv agent-skills --agent all
```

Use `--agent cursor` (or any name in the table above) for one agent, and `--scope project` to install into the current directory instead of your home directory.

Gemini CLI, Antigravity, and Claude Code install from `https://github.com/imyourboyroy/pyenv-native`. The command runs `gemini` or `agy` when they are on `PATH` and prints the commands when they are not. Use `--repo-url` for a fork.

## Project-scoped install

Run from a **client project root** to install into that repo only:

```bash
pyenv agent-skills --agent copilot --scope project
```

## After install

- Invoke by name: e.g. "Follow the **\<skill-name\>** skill"
- Read repo **`AGENTS.md`** when editing toolkit source
- Per-agent details: see the other guides in this folder

## Update

```bash
pyenv self-update
pyenv agent-skills --agent all
```
