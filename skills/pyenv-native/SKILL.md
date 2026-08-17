---
name: pyenv-native
description: Manages Python runtimes and project venvs via pyenv-native and pyenv-mcp. Use when installing Python, fixing which-python/venv issues, setting .python-version, pip env problems on Windows/Linux/macOS, or when MCP pyenv-native tools are available.
---

# pyenv-native

Native Rust Python version manager (pyenv-compatible). Human CLI: `pyenv`. Agent surface: **`pyenv-mcp`** MCP server.

## When to use

- Project needs a specific Python version or venv
- `python`, `pip`, or venv paths are wrong (especially on Windows)
- Before `pip install` in any repo — resolve interpreter first
- User mentions pyenv-native, pyenv-mcp, or managed venvs

**When NOT to use:** Node/Rust-only projects with no Python; use the app's own runtime docs instead.

## Process

### 1. Orient

- If **MCP `pyenv-native` is connected**: call `get_toolkit_guide`, then follow its tool order.
- If **MCP unavailable**: read repo `docs/MCP.md` and use CLI from `docs/INSTRUCTIONS.md`.
- Quick JSON orientation (terminal): `pyenv-mcp guide`

### 2. Resolve environment

```
MCP:  preflight (source builds) → doctor_fix if needed → inspect_environment (optional) → resolve_project_environment → list_available_versions (if needed) → ensure_runtime
CLI:  pyenv version && pyenv which python && cat .python-version (if present)
```

### 3. Ensure project venv

```
MCP:  ensure_project_venv
CLI:  pyenv venv create <runtime> <name>  OR  pyenv local <runtime>/envs/<name>
```

### 4. Dependencies (only after interpreter is known)

```
MCP:  pip_analyze_imports → pip_precheck → pip_install / pip_outdated → pip_update → pip_check
CLI:  pyenv pip precheck <target> -r requirements.txt && pyenv pip install <target> -r requirements.txt
      pyenv pip outdated <target> --json && pyenv pip update [--all] <target> [packages...]
```

### 5. Diagnose failures

```
MCP:  doctor / doctor_fix
CLI:  pyenv doctor [--json] [--fix]
```

## Recommended MCP tool order

1. `get_toolkit_guide`
2. `get_install_instructions` if pyenv-native is not installed
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

Register MCP: `pyenv-mcp print-config` → paste into Cursor MCP settings.

## Key paths & docs

| Topic | Location |
|-------|----------|
| Full handbook | `docs/INSTRUCTIONS.md` |
| MCP tools | `docs/MCP.md` |
| CLI groups | `docs/CLI.md` |
| Install (latest) | See README — GitHub `install.ps1` / `install.sh` |
| Repo cache (after skill install) | `%USERPROFILE%\.cursor\pyenv-native\` |

## Rationalizations (do not skip)

| Excuse | Reality |
|--------|---------|
| "I'll use system Python" | Breaks reproducibility; resolve via pyenv first |
| "pip install globally is faster" | Wrong interpreter risk; use `pyenv exec pip` or MCP pip tools |
| "I'll parse pyenv output manually" | Use MCP structured tools when available |
| "Windows doesn't need shell init" | Shims fail without `pyenv init`; run `pyenv doctor` |

## Verification

- [ ] `pyenv which python` points at intended runtime/venv
- [ ] `pyenv version` shows expected origin (local/global/shell)
- [ ] Imports or tests run with that interpreter
- [ ] `pyenv doctor` has no unexplained errors

## Install this skill

From a clone of this repository:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\install-agent-skills.ps1 -Agent all
```

```sh
sh ./scripts/install-agent-skills.sh --agent all
```

See [docs/agent-skills/README.md](../../docs/agent-skills/README.md) and [docs/cursor-setup.md](../../docs/cursor-setup.md).
