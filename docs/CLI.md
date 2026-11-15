# Enchi CLI Guide

Enchi is a native Rust reimplementation of the `pyenv` experience. It is designed to provide familiar workflows while removing shell and platform limitations, especially on Windows.

> [!NOTE]
> **Status: Actively Maturing**
> The CLI core is the most tested part of the ecosystem. It is stable on Windows, Linux, and macOS. Android/Termux support is currently **experimental**.

## Command Reference

The `enchi` CLI is organized into logical groups for ease of use. Run `enchi --help` for the latest information.

```text
SELECTION:
  global             Set or show the global Python version
  local              Set or show the local directory Python version
  shell              Set or show the shell-specific Python version
  latest             Print the latest installed or known version matching the prefix
  version            Show the current Python version and its origin
  version-name       Show the current Python version
  version-origin     Explain how the current Python version is set
  prefix             Display paths where the given Python versions are installed

PROVISIONING:
  install            Install Python versions from native providers
  available          List installable Python versions from native providers
  versions           List all Python versions available to Enchi
  uninstall          Uninstall a specific Python version

ENVIRONMENT:
  venv               Create, inspect, and assign managed virtual environments
  pip                List, check, install, and update packages for a runtime or venv

INTERFACE:
  init               Configure the shell environment for Enchi
  gui                Launch the beautiful Enchi GUI dashboard
  rehash             Rehash enchi shims (installs executables across all versions)
  shims              List existing enchi shims
  prompt             Print a concise prompt string for the current environment
  exec               Run an executable with the selected Python version
  completions        Print command completion script

DIAGNOSTICS & CONFIG:
  doctor             Verify Enchi installation and environment health
  preflight          Platform intelligence and install readiness
  environment        Alias for preflight
  status             Show the comprehensive environment status (versions, origins, venvs)
  config             Display or modify Enchi configuration
  root               Display the root directory where versions and shims are kept
  which              Display the full path to an executable
  whence             List all Python versions that contain the given executable
  version-file       Detect the file that sets the current enchi version
  version-file-read  Read the contents of a .python-version file

MAINTENANCE:
  self-install       Install or repair Enchi from a release bundle
  self-update        Check for or install the latest published Enchi release
  self-uninstall     Uninstall Enchi from your system
  agent-skills       Install the Enchi skill for coding assistants

SUPPORT:
  help               Display help for a command
  commands           List all available enchi commands
  hooks              List executable hooks for a given command
```

## Core Commands

### Version Selection

- `enchi global [version]` — Set or show the global Python version.
- `enchi local [version]` — Set or show the project-local Python version (via `.python-version`).
- `enchi shell [version]` — Set or show the shell-specific Python version.

### Installation

- `enchi install --list` — List all installable Python versions, including explicitly labeled alpha, beta, and release-candidate builds.
- `enchi install <version>` — Download and install a specific Python version.
- `enchi upgrade [--check] [--no-venvs] [--no-repin] [--remove-old] [--json] [versions-or-lines...]` — Move installed CPython runtimes to the newest patch of the same `major.minor` line (for example `3.14.1` to `3.14.10`). Installs the new patch, migrates the managed venvs that used the old one (same `pip check`-gated path as `venv upgrade`), and rewrites matching global/local version-file lines. It never crosses a minor line (use `venv upgrade` for 3.14 to 3.15), skips PyPy, free-threaded and pre-release installs, and keeps the old runtime unless `--remove-old` is given and nothing depends on it. `--check` (alias `--dry-run`) only reports. Project `.venv` directories and `.python-version` files in other folders are not touched.
- `enchi uninstall <version>` — Remove an installed version.

### Introspection

- `enchi version` — Show the current active Python version and its origin.
- `enchi versions` — List all installed Python versions.
- `enchi which <command>` — Show the full path to an executable (e.g., `pip`).
- `enchi whence <command>` — List all Python versions that contain the given executable.

## Native Power Features

### Managed Virtual Environments (`venv`)

Unlike upstream `pyenv` which requires a plugin (`pyenv-virtualenv`), Enchi has built-in, first-class support for managed venvs.

- `enchi venv create <version> <name>` — Create a named venv under the managed root.
- `enchi venv list` — List all managed venvs.
- `enchi venv info <spec>` — Show details for a managed venv.
- `enchi venv use <name>` — Assign local (default) or global selection to a managed venv spec. This writes a version file; it does not activate a shell session.
- `enchi venv rename <spec> <new-name>` — Rename a managed venv.
- `enchi venv delete <spec>` — Remove a managed venv.
- `enchi venv upgrade <spec> <new_runtime>` — Migrate a managed venv to another installed runtime. A prefix such as `3.15` resolves to the newest installed `3.15.x`. Snapshots `pip freeze --all` requirements, tries exact restoration first, then retries ordinary version pins as lower bounds when the target Python rejects them. Direct/VCS/editable requirements remain intact. The staged environment must pass `pip check`; only then is the old environment removed and the replacement activated. `--force` permits replacing an existing target venv through a rollback path. Use `--set-local` to write the final managed-venv spec into the current directory's `.python-version`.
- `enchi local <version>/envs/<name>` — Bind a project to a managed venv by writing it to `.python-version`.

