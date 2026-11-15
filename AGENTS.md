# Enchi — Agent Instructions

Repo-local rules for AI agents working on **Enchi** or using it to manage Python on a workstation/project.

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

- Supported platforms: **Windows**, **Linux**, and **macOS**. Exercise install and resolution on the OS you touch. Android/Termux is experimental (source builds).
- Use `cargo xtask` for release/dev automation (`bundle`, `check`, `policy`, `package`, `python`, `release`, `sync-known-versions`, `version-check`, `set-version`, `doctor`, `gui`, `new-locale`, `install-hooks`) and `scripts/dev-cargo.ps1` for Windows target selection.
- Install, update, and uninstall logic lives in Rust (`crates/enchi-core/src/installer/`, exposed as `enchi self-install` / `self-update` / `self-uninstall`). `install.sh`/`install.ps1` only download, verify, and run it — do not add install logic to scripts.
- Every install path must ship **both** `enchi` and `enchi-mcp` (release bundles are built by `cargo xtask bundle`). The legacy aliases `pyenv`, `pyenv-mcp`, and `pyenv-gui` are not in the `enchi-*` bundles; `enchi self-install` creates them only when upgrading an existing pyenv-native install, and system packages ship `pyenv` / `pyenv-mcp` as symlinks.
- Every release publishes two archive sets with the same contents: `enchi-<platform>-<arch>` and the legacy `pyenv-native-<platform>-<arch>` (which also carries a `pyenv` copy of the CLI). Already-installed pyenv-native binaries download the legacy name and run `<bundle>/pyenv self-install`, so do not rename or drop it casually (`cargo xtask bundle --no-legacy-compat` / `XTASK_LEGACY_COMPAT=false` switch it off deliberately).
- Naming lives in `crates/enchi-core/src/brand.rs` and `xtask/src/brand.rs`. The GitHub repository keeps the name `imyourboyroy/pyenv-native` until it is renamed; that name is one constant in each of those files, plus the defaults in `install.sh` (`DEFAULT_GITHUB_REPO`), `install.ps1` (`-GitHubRepo`), and `python-package/src/enchi_bootstrap/github.py`.
- Run `cargo test` in affected crates before claiming success.
- Do not break the version-selection precedence: shell → local `.python-version` → parents → global → system.
- CPython install catalogs are live. Read `https://www.python.org/ftp/python/` (24h cache). The compiled `crates/enchi-core/data/known_versions.txt` is only the base seed. Publisher caches merge new names into `ENCHI_ROOT/cache/metadata/known_versions.txt` and do not rewrite the shipped seed. Windows CPython still uses NuGet, and PyPy still uses the PyPy version manifest.
- The install root is `ENCHI_ROOT` (the classic `PYENV_ROOT` is still honoured), an existing install's root such as `~/.pyenv`, or `~/.enchi` for fresh installs. Managed venvs live under `<root>/venvs/<runtime>/<name>`; prefer `enchi venv` over ad-hoc `.venv` when documenting agent flows.

## When USING Enchi on another project

1. **Prefer `enchi-mcp` tools** when MCP is configured (`get_toolkit_guide` → `resolve_project_environment` → `ensure_runtime` → `ensure_project_venv`).
2. If MCP is unavailable, use CLI: `enchi local`, `enchi install`, `enchi venv create`, `enchi which python`.
3. Never `pip install` into an unknown global Python without resolving the active interpreter first.
4. On Windows, use **PowerShell 7+ (`pwsh`)**; run `enchi doctor` when PATH/shim issues appear.
5. Orientation blob for smaller models: `enchi-mcp guide` (JSON).

## Verification before completion

- `enchi version` / `enchi which python` match intended runtime
- Project `.python-version` or managed venv spec is set when required
- `enchi doctor` clean or known warnings documented
- For code changes: `cargo test` passes in touched crates

## Do not

- Assume classic bash `pyenv` install docs apply verbatim (this is native Rust).
- Skip checksum verification on release bundles.
- Add secrets to docs or committed config.
- Use `pip install` globally as a substitute for `enchi install`.

## Cursor skill

Install (all agents): see [docs/agent-skills/README.md](docs/agent-skills/README.md)

```text
Install the agent skills from https://github.com/imyourboyroy/pyenv-native
```

```bash
enchi agent-skills --agent all
```

Skill name: **enchi**
