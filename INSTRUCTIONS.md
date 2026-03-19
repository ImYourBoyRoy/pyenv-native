# INSTRUCTIONS

## Purpose

This document is the full, detailed usage guide for `pyenv-native`.

If `README.md` is the fast public overview, this file is the clear step-by-step handbook for:

- installation,
- uninstallation,
- shell setup,
- runtime installation,
- version selection,
- configuration,
- troubleshooting,
- development workflows.

---

## How release selection works

By default, the web installers and the Python install package target the **latest published GitHub release**.

That is intentional.

- `main` / `master` are source branches,
- published releases are versioned artifacts,
- installers should prefer checksum-verifiable release assets,
- reproducible installs should use an explicit tag such as `vX.Y.Z`.

So the rule is:

- omit `--tag` for the latest published release,
- use `--tag <vX.Y.Z>` only when you want to pin a specific published release.

When you pin a release, it is best to fetch the installer script from that same tag as well.

---

## Installation options

`pyenv-native` supports multiple entry paths depending on what kind of machine and workflow you have.

### Option 1: latest published release with no clone

#### Windows PowerShell latest-release install

```powershell
$installer = Join-Path $env:TEMP 'pyenv-native-install.ps1'; Invoke-WebRequest https://raw.githubusercontent.com/imyourboyroy/pyenv-native/main/install.ps1 -OutFile $installer; & $installer
```

#### Linux / macOS latest-release install

```sh
curl -fsSL https://raw.githubusercontent.com/imyourboyroy/pyenv-native/main/install.sh | sh
```

On Android / Termux, the same installer now resolves to the dedicated Android ARM64 release artifact. If `pyenv` is not available in the current shell immediately after install, open a new shell or run `. ~/.bashrc`.

Google's built-in Android Terminal app is different from Termux: it runs a Debian Linux VM, so it should use the Linux ARM64 bundle instead of the Termux-specific Android artifact.

This is the simplest path when you just want `pyenv-native` installed quickly from GitHub.

These entrypoints are interactive by default. They print a preflight summary, show the install root and integration changes, then ask for confirmation before they proceed. For automation, add `-Yes` on Windows or `--yes` on Linux/macOS.

### Option 2: pin a specific published release

#### Windows PowerShell pinned install

```powershell
$tag = 'vX.Y.Z'; $installer = Join-Path $env:TEMP 'pyenv-native-install.ps1'; Invoke-WebRequest "https://raw.githubusercontent.com/imyourboyroy/pyenv-native/$tag/install.ps1" -OutFile $installer; & $installer -Tag $tag -InstallRoot "$HOME\.pyenv" -Force
```

#### Linux / macOS pinned install

```sh
tag='vX.Y.Z'; curl -fsSL "https://raw.githubusercontent.com/imyourboyroy/pyenv-native/${tag}/install.sh" | sh -s -- --tag "$tag" --install-root ~/.pyenv
```

### Option 3: use the PyPI / `pipx` install package

This is useful when Python already exists on the machine and you want a Python-native entrypoint that still installs the native runtime.

#### `pipx` latest-release install

```powershell
pipx install pyenv-native
pyenv-native install --github-repo imyourboyroy/pyenv-native --install-root ~\.pyenv
```

#### `pip` latest-release install

```sh
python -m pip install pyenv-native
pyenv-native install --github-repo imyourboyroy/pyenv-native --install-root ~/.pyenv
```

### Option 4: install from a local bundle

This is useful for offline or staged release validation.

## Updating pyenv-native in place

Once `pyenv-native` is installed as a portable root-managed install, you can update it directly:

```text
pyenv self-update
pyenv self-update --check
pyenv self-update --tag vX.Y.Z
```

Use `--yes` for unattended automation.

### Important install note

All first-class install paths now install both:

- `pyenv`
- `pyenv-mcp`

That means the normal GitHub web installers, release bundles, and Python install package all give you the human CLI and the agent-friendly MCP server together.

## Installer UX and safety model

The install path is intentionally explicit rather than magical. Whether you start from the GitHub-hosted web installer or a local bundle, the workflow is:

1. detect platform and architecture,
2. resolve the target release bundle,
3. print a preflight summary showing source, install root, shell integration, and log path,
4. prompt for confirmation unless `-Yes` / `--yes` was supplied,
5. verify the bundle checksum,
6. install into a portable root,
7. run basic sanity checks like `pyenv --version`, `pyenv root`, and `pyenv commands`.

### Logs

By default, the installers write logs beneath the selected install root:

- Windows: `<InstallRoot>\logs\network-install-*.log`
- Linux/macOS: `<install-root>/logs/network-install-*.log`

