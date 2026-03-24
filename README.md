# pyenv-native

![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS%20%7C%20Android-2563eb?style=for-the-badge)
![Runtime](https://img.shields.io/badge/runtime-Rust-D97706?style=for-the-badge&logo=rust&logoColor=white)
![PyPI](https://img.shields.io/badge/PyPI-pip%20%2F%20pipx-3775A9?style=for-the-badge&logo=pypi&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-15803D?style=for-the-badge)

**A native-first, cross-platform Python version manager inspired by `pyenv`, built to feel great on Windows without giving up Linux and macOS.**

**Created by [Roy Dawson IV](https://github.com/imyourboyroy)**

## Use Case Synopsis

`pyenv-native` is a native Rust reimplementation of the `pyenv` experience for people who want:

- local, global, and shell-scoped Python version selection,
- portable Python installs under a managed root,
- clean install and uninstall flows,
- better Windows ergonomics,
- a truly cross-platform foundation.

The goal is not to be different for the sake of being different.
The goal is to keep what people love about `pyenv` and improve the places where native behavior, portability, and distribution quality matter most.

---

## A note of appreciation for upstream `pyenv`

I have loved `pyenv` for a long time. It has been genuinely inspiring and incredibly useful in my own Python workflow.

This project began as a journey to build something truly excellent for Windows. As that work matured, it became obvious that the better long-term goal was not just "Windows support," but a native-first implementation that could serve Windows, Linux, and macOS cleanly.

So `pyenv-native` is built with gratitude and respect for upstream `pyenv`. It exists because that project was worth learning from. My hope is that this becomes as useful to others as `pyenv` has been for me.

---

## Quick links

- **Full usage guide:** [`INSTRUCTIONS.md`](./INSTRUCTIONS.md)
- **Technical design:** [`ARCHITECTURE.md`](./ARCHITECTURE.md)
- **MCP / agent guide:** [`MCP.md`](./MCP.md)
- **Python install package:** [`python-package/README.md`](./python-package/README.md)

---

## Quick install - no clone required

These commands fetch the installer from GitHub and install `pyenv-native` without manually cloning the repository.

By default, the installer resolves to the **latest published GitHub release**.
That is intentional: `main` / `master` are source branches, while installs should target published, checksum-verifiable release artifacts.

The installers are intentionally explicit and operator-friendly. They print a preflight summary, show the install root and integration changes, prompt for confirmation by default, write an install log under the selected root, verify checksums, and run basic post-install sanity checks. For unattended automation, pass `-Yes` on Windows or `--yes` on Linux/macOS.

On Windows, the installer persists both `PYENV_ROOT\\bin` and `PYENV_ROOT\\shims` into your **User PATH** so `pyenv`, `python`, and `pip` resolve correctly from fresh PowerShell and CMD sessions.

### Latest published release: Windows PowerShell

```powershell
$installer = Join-Path $env:TEMP 'pyenv-native-install.ps1'; Invoke-WebRequest https://raw.githubusercontent.com/imyourboyroy/pyenv-native/main/install.ps1 -OutFile $installer; & $installer
```

### Latest published release: Linux / macOS - install and activate in the current shell

Bash:

```sh
curl -fsSL https://raw.githubusercontent.com/imyourboyroy/pyenv-native/main/install.sh | sh && export PATH="$HOME/.pyenv/bin:$PATH" && eval "$("$HOME/.pyenv/bin/pyenv" init - bash)"
```

Zsh:

```sh
curl -fsSL https://raw.githubusercontent.com/imyourboyroy/pyenv-native/main/install.sh | sh && export PATH="$HOME/.pyenv/bin:$PATH" && eval "$("$HOME/.pyenv/bin/pyenv" init - zsh)"
```

Fish:

```fish
curl -fsSL https://raw.githubusercontent.com/imyourboyroy/pyenv-native/main/install.sh | sh; and if not contains -- "$HOME/.pyenv/bin" $PATH; set -gx PATH "$HOME/.pyenv/bin" $PATH; end; and "$HOME/.pyenv/bin/pyenv" init - fish | source
```

### Latest published release: Linux / macOS - install only

Use this form if you are fine opening a new shell afterward or reloading your profile manually:

```sh
curl -fsSL https://raw.githubusercontent.com/imyourboyroy/pyenv-native/main/install.sh | sh
```

### Latest published release: Android (Termux)

```sh
# Recommended Termux prerequisites for managed CPython source builds
pkg install clang make pkg-config \
  gdbm libandroid-posix-semaphore libandroid-support libbz2 libcrypt \
  libexpat libffi liblzma libsqlite ncurses ncurses-ui-libs \
  openssl readline zlib
curl -fsSL https://raw.githubusercontent.com/imyourboyroy/pyenv-native/main/install.sh | sh
```

On Termux, the installer now targets the dedicated `pyenv-native-android-arm64.tar.gz` release artifact rather than the generic Linux ARM64 bundle.
Those `pkg` dependencies are only for building managed CPython versions inside Termux; `pyenv-native` still installs and manages Python versions itself rather than delegating version installs to `pkg`.

### Android built-in "Terminal" app

Google's built-in Android Terminal app runs a Debian Linux virtual machine, so it should use the **Linux ARM64** bundle rather than the Termux-specific Android bundle.

Because `curl ... | sh` runs the installer in a child shell, it cannot modify the already-open parent shell automatically.
If you use the `install only` form, make `pyenv` available in the current POSIX shell by first adding `~/.pyenv/bin` to `PATH`, then running the shell-specific init command:

```sh
export PATH="$HOME/.pyenv/bin:$PATH"
eval "$("$HOME/.pyenv/bin/pyenv" init - bash)"
```

For zsh:

```sh
export PATH="$HOME/.pyenv/bin:$PATH"
eval "$("$HOME/.pyenv/bin/pyenv" init - zsh)"
```

For Fish:

```fish
if not contains -- "$HOME/.pyenv/bin" $PATH
  set -gx PATH "$HOME/.pyenv/bin" $PATH
end
"$HOME/.pyenv/bin/pyenv" init - fish | source
```

Reloading your profile also works:

```sh
. ~/.bashrc
```

### Existing Python users: `pip` / `pipx`

If you already have Python installed, the PyPI package can install the native release bundle for you.
It also defaults to the latest published GitHub release.

```powershell
pipx install pyenv-native
pyenv-native install --github-repo imyourboyroy/pyenv-native --install-root ~\.pyenv
```

```sh
python -m pip install pyenv-native
pyenv-native install --github-repo imyourboyroy/pyenv-native --install-root ~/.pyenv
```

### Clean uninstall: Windows PowerShell

```powershell
$uninstaller = Join-Path $env:TEMP 'pyenv-native-uninstall.ps1'; Invoke-WebRequest https://raw.githubusercontent.com/imyourboyroy/pyenv-native/main/uninstall.ps1 -OutFile $uninstaller; & $uninstaller -RemoveRoot
```

### Clean uninstall: Linux / macOS

```sh
curl -fsSL https://raw.githubusercontent.com/imyourboyroy/pyenv-native/main/uninstall.sh | sh -s -- --remove-root
```

### Update an existing portable install

```text
pyenv self-update
pyenv self-update --check
```

Compatibility alias: `pyenv update` routes to the same native updater.

---

## Agentic / MCP integration

Every first-class install path also installs **`pyenv-mcp`**, the companion MCP server for `pyenv-native`.

That gives agents and MCP-capable IDEs a structured way to:

- inspect project Python selection,
- list installable runtimes,
- ensure a runtime exists,
- create a predictable project-local `.venv`,
- emit install instructions and MCP config as JSON.

Useful commands:

```text
pyenv-mcp guide
pyenv-mcp print-config
```

- `pyenv-mcp guide` emits a structured JSON onboarding blob for models.
- `pyenv-mcp print-config` emits a ready-to-paste MCP client config block.

For the full agent-facing workflow, see [`MCP.md`](./MCP.md).

---

## What makes it useful

### Familiar `pyenv` behavior

- `.python-version` discovery
- global version selection
- local version selection
- shell-scoped version selection
- `which`, `whence`, `prefix`, `versions`, `exec`, and `rehash`
- prefix resolution like `3.12 -> latest matching concrete version`

### Native-first runtime management

- Windows CPython installs from official NuGet packages
- Linux/macOS CPython installs from official source releases
- Windows/Linux/macOS PyPy installs from official PyPy releases
- portable install roots with configurable storage and cache directories
- native shims and shell init generation

### Better operational ergonomics

- provider-backed `install --list`
- managed `pyenv venv` environments that can be written directly into `.python-version`
- structured `doctor` diagnostics
- release bundles with checksums and manifests
- zero-clone web installers
- `pyenv-mcp` for structured, agent-friendly JSON workflows
- `pip` / `pipx` install path for users who already have Python

---

## Current status

`pyenv-native` is designed and validated as a release-quality, native-first implementation rather than a rough proof of concept.

The current focus is careful distribution polish:

- clean install and uninstall behavior,
- native runtime management across Windows, Linux, and macOS,
- clear public documentation,
- publish-ready release and package distribution channels.

---

## Supported runtimes and provider backends

| Runtime | Windows | Linux | macOS | Android (Termux) |
| --- | --- | --- | --- | --- |
| CPython | Official NuGet packages | Official CPython source releases | Official CPython source releases | Official CPython source releases |
| PyPy | Official PyPy archives | Official PyPy archives | Official PyPy archives | Official PyPy archives |
| Other definitions | Not yet | Optional fallback via upstream `python-build` | Optional fallback via upstream `python-build` | Optional fallback via upstream `python-build` |

### Important defaults

- Managed installs live under `PYENV_ROOT` by default.
- Windows registry integration defaults to **disabled**.
- Pip bootstrapping defaults to **enabled**.
- Companion base-venv creation is supported, but defaults to **off**.

## Compatibility matrix

| Target | Primary artifact / backend | CI smoke | Release artifact | Notes |
| --- | --- | --- | --- | --- |
| Windows x64 | Native Windows bundle + NuGet CPython | Yes | Yes | Primary Windows path |
| Windows ARM64 | Native Windows ARM64 bundle | Yes | Yes | Release bundle target is first-class |
| macOS Intel | Native macOS x64 bundle + source CPython | Yes | Yes | Runs on `macos-15-intel` |
| macOS Apple Silicon | Native macOS arm64 bundle + source CPython | Yes | Yes | Runs on `macos-latest` |
| Linux x64 | Native Linux x64 bundle + source CPython | Yes | Yes | Main POSIX bundle |
| Linux ARM64 | Native Linux ARM64 musl bundle | Yes | Yes | Best fit for ARM Linux distros and Android's Debian Terminal VM |
| Android / Termux ARM64 | Native Android ARM64 bundle | Yes | Yes | Uses `aarch64-linux-android` |
| Android built-in Terminal app | Linux ARM64 bundle | Indirectly via Linux ARM64 | Yes | Uses the Debian VM rather than Android userspace |
| Windows x86 / Linux x86 / Android x86_64 | Not first-class today | No | No | Possible future expansion, not currently published |

---

## Everyday usage

### Initialize your shell

PowerShell:

```powershell
iex ((pyenv init - pwsh) -join "`n")
```

Bash:

```sh
eval "$(pyenv init - bash)"
```

Zsh:

```sh
eval "$(pyenv init - zsh)"
```

Fish:

```fish
pyenv init - fish | source
```

### See what can be installed

```powershell
pyenv install --list
pyenv install --list --family cpython 3.13
pyenv install --list --family pypy --json
```

### Install runtimes

```powershell
pyenv install 3.13.12
pyenv install 3.12
pyenv install pypy3.11
```

### Search installable versions

```powershell
pyenv install --list
pyenv install --list 3.13
pyenv install --list --family cpython 3.13
pyenv available 3
pyenv available 3.12
```

### Choose versions the `pyenv` way

```powershell
pyenv global 3.13.12
pyenv local 3.12.10
pyenv shell 3.12.10
```

> [!TIP]
> If you run `pyenv local` or `pyenv global` with no arguments, they act as powerful interactive hubs, allowing you to instantly see your active environment, select a new version from a numbered list, install new versions, or build/activate managed venvs!

### Manage named virtual environments

```powershell
pyenv venv create 3.13 api
pyenv venv list
pyenv venv use api
pyenv local 3.13.12/envs/api
```

Managed envs live under `PYENV_ROOT/venvs/<runtime>/<name>`.
That means a project can point its `.python-version` file directly at a managed venv spec like
`3.13.12/envs/api`, so `python` and `pip` resolve correctly without manual activation in every shell.

Compatibility aliases are also supported for smoother upstream migration:

```powershell
pyenv virtualenv 3.13 api
pyenv virtualenvs
pyenv virtualenv-prefix api
pyenv activate api
pyenv deactivate
```

`pyenv venv ...` remains the preferred interface; the compatibility terms route through the same
managed-venv backend.

### Inspect what is active

```powershell
pyenv version
pyenv which python
pyenv whence python
pyenv prefix
pyenv versions
```

### Troubleshoot quickly

```powershell
pyenv doctor
pyenv doctor --json
pyenv doctor --fix
```

On Termux, if a managed CPython build fails, first make sure the recommended native build dependencies above are installed, then retry the `pyenv install ...` command.

For fuller installation, usage, shell, config, uninstall, and development guidance, read [`INSTRUCTIONS.md`](./INSTRUCTIONS.md).

---

## Command overview

### Version selection and lookup

```text
pyenv root
pyenv version-file [dir]
pyenv version-file-read <file>
pyenv version-file-write [-f|--force] <file> <version> [...]
pyenv version-origin
pyenv version-name [-f]
pyenv version [--bare]
pyenv global [--unset] [versions...]
pyenv local [-f] [--unset] [versions...]
pyenv shell [versions...]
pyenv latest [-k|--known] [-b|--bypass] [-f|--force] <prefix>
pyenv prefix [versions...]
pyenv versions [--bare] [--skip-aliases] [--skip-envs] [--executables]
```

### Install, uninstall, and runtime execution

```text
pyenv install --list [--known] [--family <family>] [--json] [pattern]
pyenv available [--known] [--family <family>] [--json] [pattern]
pyenv install [--dry-run] [--force] [--json] <version>...
pyenv venv <list|info|create|delete|rename|use> [options]
pyenv uninstall [-f] <version>...
pyenv which [--nosystem] [--skip-advice] <command>
pyenv whence [--path] <command>
pyenv exec <command> [args...]
pyenv rehash
pyenv shims [--short]
pyenv init [-|--path] [--no-push-path] [--no-rehash] [pwsh|cmd|bash|zsh|fish|sh]
```

### Help, hooks, diagnostics, and config

```text
pyenv help [--usage] [command]
pyenv commands [--sh|--no-sh]
pyenv hooks [--complete] <hook>
pyenv completions <command> [arg1 arg2...]
pyenv doctor [--json] [--fix] [-f|--force]
pyenv config path|show|get|set
```

### MCP companion

```text
pyenv-mcp
pyenv-mcp guide
pyenv-mcp print-config
```

For more examples and detailed explanations, see [`INSTRUCTIONS.md`](./INSTRUCTIONS.md).

---

## Development verification commands

For maintainers and contributors, common local verification commands include:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\dev-cargo.ps1 test
powershell -ExecutionPolicy Bypass -File .\scripts\test-python-bootstrap.ps1 -PythonPath C:\path\to\python.exe
cargo run -q -p pyenv-mcp -- guide
pyenv install --dry-run 3.12
```

Public-facing release usage is documented in [`INSTRUCTIONS.md`](./INSTRUCTIONS.md), while operator-oriented release steps live in the workspace-level release notes used during publishing.

---

## Project layout

```text
./
|- README.md                       # public-facing overview
|- INSTRUCTIONS.md                 # detailed usage, install, uninstall, and workflow guide
|- ARCHITECTURE.md                 # technical design and compatibility model
|- MCP.md                          # MCP / agent integration guide
|- install.ps1                     # remote-friendly Windows web installer entrypoint
|- install.sh                      # remote-friendly Linux/macOS web installer entrypoint
|- uninstall.ps1                   # remote-friendly Windows uninstall entrypoint
|- uninstall.sh                    # remote-friendly Linux/macOS uninstall entrypoint
|- crates/
|  |- pyenv-cli/                   # CLI entrypoint and command parsing
|  |- pyenv-core/                  # version resolution, install backends, shims, shell init, diagnostics
|  `- pyenv-mcp/                   # stdio MCP server and agent-facing toolkit guide
|- packaging/
|  |- winget/                      # Winget manifest generation and metadata
|  `- homebrew/                    # Homebrew formula generation and notes
|- python-package/                 # PyPI / pipx bootstrap wrapper
`- scripts/                        # build, install, publish, validation, and sync helpers
```

---

## Relationship to pyenv

`pyenv-native` is an independent reimplementation inspired by the `pyenv` user experience and workflow model.

It is not affiliated with, endorsed by, or maintained by the `pyenv` project or its maintainers.

I appreciate the original `pyenv` project for shaping the Python version management experience and for demonstrating a clean, practical design philosophy.

---

## Author, license, and links

Created by: **Roy Dawson IV**  
GitHub: <https://github.com/imyourboyroy>  
PyPI: <https://pypi.org/user/ImYourBoyRoy/>  
License: **MIT**
