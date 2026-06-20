# pyenv-native — Agent Instructions

Repo-local rules for AI agents working on **pyenv-native** or using it to manage Python on a workstation/project.

## Read first (in order)

1. `README.md` — product overview and install entry points
2. `docs/INSTRUCTIONS.md` — full install, shell init, workflows, troubleshooting
3. `docs/MCP.md` — **agent-facing** MCP tools and recommended tool order
4. `docs/CLI.md` — human CLI reference
5. `docs/ARCHITECTURE.md` — Rust crate layout when changing code
6. `docs/cursor-setup.md` — install this repo's Cursor skill

## Instruction precedence

1. Explicit user request
2. This `AGENTS.md`
3. `docs/MCP.md` for environment/runtime tasks (prefer MCP over shell guessing)
4. `README.md` / `docs/INSTRUCTIONS.md`
5. Tool defaults

## When working ON this repo (Rust / packaging)

- Primary platform: **Windows x64**; also test Linux/macOS paths when touching install or resolution logic.
- Use existing scripts: `scripts/dev-cargo.ps1`, `scripts/build-release-bundle.ps1`.
- Every install path must ship **both** `pyenv` and `pyenv-mcp`.
- Run `cargo test` in affected crates before claiming success.
- Do not break the version-selection precedence: shell → local `.python-version` → parents → global → system.
- Managed venvs live under `PYENV_ROOT/venvs/<runtime>/<name>`; prefer `pyenv venv` over ad-hoc `.venv` when documenting agent flows.

## When USING pyenv-native on another project

1. **Prefer `pyenv-mcp` tools** when MCP is configured (`get_toolkit_guide` → `resolve_project_environment` → `ensure_runtime` → `ensure_project_venv`).
2. If MCP is unavailable, use CLI: `pyenv local`, `pyenv install`, `pyenv venv create`, `pyenv which python`.
3. Never `pip install` into an unknown global Python without resolving the active interpreter first.
4. On Windows, use **PowerShell 7+ (`pwsh`)**; run `pyenv doctor` when PATH/shim issues appear.
5. Orientation blob for smaller models: `pyenv-mcp guide` (JSON).

## Verification before completion

- `pyenv version` / `pyenv which python` match intended runtime
- Project `.python-version` or managed venv spec is set when required
- `pyenv doctor` clean or known warnings documented
- For code changes: `cargo test` passes in touched crates

## Do not

- Assume classic bash `pyenv` install docs apply verbatim (this is native Rust).
- Skip checksum verification on release bundles.
- Add secrets to docs or committed config.
- Use `pip install` globally as a substitute for `pyenv install`.

## Cursor skill

Install (all agents): see [docs/agent-skills/README.md](docs/agent-skills/README.md)

```text
Install the agent skills from https://github.com/imyourboyroy/pyenv-native
```

```powershell
./scripts/install-agent-skills.ps1 -Agent all
```

Skill name: **pyenv-native**
