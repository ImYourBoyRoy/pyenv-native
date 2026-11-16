# GitHub Copilot

## Project install

From your **project root** (not the Enchi repo):

```bash
git clone --depth 1 https://github.com/imyourboyroy/pyenv-native.git /tmp/pyenv-native
mkdir -p .github/skills
cp -R /tmp/pyenv-native/skills/* .github/skills/
```

Or from inside a clone of this repository:

```bash
enchi agent-skills --agent copilot --scope project
```

Copilot discovers skills under `.github/skills/`, `.claude/skills/`, or `.agents/skills/`.

## Agent prompt

```text
Install the Enchi agent skills into this project's .github/skills from https://github.com/imyourboyroy/pyenv-native
```

## Custom instructions

Summarize key rules in `.github/copilot-instructions.md`:

- Prefer enchi-mcp or pyenv before global pip
- Run `enchi doctor` when PATH/shims fail
- Windows: PowerShell 7+, shell init via `enchi init`

Full workflow: `skills/enchi/SKILL.md`

## References

[Creating agent skills for GitHub Copilot](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/coding-agent/create-skills)
