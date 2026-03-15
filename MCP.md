# MCP

## Purpose

`pyenv-native` ships with **`pyenv-mcp`**, a stdio MCP server that gives AI agents and MCP-capable IDEs a structured way to:

- install and inspect Python runtimes,
- understand a project's effective Python selection,
- create or reuse project-local virtual environments,
- emit install instructions for `pyenv-native` itself,
- avoid brittle shell parsing whenever a structured tool is better.

If the normal `pyenv` CLI is the human-facing surface, `pyenv-mcp` is the agent-facing surface.

---

## Why this exists

Humans are comfortable with commands like:

```text
pyenv install 3.12
pyenv local 3.12.10
pyenv which python
```

Agents do better when they can ask for:

- structured arguments,
- structured JSON responses,
- explicit side effects,
- predictable tool order,
- concrete interpreter and venv paths.

That is what `pyenv-mcp` provides.

---

## Installed automatically

All first-class install paths for `pyenv-native` now install **both**:

- `pyenv`
- `pyenv-mcp`

That includes:

- GitHub web installers (`install.ps1`, `install.sh`)
- release bundles
- the `pyenv-native-bootstrap` PyPI package

You do **not** need a separate install step for the MCP server.

---

## Fastest way to get started

Install `pyenv-native`, then print the MCP client config:

### Windows PowerShell

```powershell
pyenv-mcp print-config
```

### Linux / macOS

```sh
pyenv-mcp print-config
```

That emits a ready-to-paste JSON block like:

```json
{
  "mcpServers": {
    "pyenv-native": {
      "command": "/path/to/pyenv-mcp",
      "args": [],
      "env": {
        "PYENV_ROOT": "/path/to/.pyenv"
      }
    }
  }
}
```

---

## The single best blob for smaller or less-capable models

If you want a model to understand the toolkit quickly, give it the JSON from:

```text
pyenv-mcp guide
```

That guide is designed to be the **orientation blob** for agentic use.
It includes:

- install and uninstall commands for `pyenv-native` itself,
- MCP client configuration,
- recommended tool order,
- tool summaries,
- common workflows,
- example inputs.

This is the easiest way to make even a smaller model productive without asking it to reverse-engineer the CLI first.

---

## Core commands

### Start the MCP server

Normally your MCP client starts this automatically:

```text
pyenv-mcp
```

That starts the stdio MCP server.

### Print client config

```text
pyenv-mcp print-config
```

Useful when wiring up:

- VS Code MCP clients
- Cursor
- Claude Desktop
- local agent runners
- custom MCP launchers

### Print the toolkit guide

```text
pyenv-mcp guide
```

Useful when you want a structured JSON document that explains how to use the toolkit well.

---

## Exposed MCP tools

Current high-value tools include:

- `get_toolkit_guide`
- `get_install_instructions`
- `doctor`
- `resolve_project_environment`
- `list_available_versions`
- `ensure_runtime`
- `set_local_version`
- `set_global_version`
- `ensure_project_venv`

These are intentionally higher-level than raw shell commands.

---

## Recommended agent workflow

When possible, use this order:

1. `get_toolkit_guide`
2. `resolve_project_environment`
3. `list_available_versions` if a runtime decision is needed
4. `ensure_runtime`
5. `ensure_project_venv`
6. `doctor` when something looks wrong

This keeps the workflow structured and predictable.

---

## Common recipes

### 1. Teach a user how to install `pyenv-native`

Use:

- `get_install_instructions`

This returns:

- latest-release install commands,
- pinned-release examples,
- uninstall commands,
- MCP client config.

### 2. Install CPython for a project

Use:

1. `resolve_project_environment`
2. optionally `list_available_versions`
3. `ensure_runtime`
4. `ensure_project_venv`

### 3. Install PyPy for a project

Use:

1. `list_available_versions` with `family = "pypy"`
2. `ensure_runtime`
3. `ensure_project_venv`

### 4. Make a repo use a specific version locally

Use:

1. `ensure_runtime`
2. `set_local_version`
3. optionally `ensure_project_venv`

---

## What makes this better than shelling out blindly

`pyenv-mcp` gives agents:

- machine-readable version inventories,
- installable version catalogs,
- explicit runtime install outcomes,
- predictable project `.venv` handling,
- concrete `python` and `pip` paths,
- a single JSON guide for onboarding.

That means less guesswork, less parsing, and fewer broken multi-step environment setup flows.

---

## Example install + MCP flow

1. Install `pyenv-native`
2. Run:

```text
pyenv-mcp print-config
```

3. Register the JSON with your MCP client
4. Feed the model the output of:

```text
pyenv-mcp guide
```

5. Let the model use structured tools instead of raw shell commands whenever possible

---

## Design note

`pyenv-mcp` is built directly on `pyenv-core`.
It is **not** a fragile wrapper that scrapes the human CLI output.

That matters because it keeps the agent surface:

- faster,
- cleaner,
- more stable,
- easier to extend.

---

## Related docs

- [README.md](./README.md)
- [INSTRUCTIONS.md](./INSTRUCTIONS.md)
- [ARCHITECTURE.md](./ARCHITECTURE.md)
- [python-package/README.md](./python-package/README.md)