### Pip Package Management & Diagnostics (`pip`)

Enchi includes a robust, conflict-safe `pip` integration suite to manage environment dependencies cleanly.

* `enchi pip list <target> [--json]` — List installed third-party libraries and versions inside a runtime version or managed venv target.
* `enchi pip outdated <target> [--json]` — Check PyPI for available upgrades. JSON objects are `{name, version, latest_version}`.
* `enchi pip check <target> [--json]` — Run `pip check` for broken requirements.
* `enchi pip precheck <target> -r <requirements>` — Statically resolve a local file or HTTPS URL and report conflicts before install. Pip option lines (`-r`, `--index-url`, …) and unparseable versions fail closed.
* `enchi pip analyze <target> [dir]` — Scan Python sources for third-party imports missing from the target environment.
* `enchi pip install <target> [-r requirements.txt]` — Idempotently install packages from a local file path or remote HTTPS URL (GitHub blob paths are translated to raw). `http://` is rejected.
* `enchi pip update <target> [--all] [packages...]` — Upgrade named packages, optional `name==version` pins, or every outdated package with `--all`. Outdated `pip` is upgraded first. `--all` fails closed if the outdated scan cannot run.

```text
enchi pip outdated 3.14.7/envs/api --json
enchi pip update 3.14.7/envs/api certifi cryptography
enchi pip update 3.14.7/envs/api certifi==2026.7.22
enchi pip update --all 3.14.7/envs/api
```

`pip_outdated` JSON is `{name, version, latest_version}`. There is no "submit this JSON blob" command: copy names or pins into `pip update`, or use `--all`.

* `enchi pip update-all [--dry-run] [--runtimes | --venvs] [--allow-conflicts] [--json] [only...]` — Update outdated packages in every installed runtime and managed venv in one pass (or only the runtimes, venv specs, or venv names you list). After each environment, `pip check` runs; if the update left more broken requirements than before, the touched packages are pinned back and that environment is reported as `rolled back` (the rest continue). Exit code is non-zero if any environment failed or was rolled back. Progress goes to stderr, results to stdout.

### Install, Update & Uninstall

- `enchi self-install` — Install the `enchi`, `enchi-mcp`, and (when present) `enchi-gui` binaries that sit next to this executable into a portable root; wire up the shell profile (one delimited block), Windows user PATH, shims, and desktop launcher; run sanity checks. Re-run from an installed copy to repair shell integration. Key options: `--install-root`, `--shell`, `--update-shell-profile <bool>`, `--add-to-user-path <bool>`, `--refresh-shims <bool>`, `--yes`, `--force`, `--lang`.
- `enchi self-update [--check] [--yes] [--force] [--tag vX.Y.Z]` — Download and verify the release bundle, then run its `self-install` to swap binaries in place.
- `enchi agent-skills [--agent <name|all>] [--scope user|project] [--repo-url <url>]` — Install the Enchi skill for Cursor, Kiro, Copilot, Gemini CLI, Antigravity, Claude Code, and more. The skill is compiled into the binary, so no clone or network is needed; URL-based agents use the public repository (or `--repo-url`).
- `enchi self-uninstall [--yes] [--keep-data] [--keep-shell-integration]` — Remove Enchi, its shell-profile block, and Windows PATH entries. `--keep-data` keeps installed Pythons, virtual environments, and caches.

### Diagnostics & Self-Healing

- `enchi doctor` — Run a suite of health checks to verify your installation, PATH, and platform prerequisites.
  - **Android/Termux Auditing:** On Termux environments, the doctor will automatically audit the state of required compiler toolchains (`clang`, `make`, `pkg-config`) and system header libraries (`libffi`, `openssl`, `readline`, `ncurses`) to verify source-compilation readiness.
- `enchi doctor --fix` — Attempt to automatically resolve common configuration issues and self-heal missing Termux build dependencies via automated package installation (`pkg install ...`). On Windows, `--fix` (and GUI Self-Healing) removes App Installer `python.exe` / `python3.exe` stubs from `%LOCALAPPDATA%\Microsoft\WindowsApps` when those stubs sit ahead of enchi shims. That is the functional fix; Windows does not expose a public API to flip the Settings toggles. Turning **App Installer python.exe** and **python3.exe** off in Settings is optional and only prevents Windows from recreating the stubs. Windows 11: Settings > Apps > Advanced app settings > App execution aliases (`ms-settings:advanced-apps`). Windows 10: Settings > Apps > App execution aliases.
- `enchi preflight` / `enchi environment` — Report OS, toolchain, and source-build readiness before compiling Python.

## Shell Integration

To enable shims and the `enchi` shell function, add the following to your shell profile:

### Windows (PowerShell Profile)

```powershell
$__pyenv_init = (enchi init - pwsh) -join "`n"
if ($__pyenv_init) { Invoke-Expression $__pyenv_init }
```

### Bash (`~/.bashrc`)

```bash
eval "$(enchi init - bash)"
```

### Zsh (`~/.zshrc`)

```bash
eval "$(enchi init - zsh)"
```

### Fish (`~/.config/fish/config.fish`)

```fish
enchi init - fish | source
```

---

For the full technical details on how shims and version resolution work, see [ARCHITECTURE.md](./ARCHITECTURE.md).
