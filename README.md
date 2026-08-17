# <img src="crates/pyenv-gui/icons/icon.svg" width="48" height="48" valign="middle"> pyenv-native

![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-2563eb?style=for-the-badge)
![Runtime](https://img.shields.io/badge/runtime-Rust-D97706?style=for-the-badge&logo=rust&logoColor=white)
![PyPI](https://img.shields.io/badge/PyPI-pip%20%2F%20pipx-3775A9?style=for-the-badge&logo=pypi&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-15803D?style=for-the-badge)

Native Rust Python version manager inspired by `pyenv`. Ships **`pyenv`** (CLI), **`pyenv-mcp`** (agents), and **`pyenv gui`**.

Windows is the primary platform. Linux and macOS are tested. Android/Termux is experimental (source builds).

![Pyenv-Native GUI](docs/screenshots/animated_gui.webp)

---

## Install

Pick the block for the machine you are on. Coding agents: run that block, then `pyenv doctor`. Do not guess a classic bash-pyenv install.

### Windows (PowerShell 7+)

```powershell
irm https://github.com/imyourboyroy/pyenv-native/releases/latest/download/install.ps1 | iex
pyenv doctor
```

### Linux / macOS / Termux

```bash
curl -fsSL https://github.com/imyourboyroy/pyenv-native/releases/latest/download/install.sh | sh
export PATH="$HOME/.pyenv/bin:$PATH"
eval "$("$HOME/.pyenv/bin/pyenv" init - bash)"   # use zsh or fish if that is your shell
pyenv doctor
```

Open a **new terminal** after install so shims are on PATH. On Termux, install compilers first: `pkg install clang make pkg-config libffi openssl readline ncurses`.

### pipx (any OS with Python)

```bash
pipx install pyenv-native
pyenv-native install --install-root "$HOME/.pyenv"
```

Winget and Homebrew manifests are generated on each release (`dist/winget/`, `dist/homebrew/`) but are **not published** yet. Do not run `winget install` or `brew install` for pyenv-native.

Full handbook (shells, tags, custom roots): [docs/INSTRUCTIONS.md](./docs/INSTRUCTIONS.md)

---

## After install

```bash
pyenv install 3.14          # latest matching 3.14.x
pyenv global 3.14
pyenv venv create 3.14 app
pyenv local 3.14/envs/app
pyenv which python
pyenv gui                   # optional desktop UI
```

Managed venvs live under `PYENV_ROOT/venvs/<runtime>/<name>`. Version selection is **shell → local `.python-version` → parents → global → system**.

---

## Upgrade / uninstall

| | Windows | Linux / macOS |
| :--- | :--- | :--- |
| **Upgrade** | Re-run `install.ps1`, or `pyenv self-update --yes` | Re-run `install.sh`, or `pyenv self-update --yes` |
| **Uninstall** | `pyenv self-uninstall`, or `irm …/uninstall.ps1 \| iex` | `pyenv self-uninstall`, or `curl …/uninstall.sh \| sh -s -- --remove-root` |

pipx: `pipx upgrade pyenv-native` then `pyenv-native install --install-root "$HOME/.pyenv"`.

---

## For coding agents

1. If **pyenv-mcp** is connected: `get_toolkit_guide` → `resolve_project_environment` → `ensure_runtime` → `ensure_project_venv`. Details: [docs/MCP.md](./docs/MCP.md).
2. If MCP is missing: install with the OS command above, then CLI (`pyenv local`, `pyenv install`, `pyenv venv create`, `pyenv which python`).
3. Never `pip install` into an unknown global Python. Resolve the interpreter first.

Register MCP: `pyenv-mcp print-config` (paste into Cursor MCP settings). Quick JSON: `pyenv-mcp guide`.

**Install agent skills** (Cursor, Claude Code, Gemini CLI, Copilot, and more):

```text
Install the agent skills from https://github.com/imyourboyroy/pyenv-native
```

```powershell
# Windows (PowerShell 7+)
git clone --depth 1 https://github.com/imyourboyroy/pyenv-native.git
cd pyenv-native
./scripts/install-agent-skills.ps1 -Agent all
```

```bash
# macOS / Linux
git clone --depth 1 https://github.com/imyourboyroy/pyenv-native.git
cd pyenv-native
chmod +x ./scripts/install-agent-skills.sh
./scripts/install-agent-skills.sh --agent all
```

Guides: [docs/agent-skills/README.md](./docs/agent-skills/README.md) · [getting-started.md](./docs/agent-skills/getting-started.md)

---

## GUI

Launch with `pyenv gui` after a native install. Stable on Windows; experimental on Linux/macOS. Native bundles ship `pyenv-gui` next to `pyenv`. Standalone binaries and `.sha256` files are on [GitHub Releases](https://github.com/imyourboyroy/pyenv-native/releases/latest).

| Dashboard | Install Runtimes | Installed | Venvs |
| :---: | :---: | :---: | :---: |
| ![Dashboard](docs/screenshots/Dashboard.webp) | ![Install Runtimes](docs/screenshots/Available.webp) | ![Installed](docs/screenshots/Installed_Versions.webp) | ![Venvs](docs/screenshots/VENVs.webp) |

More views and feature notes: [docs/GUI.md](./docs/GUI.md)

---

## Docs

| | |
| :--- | :--- |
| [INSTRUCTIONS](./docs/INSTRUCTIONS.md) | Install, shell init, workflows, troubleshooting |
| [CLI](./docs/CLI.md) | Command reference |
| [MCP](./docs/MCP.md) | Agent tools and recommended order |
| [GUI](./docs/GUI.md) | Desktop companion |
| [ARCHITECTURE](./docs/ARCHITECTURE.md) | Crate layout |

---

## Issues

[Open an issue](https://github.com/imyourboyroy/pyenv-native/issues) with OS, architecture, shell, `pyenv doctor` output, and logs from `.pyenv/logs/`. Run `pyenv preflight` before source installs on macOS, Linux, or Android.

`pyenv-native` is an independent reimplementation inspired by `pyenv`. It is not affiliated with or endorsed by the official pyenv project.

Created by: **Roy Dawson IV** | [GitHub](https://github.com/imyourboyroy) | [PyPI](https://pypi.org/user/ImYourBoyRoy/) | License: **MIT**