The bundled local installers also accept an explicit log-path override when you need deterministic automation logs.

### Confirmation and automation

- interactive runs prompt for consent before install,
- non-interactive automation should pass `-Yes` on Windows or `--yes` on Linux/macOS,
- `-Force` / `--force` is reserved for replacing an existing portable install at the same root.

### Elevation and permissions

For the default user-scoped install roots, administrator rights are usually unnecessary.

If you target a protected location instead, the installers stop early and explain that you should either:

- rerun with elevated permissions, or
- choose a user-writable install root.

#### Windows local-bundle install

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1 -BundleUrl file:///C:/path/to/dist/pyenv-native-windows-x64.zip -ChecksumUrl file:///C:/path/to/dist/pyenv-native-windows-x64.zip.sha256 -InstallRoot $HOME\.pyenv -Force
```

#### Linux / macOS local-bundle install

```sh
sh ./install.sh --bundle-url file:///tmp/pyenv-native-linux-x64.tar.gz --checksum-url file:///tmp/pyenv-native-linux-x64.tar.gz.sha256 --install-root ~/.pyenv --force
```

---

## What the installer does

The installer flow is designed to be safe, explicit, and portable.

It performs or supports:

- platform and architecture detection,
- release/tag selection,
- existing-install detection,
- checksum verification,
- install-root selection,
- shell/profile integration controls,
- confirmation before install unless automation flags are used,
- install log creation under `<install-root>/logs/`,
- post-install sanity checks against the installed binary,
- optional force/replace behavior,
- uninstall helpers for reversing installer-owned shell/profile changes.

### Default behavior highlights

- installs under `PYENV_ROOT` or the platform default managed root,
- defaults to a user-writable install root so admin rights are typically unnecessary,
- surfaces an explicit elevation warning when the selected root is not writable,
- avoids Windows registry registration by default,
- keeps managed runtimes portable,
- wires shell init in a reversible way,
- writes an install log beneath the chosen root,
- leaves the native runtime as the source of truth.

---

## Uninstall options

### Windows web uninstall

```powershell
$uninstaller = Join-Path $env:TEMP 'pyenv-native-uninstall.ps1'; Invoke-WebRequest https://raw.githubusercontent.com/imyourboyroy/pyenv-native/main/uninstall.ps1 -OutFile $uninstaller; & $uninstaller -RemoveRoot
```

### Linux / macOS web uninstall

```sh
curl -fsSL https://raw.githubusercontent.com/imyourboyroy/pyenv-native/main/uninstall.sh | sh -s -- --remove-root
```

### Windows local uninstall helper

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\uninstall-pyenv-native.ps1 -InstallRoot .\portable-pyenv -RemoveRoot
```

### Linux / macOS local uninstall helper

```sh
sh ./scripts/uninstall-pyenv-native.sh --install-root ~/.pyenv --remove-root
```

### Uninstall behavior

The uninstall helpers can remove:

- installer-added PATH changes,
- installer-added shell/profile blocks,
- the portable install root itself.

When the installer-created profile block was the only content in a profile file, the uninstall flow removes the empty file rather than leaving a blank stub behind.

---

## First-time shell setup

After installation, initialize your shell so shim resolution and shell-scoped version changes work comfortably.

### PowerShell shell init

