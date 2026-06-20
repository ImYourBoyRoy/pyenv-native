# Windsurf

Windsurf uses `.windsurfrules` or global rules — not a native skills folder.

## Project rules

```bash
cat skills/pyenv-native/SKILL.md > .windsurfrules
```

Add `AGENTS.md` summaries if you need Rust development rules in the same project.

## Global rules

Windsurf → Settings → AI → Global Rules → paste `skills/pyenv-native/SKILL.md` (keep concise).

## Agent prompt

```text
Add the pyenv-native skill from https://github.com/imyourboyroy/pyenv-native to .windsurfrules for this project
```

## Tip

Keep 1–2 skills in `.windsurfrules`; paste MCP.md excerpts when debugging pyenv-mcp.
