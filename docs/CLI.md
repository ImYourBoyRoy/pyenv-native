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
- `pyenv venv use <name>` — Activate a managed venv in the current shell.
- `pyenv local <version>/envs/<name>` — Bind a project to a managed venv by writing it to `.python-version`.

### Diagnostics

- `pyenv doctor` — Run a suite of health checks to verify your installation and PATH.
- `pyenv doctor --fix` — Attempt to automatically resolve common configuration issues.

## Shell Integration

To enable shims and the `pyenv` shell function, add the following to your shell profile:

### Windows (PowerShell Profile)

```powershell
iex ((pyenv init - pwsh) -join "`n")
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
