# ARCHITECTURE

## Purpose

This document explains how `pyenv-native` is structured, what it is trying to preserve from upstream `pyenv`, and where it intentionally diverges in order to provide a better native experience.

The short version:

> `pyenv-native` uses upstream `pyenv` as a behavioral guide, not as a runtime code dependency.

---

## Design goals

### 1. Preserve the parts of `pyenv` people love
- local / global / shell version selection,
- `.python-version` traversal,
- familiar command names,
- predictable shim-based command resolution,
- plugin and hook extensibility.

### 2. Improve the places where native behavior matters
- first-class Windows support,
- portable managed runtimes,
- structured diagnostics,
- cleaner install and uninstall flows,
- easier distribution through bundles, shell installers, and a PyPI bootstrap package.

### 3. Stay honest about compatibility
`pyenv-native` aims for behavioral compatibility where it makes sense, but it does not try to preserve upstream shell-script implementation details when a native approach is clearly better.

---

## Compatibility model

### Upstream `pyenv` is the reference
Upstream `pyenv` remains the most important compatibility reference for:
- command semantics,
- version-selection rules,
- plugin naming conventions,
- hook naming conventions,
- overall user expectations.

### What is intentionally different
Some implementation details are intentionally different because the native runtime needs stronger platform behavior:
- runtime code is written in Rust,
- Windows support is first-class instead of an afterthought,
- shim generation is native-first,
- installer backends are provider-aware,
- install catalogs prefer real installability over dumping every known upstream definition,
- shell integration uses generated shell code backed by a native core instead of a Bash runtime.

When UX changes, the design rule is simple:

> If it changes, it should become clearer, safer, or more useful.

---

## High-level layers

```text
CLI
  -> command parsing and dispatch
Core runtime
  -> version resolution, command logic, install planning, shell/shim generation
Provider backends
  -> CPython/PyPy resolution, download, extraction/build, receipts
Shell and shim adapters
  -> PowerShell, CMD, Bash, Zsh, Fish, sh, native shims
Packaging and distribution
  -> release bundles, web installers, PyPI bootstrap, Winget/Homebrew metadata
```

---

## Repository structure

```text
crates/
  pyenv-cli/      CLI entrypoint and public command wiring
  pyenv-core/     runtime behavior, catalogs, installers, diagnostics, shell/shim logic
python-package/   Python bootstrap wrapper for native release bundles
packaging/        package-manager metadata and generators
scripts/          build, release, install, uninstall, sync, and validation helpers
```

---

## Core runtime responsibilities

The Rust core owns the behavior that should be consistent across platforms:
- `PYENV_ROOT` discovery,
- `.python-version` search and parsing,
- local/global/shell precedence,
- prefix resolution such as `3.12 -> latest matching version`,
- version origin reporting,
- executable lookup via `which` / `whence`,
- install planning,
- rehash and shim generation,
- config storage and resolution,
- diagnostics via `doctor`,
- plugin and hook resolution.

This keeps the most important behavior centralized and testable.

---

## Version-selection model

The runtime follows the familiar selection order:
1. shell environment overrides,
2. local `.python-version` files,
3. parent-directory `.python-version` traversal,
4. global version file,
5. `system` fallback.

The same selection logic is shared by:
- `version-name`,
- `version`,
- `prefix`,
- `which`,
- `whence`,
- `exec`.

This is one of the most important compatibility decisions in the project.

---

## Install provider model

`pyenv-native` separates **catalog knowledge** from **provider availability**.

### Catalog knowledge
This answers:
- what version names are known,
- what runtime families exist,
- what prefixes map to what concrete versions.

The project keeps an embedded upstream-derived known-version seed in:
- `crates/pyenv-core/data/known_versions.txt`

### Provider availability
This answers:
- what can actually be installed on this platform,
- where it comes from,
- what archive/build flow is required.

This is why `install --list` defaults to provider-backed installable versions instead of dumping the entire known universe.

---

## Current install backends

### CPython
- **Windows**: official NuGet packages
- **Linux**: official CPython source releases
- **macOS**: official CPython source releases

### PyPy
- **Windows**: official PyPy ZIP releases
- **Linux/macOS**: official PyPy tarball releases

### Fallback compatibility path
- **Linux/macOS optional fallback**: upstream `python-build` via explicit config or discovered path

This gives the project a native path for the most important cases while keeping a broader compatibility escape hatch on POSIX systems.

---

## Storage model

Managed runtimes are treated as portable assets under a single managed root.

Default layout:

```text
<PYENV_ROOT>/
  versions/
  shims/
  cache/
```

Key decisions:
- Windows registry integration defaults to **disabled**.
- install/cache locations are configurable,
- receipts are written for installed runtimes,
- package metadata and downloads are cached,
- portable installs are preferred over system-level mutation.

If the runtime is launched from `<root>/bin/pyenv(.exe)` and `PYENV_ROOT` is not explicitly set, the executable can infer that portable root automatically.