```powershell
iex ((pyenv init - pwsh) -join "`n")
```

### CMD shell init

```cmd
FOR /F "delims=" %i IN ('pyenv init - cmd') DO @%i
```

### Bash shell init

```sh
eval "$(pyenv init - bash)"
```

### Zsh shell init

```sh
eval "$(pyenv init - zsh)"
```

### Fish shell init

```fish
pyenv init - fish | source
```

### POSIX sh shell init

```sh
eval "$(pyenv init - sh)"
```

---

## MCP / AI agent integration

`pyenv-native` includes **`pyenv-mcp`**, a stdio MCP server built directly on `pyenv-core`.

This exists so agents and MCP-capable IDEs can use structured tools instead of scraping the human CLI.

### Quick commands

```text
pyenv-mcp
pyenv-mcp guide
pyenv-mcp print-config
```

### What these do

- `pyenv-mcp` starts the stdio MCP server.
- `pyenv-mcp guide` emits a structured JSON onboarding blob with install instructions, workflow guidance, tool summaries, and example inputs.
- `pyenv-mcp print-config` emits a ready-to-paste MCP client config block.

### Why the guide matters

If you are working with a smaller or less-capable model, the best first move is usually to give it the JSON from:

```text
pyenv-mcp guide
```

That single blob is designed to teach the model:

- how to install `pyenv-native`,
- how to register the MCP server,
- what tools exist,
- what order to use them in,
- how to install CPython or PyPy runtimes,
- how to prepare a project-local `.venv`.

### Recommended tool order for agents

1. `get_toolkit_guide`
2. `resolve_project_environment`
3. `list_available_versions` when choosing a runtime
4. `ensure_runtime`
5. `ensure_project_venv`
6. `doctor` when something looks wrong

For the full MCP-specific guide, see [`MCP.md`](./MCP.md).

---

## Common workflows

### List installable runtimes

```powershell
pyenv install --list
pyenv install --list --family cpython 3.13
pyenv install --list --family pypy --json
pyenv install --list --known --family pypy pypy3.11
pyenv available 3
pyenv available 3.12
```

### Install a runtime

```powershell
pyenv install 3.13.12
pyenv install 3.12
pyenv install pypy3.11
```

### Preview an install plan before downloading or building

```powershell
pyenv install --dry-run 3.12
pyenv install --dry-run --json 3.13
```

### Set versions

```powershell
pyenv global 3.13.12
pyenv local 3.12.10
pyenv shell 3.12.10
```

### Manage named virtual environments

```powershell
pyenv venv create 3.13 api
pyenv venv list
pyenv venv info api
pyenv venv use api
pyenv local 3.13.12/envs/api
```

Managed envs live under `PYENV_ROOT/versions/<runtime>/envs/<name>`.
That gives you predictable names, avoids hidden project-specific duplication, and lets
`.python-version` point directly at a managed env spec such as `3.13.12/envs/api`.

### Inspect selection and resolution

```powershell
pyenv version
pyenv version --bare
pyenv version-origin
pyenv version-name
pyenv which python
pyenv whence python
pyenv prefix
pyenv versions
```

### Run commands through the selected runtime

```powershell
pyenv exec python -V
pyenv exec pip --version
```

### Refresh shims

```powershell
pyenv rehash
pyenv shims --short
```

### Remove runtimes

```powershell
pyenv uninstall -f 3.12.10
```

---

## Version-selection behavior

`pyenv-native` follows the expected selection precedence:

1. shell override,
2. local `.python-version`,
3. parent directory `.python-version` traversal,
4. global version,
5. `system` fallback.

This is shared across:

- `version-name`,
- `version`,
- `which`,
- `whence`,
- `prefix`,
- `exec`.

This is a major part of the compatibility contract with the `pyenv` experience.

---

## Configuration reference

### Show configuration

```powershell
pyenv config show
```

### Get a single config key

```powershell
pyenv config get storage.versions_dir
```

### Set a config value

```powershell
pyenv config set venv.auto_create_base_venv true
pyenv config set storage.versions_dir D:\PythonRuntimes
```

### Config keys

| Key | Default | Purpose |
| --- | --- | --- |
| `storage.versions_dir` | `<PYENV_ROOT>/versions` | Managed runtimes directory |
| `storage.cache_dir` | `<PYENV_ROOT>/cache` | Download and metadata cache |
| `windows.registry_mode` | `disabled` | Registry integration policy |
| `install.arch` | `auto` | Requested install architecture |
| `install.source_base_url` | provider default | Override provider source URL |
| `install.python_build_path` | unset | Optional non-Windows fallback path |
| `install.bootstrap_pip` | `true` | Bootstrap pip after install |
| `venv.auto_create_base_venv` | `false` | Create a companion base venv after install |
| `venv.auto_use_base_venv` | `false` | Prefer that base venv during command lookup |

### Venv policy

Companion base-venv support exists for users who want a more protected default runtime experience.

It is:

- supported,
- optional,
- off by default.

Example:

```powershell
pyenv config set venv.auto_create_base_venv true
pyenv install 3.12
```

### Managed env policy

Named managed envs are different from the optional companion base venv:

- companion base venvs are runtime-scoped internals,
- managed envs are user-facing named envs under `versions/<runtime>/envs/<name>`,
- managed envs can be written directly into `.python-version`,
- `pyenv venv create` refuses ambiguous name collisions so a short name like `api` stays predictable.

## Formal compatibility matrix

| Target | Primary artifact / backend | CI smoke | Release artifact | Notes |
| --- | --- | --- | --- | --- |
| Windows x64 | Native Windows bundle + NuGet CPython | Yes | Yes | Primary Windows path |
| Windows ARM64 | Native Windows ARM64 bundle | Yes | Yes | First-class Windows ARM target |
| macOS Intel | Native macOS x64 bundle + source CPython | Yes | Yes | Uses `macos-15-intel` |
| macOS Apple Silicon | Native macOS arm64 bundle + source CPython | Yes | Yes | Uses `macos-latest` |
| Linux x64 | Native Linux x64 bundle + source CPython | Yes | Yes | Main POSIX bundle |
| Linux ARM64 | Native Linux ARM64 musl bundle | Yes | Yes | Good fit for ARM Linux distros and Android's Debian Terminal VM |
| Android / Termux ARM64 | Native Android ARM64 bundle | Yes | Yes | Uses `aarch64-linux-android` |
| Android built-in Terminal app | Linux ARM64 bundle | Indirectly via Linux ARM64 | Yes | Uses the Debian VM rather than Android userspace |
| Windows x86 / Linux x86 / Android x86_64 | Not first-class today | No | No | Possible future expansion, not currently published |

---

## Diagnostics and troubleshooting

### Doctor output

```powershell
pyenv doctor
pyenv doctor --json
pyenv doctor --fix
```

`doctor` helps surface issues around:

- root detection,
- shim visibility,
- system Python visibility,
- Windows Store alias conflicts,
- Linux/macOS/Android source-build readiness,
- missing managed env selections,
- shell init / PATH repair hints.

### Source-build readiness on Linux/macOS

Current checks include:

- shell availability,
- `make` / `gmake`,
- compiler availability,
- `pkg-config` visibility.

### Useful help surfaces

```powershell
pyenv help
pyenv help install
pyenv commands
pyenv hooks rehash
pyenv completions install --family
```

---

## Development workflows

### Build the native CLI

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\dev-cargo.ps1 build
```

