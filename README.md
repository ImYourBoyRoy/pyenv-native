# <img src="crates/pyenv-gui/icons/icon.svg" width="48" height="48" valign="middle"> pyenv-native

![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-2563eb?style=for-the-badge)
![Runtime](https://img.shields.io/badge/runtime-Rust-D97706?style=for-the-badge&logo=rust&logoColor=white)
![PyPI](https://img.shields.io/badge/PyPI-pip%20%2F%20pipx-3775A9?style=for-the-badge&logo=pypi&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-15803D?style=for-the-badge)

**A native-first, cross-platform Python version manager inspired by `pyenv`. Built for speed and reliability on Windows, Linux, and macOS.**

`pyenv-native` is a native Rust reimplementation of the `pyenv` experience. It provides familiar workflows for version selection while removing shell and platform limitations, especially on Windows.

---

## Current Status: Actively Maturing

`pyenv-native` is currently in active development. While it is used daily by its creators, it should be considered "production-intended" but still subject to community validation.

- **Windows**: Stable (Primary platform)
- **Linux/macOS**: Tested
- **Android/Termux**: **Experimental** (Requires manual setup for compilation)

<details>
<summary><b>Android / Termux Build Prerequisites</b></summary>
<br />

Since Android/Termux does not ship with pre-built CPython binaries, `pyenv-native` automatically fetches and compiles Python from source. To prevent compilation failures, you **must** install the required compiler tools and system development libraries inside Termux first:

```bash
# 1. Update Termux package repositories
pkg update && pkg upgrade -y

# 2. Install required compilers, builders, and standard libraries
pkg install clang make pkg-config libffi openssl readline ncurses -y
```

After installing these prerequisites, running `pyenv install <version>` will compile and build your chosen Python runtime flawlessly on Android.
</details>

---

## The Ecosystem

`pyenv-native` is more than a CLI; it is a native foundation for Python development.

### 💻 [The CLI (Core Product)](./docs/CLI.md)

The high-performance core. Manages Python installations, shims, and shell integration.

- **Native-First**: No Bash dependency.
- **Opinionated Power**: Built-in managed `venv` support (replaces `pyenv-virtualenv`).
- **Validated Performance**: Reliable version selection on Windows, Linux, and macOS.

- **Dashboard**: Live view of your managed environments.
- **Visual Control**: Install versions and manage venvs with one click.
- **Status**: Stable on Windows; Experimental on Linux/macOS.

#### GUI Standalone (Latest)

- **Windows**: [Download .exe](https://github.com/imyourboyroy/pyenv-native/releases/latest/download/pyenv-gui-windows-x64.exe)
  - *Note: You may need to Right-click -> Properties -> **Unblock** if Windows SmartScreen blocks execution.*
- **Linux**: [Download Binary](https://github.com/imyourboyroy/pyenv-native/releases/latest/download/pyenv-gui-linux-x64)
- **macOS (Apple Silicon)**: [Download Binary](https://github.com/imyourboyroy/pyenv-native/releases/latest/download/pyenv-gui-macos-arm64)
- **macOS (Intel)**: [Download Binary](https://github.com/imyourboyroy/pyenv-native/releases/latest/download/pyenv-gui-macos-x64)
  - *Note: On Linux/macOS, run `chmod +x <binary>` in your terminal before launching.*

### 🤖 [Agentic / MCP Support](./docs/MCP.md)

A structured bridge for AI models like Claude or Gemini.

- **Standardized**: Built-in MCP server support.
- **Model-Friendly**: Allows AI agents to inspect, configure, and manage Python environments safely.

---

## Installation

`pyenv-native` can be installed using modern package managers or standard interactive terminal scripts.

### 1. Package Managers (Recommended)

| Platform | Command | Description |
| :--- | :--- | :--- |
| **Windows (winget)** | `winget install pyenv-native` | Direct, system-wide Windows installation |
| **macOS / Linux (Homebrew)** | `brew install imyourboyroy/pyenv-native/pyenv-native` | Universal Unix taps management |
| **Universal (pipx)** | `pipx install pyenv-native` | Isolated Python application bootstrap |

> [!NOTE]
> When installing via `pipx`, you will run the self-installer command immediately after package installation:
> ```bash
> pyenv-native install --install-root ~/.pyenv
> ```

---

### 2. Interactive Shell Scripts (Fastest)

For single-line boots directly from standard terminal shells:

#### Windows (PowerShell)
```powershell
irm https://github.com/imyourboyroy/pyenv-native/releases/latest/download/install.ps1 | iex
```

#### macOS / Linux / Android (Bash/Zsh)
```bash
curl -LsSf https://github.com/imyourboyroy/pyenv-native/releases/latest/download/install.sh | sh
```

---

## Uninstallation

If you need to remove `pyenv-native` and its shims, you can use the native CLI or standard web scripts:

### 1. Native CLI (Self-Contained)
If `pyenv` is already in your PATH, simply run:
```bash
pyenv self-uninstall
```

### 2. Interactive Shell Scripts

#### Windows (PowerShell)
```powershell
irm https://github.com/imyourboyroy/pyenv-native/releases/latest/download/uninstall.ps1 | iex
```

#### macOS / Linux (Bash/Zsh)
```bash
curl -LsSf https://github.com/imyourboyroy/pyenv-native/releases/latest/download/uninstall.sh | sh -s -- --remove-root
```

---

## Agent skills (Cursor, Claude Code, Gemini, Copilot, and more)

This repo ships **agent skills** so coding agents follow pyenv-native / pyenv-mcp workflows instead of guessing Python setup.

### Tell your agent (copy-paste)

```text
Install the agent skills from https://github.com/imyourboyroy/pyenv-native. Clone the repo, then run scripts/install-agent-skills.ps1 -Agent all on Windows (PowerShell 7+) or scripts/install-agent-skills.sh --agent all on macOS/Linux. Install for Cursor, Claude Code, Gemini CLI, Antigravity, GitHub Copilot, Windsurf, OpenCode, and Kiro.
```

### One-shot (no manual cd)

**Windows (PowerShell 7+):**

```powershell
$repo = "https://github.com/imyourboyroy/pyenv-native"
$dir = Join-Path $env:TEMP "agent-skills-$(Get-Random)"
git clone --depth 1 $repo $dir
& (Join-Path $dir "scripts/install-agent-skills.ps1") -RepoRoot $dir -Agent all
```

**macOS / Linux:**

```bash
repo="https://github.com/imyourboyroy/pyenv-native"
dir="$(mktemp -d)"
git clone --depth 1 "$repo" "$dir"
chmod +x "$dir/scripts/install-agent-skills.sh"
"$dir/scripts/install-agent-skills.sh" --repo-root "$dir" --agent all
```

### Quick install

**Windows (PowerShell 7+):**

```powershell
git clone --depth 1 https://github.com/imyourboyroy/pyenv-native.git
cd pyenv-native
./scripts/install-agent-skills.ps1 -Agent all
```

**macOS / Linux:**

```bash
git clone --depth 1 https://github.com/imyourboyroy/pyenv-native.git
cd pyenv-native
chmod +x ./scripts/install-agent-skills.sh
./scripts/install-agent-skills.sh --agent all
```

Full guides for every supported agent: **[docs/agent-skills/README.md](./docs/agent-skills/README.md)** · **[Getting started](./docs/agent-skills/getting-started.md)**

---

## Documentation Registry

Detailed technical guides and instructions:

- 📖 **[CLI Usage Guide](./docs/CLI.md)** — Core commands, `venv` management, and shell setup.
- 🎨 **[GUI Dashboard Guide](./docs/GUI.md)** — Features, screenshots, and visual management.
- 🔗 **[MCP / Agent Guide](./docs/MCP.md)** — Integration for AI models and IDEs.
- 🤖 **[Agent Skills Install](./docs/agent-skills/README.md)** — Cursor, Claude Code, Gemini CLI, Antigravity, Copilot, Windsurf, OpenCode, Kiro.
- 🏗️ **[Architecture](./docs/ARCHITECTURE.md)** — Native shims, version resolution, and design philosophy.
- 🗑️ **[Uninstallation Guide](./docs/INSTRUCTIONS.md#uninstallation)** — Safely removing `pyenv-native`.

---

## Visual Previews

### CLI Environment

```bash
$ pyenv versions
  system
* 3.13.1 (set by C:\Users\Roy\.pyenv\version)
  3.12.8
  3.12.8/envs/api  (managed venv)
```

### Categorized Help Reference

```text
SELECTION:      global, local, shell, latest, version, version-name, prefix
PROVISIONING:   install, available, versions, uninstall
ENVIRONMENT:    venv (managed virtual environments)
INTERFACE:      init, gui, rehash, shims, prompt, exec, completions
DIAGNOSTICS:    doctor, status, config, root, which, whence
MAINTENANCE:    self-update, self-uninstall
```

### GUI Dashboard

![Pyenv-Native GUI Animation](docs/screenshots/animated_gui.webp)

---

## Reporting Issues

If you encounter an issue, please [open a GitHub Issue](https://github.com/imyourboyroy/pyenv-native/issues). To help us troubleshoot, please include:

- **OS Version** (e.g., Windows 11, macOS Sequoia, Ubuntu 24.04)
- **Processor Architecture** (e.g., x64, ARM64/Apple Silicon)
- **Shell** (e.g., PowerShell 7, Bash, Zsh, Fish)
- **Relevant Logs** (found in your `.pyenv/logs/` directory)
- **Problematic Output** (the full command and any error messages)

> [!TIP]
> Run `pyenv doctor` to get a quick summary of your environment health if the CLI is already installed.

---

## Relationship to pyenv

`pyenv-native` is an independent reimplementation inspired by the `pyenv` experience. It is not affiliated with or endorsed by the official `pyenv` project. We thank the `pyenv` maintainers for shaping the standard for Python version management.

---

Created by: **Roy Dawson IV** | [GitHub](https://github.com/imyourboyroy) | [PyPI](https://pypi.org/user/ImYourBoyRoy/) | License: **MIT**
