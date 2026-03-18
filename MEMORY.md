# Project Memory: pyenv-native

## 1. Project Snapshot
`pyenv-native` is a native Rust reimplementation of the `pyenv` experience designed for Windows without sacrificing Linux and macOS support. It provides local, global, and shell-scoped Python version selection with native shims and rapid rust-based execution.

## 2. Working Directory Map
- `crates/pyenv-cli/`: CLI entrypoint (clap), handles arg parsing and UX.
- `crates/pyenv-core/`: Core behavior (version resolution, `doctor`, install backends, `context` mapping).
- `crates/pyenv-mcp/`: Companion MCP server for agent integration.
- `python-package/`: PyPI/pipx bootstrap wrapper.
- `scripts/`: Build, publish, validation, and sync scripts.
- `install.ps1`/`install.sh`: Setup scripts for Windows/Linux.

## 3. Current Goals
- Ensure pyenv-native completely outclasses older tools like `pyenv-win` on Windows.
- Provide highly resilient detection of conflicts (stale paths, bad env vars).
- Present top-tier help text and diagnostic messaging to end users.

## 4. Active Tasks / TODOs
- [ ] Monitor user adoption of new `--help` outputs.
- [ ] Consider additional checks for Windows Store Python alias overrides if further isolation is required.

## 5. Architecture Notes
- **Resiliency Over Purism**: When `PYENV_ROOT` is explicitly set but points to `pyenv-win` (`pyenv-win` as the last directory component), `pyenv-native` intentionally overrides the environment variable and infers the root from its own executable location instead. This prevents older pyenv-win installations from subtly corrupting pyenv-native state.
- **Provider Backends**: Uses NuGet for Windows CPython, source builds for Linux/macOS. PyPy downloads pre-built archives.

## 6. Decisions & Conventions
- **Actionable Errors**: The `VersionNotInstalled` error now intentionally outputs a hint `run pyenv install <version>`.
- **Diagnostic-First Approach**: The `pyenv doctor` implementation checks PATH ordering to warn if users have conflicting `pyenv-win` bin directories overshadowing the `pyenv-native` shims.

## 7. Known Issues / Risks
- **Shell Artifacts**: Users migrating from `pyenv-win` may have lingering `.bat` shims in their path or cached by their shell (e.g. `rehash` equivalents needed for Powershell).

## 8. Recent Changes
- (Mar 2026) Overrode `pyenv-win` environment variables in `context.rs`. 
- (Mar 2026) Built comprehensive `[WARN]` diagnostic checks into `doctor.rs` for `pyenv-win` detection.
- (Mar 2026) Rewrote CLI help text globally across 20 commands in `meta.rs` with copy-pasteable examples.
- (Mar 2026) Merged `about` attributes down to clap structs in `main.rs`.

## 9. Validation / Tests Run
- Successfully passed `cargo test --workspace` (Exit code: 0).
- Validated `pyenv doctor` emits proper warnings when run on environments with legacy `pyenv-win` variables.
- Release binary compiled and manually injected into `~/.pyenv/bin/pyenv.exe` for local user testing.

## 10. Next Session Quick Start
- To build: `$env:PATH = "C:\Users\Roy\.cargo\bin;" + $env:PATH; cargo build --release -p pyenv-cli`
- To test: `$env:PATH = "C:\Users\Roy\.cargo\bin;" + $env:PATH; cargo test --workspace`
- Verify any further diagnostic tests using `pyenv doctor` output.