### Test the native workspace

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\dev-cargo.ps1 test
```

On Windows, you can override the Rust ABI target when needed:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\dev-cargo.ps1 -TargetTriple x86_64-pc-windows-msvc test
```

### Build the Windows release bundle

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-release-bundle.ps1 -OutputRoot .\dist
```

### Build the Linux/macOS release bundle

```sh
sh ./scripts/build-release-bundle.sh --output-root ./dist
```

### Build the Python install package

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-python-bootstrap.ps1 -PythonPath C:\path\to\python.exe
```

### Preview the MCP guide and client config

```powershell
cargo run -q -p pyenv-mcp -- guide
cargo run -q -p pyenv-mcp -- print-config
```

### Generate Winget packaging artifacts

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-winget-manifests.ps1 -GitHubRepo imyourboyroy/pyenv-native -Tag vX.Y.Z -Validate
```

### Generate Homebrew packaging artifacts

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-homebrew-formula.ps1 -GitHubRepo imyourboyroy/pyenv-native -Tag vX.Y.Z -AssetRoots .\dist\linux, .\dist\macos
```

---

## Public command surface

### Version and environment selection

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

### Discovery, install, and execution

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

### Help, hooks, diagnostics, and configuration

```text
pyenv help [--usage] [command]
pyenv commands [--sh|--no-sh]
pyenv hooks [--complete] <hook>
pyenv completions <command> [arg1 arg2...]
pyenv doctor [--json] [--fix] [-f|--force]
pyenv config path|show|get|set
```

### Python install package commands

```text
pyenv-native verify <bundle-archive> [--checksum-path <bundle.sha256>]
pyenv-native download [--bundle-url <url> | --release-base-url <url> | --github-repo <owner/repo>] [--tag <tag>]
pyenv-native install [--bundle-path <bundle-archive> | --release-base-url <url> | --github-repo <owner/repo>] [--tag <tag>] [--install-root <dir>]
```

### MCP companion commands

```text
pyenv-mcp
pyenv-mcp guide
pyenv-mcp print-config
```

Internal helper commands such as `sh-shell`, `sh-rehash`, and `sh-cmd` exist for shell integration, but they are intentionally not part of the normal end-user surface.

---

## Project structure

```text
README.md                       public overview
INSTRUCTIONS.md                 detailed usage guide
ARCHITECTURE.md                 technical design notes
MCP.md                          agent-facing MCP guide
install.ps1 / install.sh        remote-friendly web installers
uninstall.ps1 / uninstall.sh    remote-friendly uninstallers
crates/                         Rust CLI and core runtime
packaging/                      Winget and Homebrew generation assets
python-package/                 PyPI / pipx bootstrap wrapper
scripts/                        build, install, publish, validation, and sync helpers
```

---

## Final notes

`pyenv-native` is trying to be respectful to upstream `pyenv` while also being unafraid to improve the experience where native implementation details genuinely matter.

That means the design bias is:

- familiar where users expect familiarity,
- better where users benefit from improvement,
- and always clear enough that installation, usage, and cleanup are never mysterious.
