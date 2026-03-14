# pyenv-native

![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-2563eb?style=for-the-badge)
![Runtime](https://img.shields.io/badge/runtime-Rust-D97706?style=for-the-badge&logo=rust&logoColor=white)
![Bootstrap](https://img.shields.io/badge/bootstrap-PyPI%20%2F%20pipx-3775A9?style=for-the-badge&logo=pypi&logoColor=white)
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
- **Python bootstrap package:** [`python-package/README.md`](./python-package/README.md)

---

## Quick install - no clone required

These commands fetch the installer from GitHub and install `pyenv-native` without manually cloning the repository.

By default, the installer resolves to the **latest published GitHub release**.
That is intentional: `main` / `master` are source branches, while installs should target published, checksum-verifiable release artifacts.

The installers are intentionally explicit and operator-friendly. They print a preflight summary, show the install root and integration changes, prompt for confirmation by default, write an install log under the selected root, verify checksums, and run basic post-install sanity checks. For unattended automation, pass `-Yes` on Windows or `--yes` on Linux/macOS.

### Latest published release: Windows PowerShell

```powershell
$installer = Join-Path $env:TEMP 'pyenv-native-install.ps1'; Invoke-WebRequest https://raw.githubusercontent.com/imyourboyroy/pyenv-native/main/install.ps1 -OutFile $installer; & $installer
```

### Latest published release: Linux / macOS

```sh
curl -fsSL https://raw.githubusercontent.com/imyourboyroy/pyenv-native/main/install.sh | sh
```

### Pinned published release: Windows PowerShell

```powershell
$tag = 'vX.Y.Z'; $installer = Join-Path $env:TEMP 'pyenv-native-install.ps1'; Invoke-WebRequest "https://raw.githubusercontent.com/imyourboyroy/pyenv-native/$tag/install.ps1" -OutFile $installer; & $installer -Tag $tag -InstallRoot "$HOME\.pyenv" -Force
```

### Pinned published release: Linux / macOS

```sh
tag='vX.Y.Z'; curl -fsSL "https://raw.githubusercontent.com/imyourboyroy/pyenv-native/${tag}/install.sh" | sh -s -- --tag "$tag" --install-root ~/.pyenv
```

### Existing Python users: `pip` / `pipx`

If you already have Python installed, the bootstrap package can install the native release bundle for you.
It also defaults to the latest published GitHub release unless you pass `--tag <vX.Y.Z>`.

```powershell
pipx install pyenv-native-bootstrap
pyenv-native-bootstrap install --github-repo imyourboyroy/pyenv-native --install-root ~\.pyenv
```

```sh
python -m pip install pyenv-native-bootstrap
pyenv-native-bootstrap install --github-repo imyourboyroy/pyenv-native --install-root ~/.pyenv
```

### Clean uninstall: Windows PowerShell

```powershell
$uninstaller = Join-Path $env:TEMP 'pyenv-native-uninstall.ps1'; Invoke-WebRequest https://raw.githubusercontent.com/imyourboyroy/pyenv-native/main/uninstall.ps1 -OutFile $uninstaller; & $uninstaller -RemoveRoot
```

### Clean uninstall: Linux / macOS

```sh
curl -fsSL https://raw.githubusercontent.com/imyourboyroy/pyenv-native/main/uninstall.sh | sh -s -- --remove-root
```

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
- structured `doctor` diagnostics
- release bundles with checksums and manifests
- zero-clone web installers
- `pip` / `pipx` bootstrap path for users who already have Python

---

## Current status

`pyenv-native` is designed and validated as a release-quality, native-first implementation rather than a rough proof of concept.

The current focus is careful distribution polish:

- clean install and uninstall behavior,
- native runtime management across Windows, Linux, and macOS,
- clear public documentation,
- publish-ready release and bootstrap channels.

---

## Supported runtimes and provider backends

| Runtime | Windows | Linux | macOS |
| --- | --- | --- | --- |
| CPython | Official NuGet packages | Official CPython source releases | Official CPython source releases |
| PyPy | Official PyPy archives | Official PyPy archives | Official PyPy archives |
| Other definitions | Not yet | Optional fallback via upstream `python-build` | Optional fallback via upstream `python-build` |

### Important defaults

- Managed installs live under `PYENV_ROOT` by default.
- Windows registry integration defaults to **disabled**.
- Pip bootstrapping defaults to **enabled**.
- Companion base-venv creation is supported, but defaults to **off**.

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

### Choose versions the `pyenv` way

```powershell
pyenv global 3.13.12
pyenv local 3.12.10
pyenv shell 3.12.10
```

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
```

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
pyenv latest [-k|--known] [-b|--bare] [-f|--family <family>] <prefix>
pyenv prefix [versions...]
pyenv versions [--bare] [--skip-aliases] [--skip-envs] [--executables]
```

### Install, uninstall, and runtime execution

```text
pyenv install --list [--known] [--family <family>] [--json] [pattern]
pyenv install [--dry-run] [--force] [--json] <version>...
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
pyenv doctor [--json]
pyenv config path|show|get|set
```

For more examples and detailed explanations, see [`INSTRUCTIONS.md`](./INSTRUCTIONS.md).

---

## Development verification commands

For maintainers and contributors, common local verification commands include:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\dev-cargo.ps1 test
powershell -ExecutionPolicy Bypass -File .\scripts	est-python-bootstrap.ps1 -PythonPath C:\path	o\python.exe
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
|- install.ps1                     # remote-friendly Windows web installer entrypoint
|- install.sh                      # remote-friendly Linux/macOS web installer entrypoint
|- uninstall.ps1                   # remote-friendly Windows uninstall entrypoint
|- uninstall.sh                    # remote-friendly Linux/macOS uninstall entrypoint
|- crates/
|  |- pyenv-cli/                   # CLI entrypoint and command parsing
|  `- pyenv-core/                  # version resolution, install backends, shims, shell init, diagnostics
|- packaging/
|  |- winget/                      # Winget manifest generation and metadata
|  `- homebrew/                    # Homebrew formula generation and notes
|- python-package/                 # PyPI / pipx bootstrap wrapper
`- scripts/                        # build, install, publish, validation, and sync helpers
```

---

## Author, license, and links

Created by: **Roy Dawson IV**  
GitHub: <https://github.com/imyourboyroy>  
PyPI: <https://pypi.org/user/ImYourBoyRoy/>  
License: **MIT**
