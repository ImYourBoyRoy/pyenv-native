---
name: enchi
description: Manages Python runtimes and project venvs via Enchi and enchi-mcp. Use when installing Python, fixing which-python/venv issues, setting .python-version, pip env problems on Windows/Linux/macOS, or when the MCP `enchi` tools are available.
---

# Enchi

Native Rust Python runtime & environment manager (pyenv-compatible; formerly pyenv-native). Human CLI: `enchi` (`pyenv` remains as an alias on upgraded installs). Agent surface: **`enchi-mcp`** MCP server (`pyenv-mcp` remains as an alias).

## When to use

- Project needs a specific Python version or venv
- `python`, `pip`, or venv paths are wrong (especially on Windows)
- Before `pip install` in any repo — resolve interpreter first
- User mentions Enchi, pyenv-native, enchi-mcp, or managed venvs

**When NOT to use:** Node/Rust-only projects with no Python; use the app's own runtime docs instead.

## Process

### 1. Orient

- If the **MCP `enchi` server is connected** (older configs may call it `pyenv-native` and launch `pyenv-mcp`): call `get_toolkit_guide`, then follow its tool order.
- If **MCP unavailable**: read repo `docs/MCP.md` and use CLI from `docs/INSTRUCTIONS.md`.
- Quick JSON orientation (terminal): `enchi-mcp guide`

### 2. Resolve environment

```
MCP:  preflight (source builds) → doctor_fix if needed → inspect_environment (optional) → resolve_project_environment → list_available_versions (if needed) → ensure_runtime
CLI:  enchi version && enchi which python && cat .python-version (if present)
```

### 3. Ensure project venv

```
MCP:  ensure_project_venv
CLI:  enchi venv create <runtime> <name>  OR  enchi local <runtime>/envs/<name>
```

### 4. Dependencies (only after interpreter is known)

```
MCP:  pip_analyze_imports → pip_precheck → pip_install / pip_outdated → pip_update → pip_check
CLI:  enchi pip precheck <target> -r requirements.txt && enchi pip install <target> -r requirements.txt
      enchi pip outdated <target> --json && enchi pip update [--all] <target> [packages...]
```

### 5. Diagnose failures

```
MCP:  doctor / doctor_fix
CLI:  enchi doctor [--json] [--fix]
```

## Recommended MCP tool order

1. `get_toolkit_guide`
2. `get_install_instructions` if Enchi is not installed
3. `preflight` before source installs on macOS / Linux / Android
4. `doctor_fix` when preflight reports automated blockers
5. `inspect_environment` for a status snapshot
6. `resolve_project_environment`
7. `list_available_versions`
8. `ensure_runtime`
9. `set_local_version` / `set_global_version` when pinning
10. `ensure_project_venv`
11. `pip_analyze_imports` / `pip_precheck` / `pip_install` as needed
12. `doctor` on failure

Register MCP: `enchi-mcp print-config` → paste into Cursor MCP settings.

## Key paths & docs

| Topic | Location |
|-------|----------|
| Full handbook | `docs/INSTRUCTIONS.md` |
| MCP tools | `docs/MCP.md` |
| CLI groups | `docs/CLI.md` |
| Install (latest) | See README — GitHub `install.ps1` / `install.sh` |
| Repo cache (after skill install) | `%USERPROFILE%\.cursor\enchi\` |

## Rationalizations (do not skip)

| Excuse | Reality |
|--------|---------|
| "I'll use system Python" | Breaks reproducibility; resolve via enchi first |
| "pip install globally is faster" | Wrong interpreter risk; use `enchi exec pip` or MCP pip tools |
| "I'll parse enchi output manually" | Use MCP structured tools when available |
| "Windows doesn't need shell init" | Shims fail without `enchi init`; run `enchi doctor` |

## Verification

- [ ] `enchi which python` points at intended runtime/venv
- [ ] `enchi version` shows expected origin (local/global/shell)
- [ ] Imports or tests run with that interpreter
- [ ] `enchi doctor` has no unexplained errors

## Install this skill

From a clone of this repository:

```sh
enchi agent-skills --agent all
```

See [docs/agent-skills/README.md](../../docs/agent-skills/README.md) and [docs/cursor-setup.md](../../docs/cursor-setup.md).
