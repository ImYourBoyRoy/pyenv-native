# pyenv-native CLI Guide

`pyenv-native` is a native Rust reimplementation of the `pyenv` experience. It is designed to provide familiar workflows while removing shell and platform limitations, especially on Windows.

> [!NOTE]
> **Status: Actively Maturing**
> The CLI core is the most tested part of the ecosystem. It is stable on Windows, Linux, and macOS. Android/Termux support is currently **experimental**.

## Command Reference

The `pyenv` CLI is organized into logical groups for ease of use. Run `pyenv --help` for the latest information.

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
  versions           List all Python versions available to pyenv
  uninstall          Uninstall a specific Python version

ENVIRONMENT:
  venv               Create, inspect, and assign managed virtual environments
  pip                List, check, install, and update packages for a runtime or venv

INTERFACE:
  init               Configure the shell environment for pyenv
  gui                Launch the beautiful Pyenv Native GUI dashboard
  rehash             Rehash pyenv shims (installs executables across all versions)
  shims              List existing pyenv shims
  prompt             Print a concise prompt string for the current environment
  exec               Run an executable with the selected Python version
  completions        Print command completion script

DIAGNOSTICS & CONFIG:
  doctor             Verify pyenv installation and environment health
  preflight          Platform intelligence and install readiness
  environment        Alias for preflight
  status             Show the comprehensive environment status (versions, origins, venvs)
  config             Display or modify pyenv-native configuration
  root               Display the root directory where versions and shims are kept
  which              Display the full path to an executable
  whence             List all Python versions that contain the given executable
  version-file       Detect the file that sets the current pyenv version
  version-file-read  Read the contents of a .python-version file

MAINTENANCE:
  self-update        Check for or install the latest published pyenv-native release
  self-uninstall     Uninstall pyenv-native from your system

SUPPORT:
  help               Display help for a command
  commands           List all available pyenv commands
  hooks              List executable hooks for a given command
```

## Core Commands

### Version Selection

- `pyenv global [version]` — Set or show the global Python version.
- `pyenv local [version]` — Set or show the project-local Python version (via `.python-version`).
- `pyenv shell [version]` — Set or show the shell-specific Python version.

### Installation

- `pyenv install --list` — List all installable Python versions.
- `pyenv install <version>` — Download and install a specific Python version.
- `pyenv uninstall <version>` — Remove an installed version.

### Introspection

- `pyenv version` — Show the current active Python version and its origin.
- `pyenv versions` — List all installed Python versions.
- `pyenv which <command>` — Show the full path to an executable (e.g., `pip`).
- `pyenv whence <command>` — List all Python versions that contain the given executable.

## Native Power Features

### Managed Virtual Environments (`venv`)

Unlike upstream `pyenv` which requires a plugin (`pyenv-virtualenv`), `pyenv-native` has built-in, first-class support for managed venvs.

- `pyenv venv create <version> <name>` — Create a named venv under the managed root.
- `pyenv venv list` — List all managed venvs.
- `pyenv venv info <spec>` — Show details for a managed venv.
- `pyenv venv use <name>` — Assign local (default) or global selection to a managed venv spec. This writes a version file; it does not activate a shell session.
- `pyenv venv rename <spec> <new-name>` — Rename a managed venv.
- `pyenv venv delete <spec>` — Remove a managed venv.
- `pyenv venv upgrade <spec> <new_runtime>` — Migrate a managed venv to another installed runtime. Inventories packages first and fails closed if that scan cannot run. Creates a temporary env, restores packages, then removes the old env and renames.
- `pyenv local <version>/envs/<name>` — Bind a project to a managed venv by writing it to `.python-version`.

### Pip Package Management & Diagnostics (`pip`)

`pyenv-native` includes a robust, conflict-safe `pip` integration suite to manage environment dependencies cleanly.

* `pyenv pip list <target> [--json]` — List installed third-party libraries and versions inside a runtime version or managed venv target.
* `pyenv pip outdated <target> [--json]` — Check PyPI for available upgrades. JSON objects are `{name, version, latest_version}`.
* `pyenv pip check <target> [--json]` — Run `pip check` for broken requirements.
* `pyenv pip precheck <target> -r <requirements>` — Statically resolve a local file or HTTPS URL and report conflicts before install. Pip option lines (`-r`, `--index-url`, …) and unparseable versions fail closed.
* `pyenv pip analyze <target> [dir]` — Scan Python sources for third-party imports missing from the target environment.
* `pyenv pip install <target> [-r requirements.txt]` — Idempotently install packages from a local file path or remote HTTPS URL (GitHub blob paths are translated to raw). `http://` is rejected.
* `pyenv pip update <target> [--all] [packages...]` — Upgrade named packages, optional `name==version` pins, or every outdated package with `--all`. Outdated `pip` is upgraded first. `--all` fails closed if the outdated scan cannot run.

```text
pyenv pip outdated 3.14.7/envs/api --json
pyenv pip update 3.14.7/envs/api certifi cryptography
pyenv pip update 3.14.7/envs/api certifi==2026.7.22
pyenv pip update --all 3.14.7/envs/api
```

`pip_outdated` JSON is `{name, version, latest_version}`. There is no "submit this JSON blob" command: copy names or pins into `pip update`, or use `--all`.

### Diagnostics & Self-Healing

- `pyenv doctor` — Run a suite of health checks to verify your installation, PATH, and platform prerequisites.
  - **Android/Termux Auditing:** On Termux environments, the doctor will automatically audit the state of required compiler toolchains (`clang`, `make`, `pkg-config`) and system header libraries (`libffi`, `openssl`, `readline`, `ncurses`) to verify source-compilation readiness.
- `pyenv doctor --fix` — Attempt to automatically resolve common configuration issues and self-heal missing Termux build dependencies via automated package installation (`pkg install ...`). On Windows, `--fix` (and GUI Self-Healing) removes App Installer `python.exe` / `python3.exe` stubs from `%LOCALAPPDATA%\Microsoft\WindowsApps` when those stubs sit ahead of pyenv shims. That is the functional fix; Windows does not expose a public API to flip the Settings toggles. Turning **App Installer python.exe** and **python3.exe** off in Settings is optional and only prevents Windows from recreating the stubs. Windows 11: Settings > Apps > Advanced app settings > App execution aliases (`ms-settings:advanced-apps`). Windows 10: Settings > Apps > App execution aliases.
- `pyenv preflight` / `pyenv environment` — Report OS, toolchain, and source-build readiness before compiling Python.

## Shell Integration

To enable shims and the `pyenv` shell function, add the following to your shell profile:

### Windows (PowerShell Profile)

```powershell
$__pyenv_init = (pyenv init - pwsh) -join "`n"
if ($__pyenv_init) { Invoke-Expression $__pyenv_init }
```

### Bash (`~/.bashrc`)

```bash
eval "$(pyenv init - bash)"
```

### Zsh (`~/.zshrc`)

```bash
eval "$(pyenv init - zsh)"
```

### Fish (`~/.config/fish/config.fish`)

```fish
pyenv init - fish | source
```

---

For the full technical details on how shims and version resolution work, see [ARCHITECTURE.md](./ARCHITECTURE.md).