---

## Shim model

Shims are generated into `<PYENV_ROOT>/shims` and act as the execution handoff point between the user and the selected runtime.

### Windows
Windows uses a native-first shim strategy:
- `.exe` shims where appropriate,
- companion `.cmd`, `.bat`, and `.ps1` wrappers,
- shim manifest tracking,
- self-dispatch by executable name.

### POSIX
POSIX uses executable shim scripts that dispatch through the native runtime.

### Rehash behavior
`rehash`:
- scans managed runtime executables,
- regenerates shim files,
- updates the shim manifest,
- uses a lock file to avoid overlap and recover stale-lock situations.

---

## Shell integration model

The core generates shell-specific init output rather than embedding a shell runtime as the heart of the tool.

Supported shells:
- PowerShell
- CMD
- Bash
- Zsh
- Fish
- POSIX `sh`

### Why this matters
The runtime still feels like `pyenv`, but shell support is generated and coordinated by the native core.

### Current emphasis
- PowerShell is the strongest Windows path.
- CMD support is real, but interactive macro behavior is naturally harder to validate than PowerShell.
- POSIX shells are supported through generated shell code and tested portability improvements.

---

## Plugin and hook model

`pyenv-native` preserves the idea that external commands can extend the tool.

### Plugin command discovery
Plugins can be discovered from:
- `PYENV_ROOT/plugins/*/bin`,
- adjacent compatible plugin layouts,
- `PATH` entries containing `pyenv-<command>` executables.

### Hooks
Hooks are resolved from:
- `PYENV_HOOK_PATH`,
- local pyenv hook roots,
- plugin hook roots,
- common POSIX system hook roots.

The runtime supports:
- `pyenv hooks`,
- completion passthrough for plugin commands,
- upstream-style help parsing from plugin header comments,
- structured hook output directives,
- shell-style environment assignment compatibility.

This is more structured than upstream's shell-sourcing behavior, but intentionally so.

---

## Packaging and distribution model

The project is designed to distribute through native assets first.

### Primary artifacts
- Windows portable ZIP bundle
- Linux portable `.tar.gz` bundle
- macOS portable `.tar.gz` bundle
- SHA-256 checksum files
- bundle manifest metadata

### Installer entrypoints
- `install.ps1`
- `install.sh`
- `uninstall.ps1`
- `uninstall.sh`

These provide zero-clone web installation paths for end users.

### Python bootstrap package
The `pyenv-native-bootstrap` package exists for users who already have Python installed and want a `pip` / `pipx`-friendly install path.

Important principle:

> The Python package bootstraps the native runtime. It does not replace it.

### Package-manager preparation
The repo also includes generator-backed preparation for:
- Winget
- Homebrew

Those channels are intentionally treated as distribution layers on top of the release-bundle model.

---

## Diagnostics model

`pyenv doctor` exists to make common problems easier to reason about.

Current checks include:
- root visibility,
- shims visibility,
- system Python detection,
- Windows Store alias warnings,
- Linux/macOS source-build readiness:
  - shell,
  - `make` / `gmake`,
  - compiler,
  - `pkg-config`.

The project favors explicit diagnostics over silent failure.

---

## Venv policy

`pyenv-native` supports companion base-venv creation for users who want a more protected default experience.

Current policy:
- supported,
- configurable,
- default-off.

When enabled:
- a companion base venv is created after install,
- command lookup can optionally prefer that venv.

This is intentionally an enhancement, not a forced behavior change.

---

## Testing and validation philosophy

The project is treated as infrastructure, so validation matters as much as features.

Current validation emphasis:
- Rust unit and integration tests,
- Python bootstrap tests,
- Windows local validation,
- Ubuntu WSL validation,
- release bundle generation,
- install/uninstall smoke tests,
- package-manager metadata generation and validation.

The architectural goal is to keep core behaviors deterministic, portable, and easy to verify.

---

## Release engineering model

The repository includes helper scripts for:
- version synchronization,
- bundle generation,
- bootstrap package builds,
- Winget manifest generation,
- Homebrew formula generation,
- GitHub release preparation,
- PyPI publication preparation.

The release workflows are intentionally scriptable and repeatable instead of relying on tribal knowledge.

---

## Intentional non-goals

At the current stage, the project is **not** trying to:
- be a source-level fork of upstream `pyenv`,
- preserve Bash as the core runtime,
- register every managed runtime into the Windows registry by default,
- replace the native runtime with a Python implementation,
- hide platform differences that genuinely matter.

---

## Summary

`pyenv-native` is designed as a native, cross-platform runtime that keeps the best ideas from `pyenv` while improving the areas where native behavior, portability, and distribution quality matter most.

That means:
- compatibility where users expect it,
- improvement where users benefit from it,
- and a cleaner foundation for Windows, Linux, and macOS alike.