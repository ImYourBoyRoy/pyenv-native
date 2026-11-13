# INSTRUCTIONS

## Purpose

This document is the full, detailed usage guide for `pyenv-native`.

If `../README.md` is the fast public overview, this file is the clear step-by-step handbook for:

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
$installer = Join-Path $env:TEMP 'pyenv-native-install.ps1'; Invoke-WebRequest https://github.com/imyourboyroy/pyenv-native/releases/latest/download/install.ps1 -OutFile $installer; & $installer
```

#### Linux / macOS latest-release install and activate in the current shell

##### Bash

```sh
curl -fsSL https://github.com/imyourboyroy/pyenv-native/releases/latest/download/install.sh | sh && export PATH="$HOME/.pyenv/bin:$PATH" && eval "$("$HOME/.pyenv/bin/pyenv" init - bash)"
```

##### Zsh

```sh
curl -fsSL https://github.com/imyourboyroy/pyenv-native/releases/latest/download/install.sh | sh && export PATH="$HOME/.pyenv/bin:$PATH" && eval "$("$HOME/.pyenv/bin/pyenv" init - zsh)"
```

##### Fish

```fish
curl -fsSL https://github.com/imyourboyroy/pyenv-native/releases/latest/download/install.sh | sh; and if not contains -- "$HOME/.pyenv/bin" $PATH; set -gx PATH "$HOME/.pyenv/bin" $PATH; end; and "$HOME/.pyenv/bin/pyenv" init - fish | source
```

#### Linux / macOS latest-release install only

Use this form if you are fine opening a new shell afterward or reloading your profile manually:

```sh
curl -fsSL https://github.com/imyourboyroy/pyenv-native/releases/latest/download/install.sh | sh
```

On Android / Termux, the same installer now resolves to the dedicated Android ARM64 release artifact. Because `curl ... | sh` runs in a child shell, it cannot modify the already-open parent shell automatically. If you want `pyenv` available in the current shell immediately after install, run `export PATH="$HOME/.pyenv/bin:$PATH"` followed by `eval "$("$HOME/.pyenv/bin/pyenv" init - bash)"` (or swap `bash` for `zsh` on macOS). Reloading your profile with `. ~/.bashrc` still works too.

If you plan to build managed CPython versions inside Termux, install the recommended native prerequisites first:

```sh
pkg install clang make pkg-config \
  gdbm libandroid-posix-semaphore libandroid-support libbz2 libcrypt \
  libexpat libffi liblzma libsqlite ncurses ncurses-ui-libs \
  openssl readline zlib
```

Those packages are only build prerequisites. `pyenv-native` still downloads, builds, and manages the Python versions itself.

Google's built-in Android Terminal app is different from Termux: it runs a Debian Linux VM, so it should use the Linux ARM64 bundle instead of the Termux-specific Android artifact.

This is the simplest path when you just want `pyenv-native` installed quickly from GitHub.

These entrypoints are interactive by default. They print a preflight summary, show the install root and integration changes, then ask for confirmation before they proceed. For automation, add `-Yes` on Windows or `--yes` on Linux/macOS.

### Option 2: pin a specific published release

#### Windows PowerShell pinned install

```powershell
$tag = 'vX.Y.Z'; $installer = Join-Path $env:TEMP 'pyenv-native-install.ps1'; Invoke-WebRequest "https://github.com/imyourboyroy/pyenv-native/releases/download/$tag/install.ps1" -OutFile $installer; & $installer -Tag $tag -InstallRoot "$HOME\.pyenv" -Force
```

#### Linux / macOS pinned install

```sh
tag='vX.Y.Z'; curl -fsSL "https://github.com/imyourboyroy/pyenv-native/releases/download/${tag}/install.sh" | sh -s -- --tag "$tag" --install-root ~/.pyenv
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

## Native installers

Releases attach installers for each platform. They are built by `cargo xtask installer <kind>` from the same release bundles, so every executable is compiled once.

| Kind | Asset | Owner of updates |
| --- | --- | --- |
| Windows setup | `pyenv-native-windows-<arch>-setup.exe` | pyenv-native (`pyenv self-update`) |
| Debian / Ubuntu | `pyenv-native_<version>_<arch>.deb` | `apt` |
| Fedora / RHEL / openSUSE | `pyenv-native-<version>-1.<arch>.rpm` | `dnf` / `zypper` |
| macOS (Apple silicon, Intel) | `pyenv-native-macos-<arm64\|x64>.pkg`, `.dmg` | a newer `.pkg` installed over it |
| Termux | `pyenv-native_<version>_aarch64.deb` | `pkg` |

### Windows setup

A per-user NSIS setup (no administrator rights). It unpacks the release bundle to a temporary folder and runs `pyenv self-install` from there, so it produces exactly the layout `install.ps1` does (`%USERPROFILE%\.pyenv`, user `PATH`, PowerShell profile). Consequences:

- **Upgrades are in place.** The setup finds the existing root (the Apps & features entry, then `PYENV_ROOT`) and upgrades it; installed Python versions and virtual environments are kept. Installs made by earlier releases, by `install.ps1`, or by `pyenv self-update` are all upgraded the same way.
- **`pyenv self-update` keeps working** after a setup-based install, because the layout did not change.
- **Apps & features** is populated by `pyenv self-install` itself (HKCU, so no elevation), which means script installs and updates get the entry too. Its uninstall command is `pyenv self-uninstall`; the setup ships no second uninstaller.
- Silent install: `pyenv-native-windows-x64-setup.exe /S` (optionally `/D=C:\path\to\root`, which must be last).
- PowerShell 7 is not installed by the setup (`--install-powershell7 false`); it works from Windows PowerShell 5.1 and `cmd` too.

### macOS package

`pyenv-native-macos-arm64.pkg` (Apple silicon) and `pyenv-native-macos-x64.pkg` (Intel) are flat installer packages; the matching `.dmg` is a disk image that carries the same `.pkg` plus a read-me. They require macOS 11 or newer and install system-wide, like the Linux packages:

- `pyenv` and `pyenv-mcp` in `/usr/local/bin` (already on every user's `PATH`) and the desktop app as `/Applications/Pyenv Native.app`.
- Python versions, virtual environments, and shims stay per user in `~/.pyenv`. Add `eval "$(pyenv init - zsh)"` to `~/.zshrc` to put the shims on `PATH`; the installer's last page says so.
- **Upgrade** by installing a newer `.pkg` over the old one. `pyenv self-update` declines on a package install (the file `/usr/local/share/pyenv-native/install-source` says `pkg`) and prints that instruction.
- **Uninstall** with `sudo pyenv self-uninstall --system`, which removes exactly what the installer receipt lists and forgets the receipt (`pkgutil --forget`). Run `pyenv self-uninstall` first (no sudo) if you also want `~/.pyenv` and the shell integration gone.
- **Signing.** Packages are ad-hoc signed unless the maintainer supplies Apple identities (`APPLE_CODESIGN_IDENTITY`, `APPLE_INSTALLER_IDENTITY`); they are not notarized, so a package downloaded in a browser may need Control-click → Open the first time. Downloading with `curl` does not trigger that prompt. Verify against `SHA256SUMS` first.
- CI installs the package, runs `pyenv`, upgrades over it, mounts the `.dmg`, and uninstalls on both an Apple silicon and an Intel runner before every release.

```bash
sudo installer -pkg pyenv-native-macos-arm64.pkg -target /
```

### Debian, RPM, and Termux packages

System packages install `pyenv` and `pyenv-mcp` (plus `pyenv-gui`, its launcher and icons where the bundle has them) under the system prefix: `/usr` on Debian and Fedora, `$PREFIX` on Termux. A file in `/etc/profile.d` (Termux: `$PREFIX/etc/profile.d`) puts `~/.pyenv/shims` on `PATH` for login shells; for `pyenv shell` and completions add `eval "$(pyenv init - bash)"` to your shell profile as usual.

```bash
sudo apt install ./pyenv-native_<version>_amd64.deb
sudo dnf install ./pyenv-native-<version>-1.x86_64.rpm
pkg install ./pyenv-native_<version>_aarch64.deb      # Termux
```

- **Python versions, shims, and venvs stay in `~/.pyenv`** (or `PYENV_ROOT`), per user. The package only provides the program.
- **The package manager owns updates.** Packages ship `share/pyenv-native/install-source`; `pyenv self-update` reads it, declines, and prints the right command (`sudo apt install --only-upgrade pyenv-native`, `sudo dnf upgrade pyenv-native`, `pkg upgrade pyenv-native`). The GUI update check reports the same.
- The `.deb` declares `Conflicts: pyenv` (Debian's unrelated `pyenv` package) and a `libc6` floor read from the binary itself, so installing on a too-old distribution fails cleanly instead of crashing at run time.
- **Supported distributions.** The Linux x64 bundle is built on Ubuntu 22.04 (glibc 2.35) and the release fails if the executables ever need more (`--max-glibc 35`). The `.deb` declares `libc6 (>= 2.N)` and the `.rpm` a matching `libc.so.6(GLIBC_2.N)` requirement from the binaries' real symbol versions, so unsupported systems refuse the install. CI installs and runs the packages on Ubuntu 22.04/24.04, Debian 12/13, Rocky 9, Fedora, Amazon Linux 2023, and openSUSE Leap 15.6. The ARM64 packages are static (musl) and have no glibc requirement.
- The packages are reproducible for a given commit (`SOURCE_DATE_EPOCH` or the commit time is used for file timestamps).

**Moving from a script install to a package:** run `pyenv self-uninstall --keep-data` (removes the program, shims, and shell blocks; keeps `versions/`, `venvs/`, and caches), install the package, then open a new shell. Python versions installed under `~/.pyenv/versions` are picked up as they are.

### Verifying a release

Each asset has a `.sha256` sidecar, the release carries a combined `SHA256SUMS`, a CycloneDX SBOM (`pyenv-native.cdx.json`), and GitHub build-provenance attestations:

```bash
sha256sum -c SHA256SUMS --ignore-missing
gh attestation verify pyenv-native-linux-x64.tar.gz --repo imyourboyroy/pyenv-native
```

### Building them locally

```bash
cargo xtask bundle --output-root dist
cargo xtask installer deb  --bundle dist/pyenv-native-linux-x64.tar.gz
cargo xtask installer rpm  --bundle dist/pyenv-native-linux-x64.tar.gz
cargo xtask installer termux --bundle dist/pyenv-native-android-arm64.tar.gz
cargo xtask installer nsis --bundle dist/pyenv-native-windows-x64.zip   # needs makensis
cargo xtask checksums dist
```

Debian and RPM packages are written in Rust, so they build on any runner without `dpkg-deb` or `rpmbuild`. The NSIS setup needs `makensis` (`apt install nsis`, `choco install nsis`) and can be cross-built from Linux.

## Updating pyenv-native in place

Once `pyenv-native` is installed as a portable root-managed install (scripts, the Windows setup, Homebrew-seeded roots), you can update it directly. Installs owned by a system package (`.deb`, `.rpm`, Termux, the macOS `.pkg`) are updated by the package manager instead; see [Native installers](#native-installers).

```text
pyenv self-update
pyenv self-update --check
pyenv self-update --tag vX.Y.Z
```

Use `--yes` for unattended automation.
Compatibility alias: `pyenv update` is also supported for upstream-style workflows.

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
6. on Windows, offer PowerShell 7 via `winget install --id Microsoft.PowerShell` when `pwsh` is missing (`-InstallPowerShell7 auto|true|false`),
7. install into a portable root,
8. run basic sanity checks like `pyenv --version`, `pyenv root`, and `pyenv commands`.

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

Installing is split in two so the part that touches your machine is ordinary, tested Rust:

1. **`install.sh` / `install.ps1`** are thin bootstraps. They detect the platform, pick the release/tag, download the bundle and its `.sha256` sidecar, **verify the checksum**, extract it, and run the bundled binary. They contain no install logic.
2. **`pyenv self-install`** (inside the bundle) does the real work:
   - existing-install detection and install-root safety checks,
   - copying `pyenv`, `pyenv-mcp`, and `pyenv-gui` (when present), replacing binaries atomically (and, on Windows, even while they are running),
   - shell/profile integration through one delimited `# >>> pyenv-native init >>>` block that can be updated or removed cleanly,
   - persistent user PATH on Windows (registry `REG_EXPAND_SZ` is preserved),
   - Linux desktop launcher and icons for the GUI,
   - shim refresh and post-install sanity checks against the installed binaries,
   - an install log under `<install-root>/logs/`,
   - confirmation before install unless `--yes`/`--force` is given.

The same command powers every entry point: the web bootstraps, `pyenv self-update`, the GUI's setup step, the Python bootstrap package, and Homebrew. Run it from an installed copy to repair shell integration:

```sh
pyenv self-install --yes            # re-apply PATH/profile integration and refresh shims
pyenv self-install --help           # every option
```

`pyenv self-update` downloads and verifies the target release bundle, then runs the bundle's `pyenv self-install` to swap the binaries in place. For mirrors or offline testing, set `PYENV_NATIVE_RELEASE_BASE_URL` to a directory that holds the bundle and its `.sha256` (`https://`, or `http://` on loopback).

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
$uninstaller = Join-Path $env:TEMP 'pyenv-native-uninstall.ps1'; Invoke-WebRequest https://github.com/imyourboyroy/pyenv-native/releases/latest/download/uninstall.ps1 -OutFile $uninstaller; & $uninstaller -RemoveRoot
```

### Linux / macOS web uninstall

```sh
curl -fsSL https://github.com/imyourboyroy/pyenv-native/releases/latest/download/uninstall.sh | sh -s -- --remove-root
```

### Uninstall from the installed binary

```sh
pyenv self-uninstall                  # remove everything, including installed Pythons and venvs
pyenv self-uninstall --keep-data      # remove only pyenv-native and its shell integration
pyenv self-uninstall --yes            # no prompt (required when not interactive)
```

The web uninstallers (`uninstall.sh` / `uninstall.ps1`) simply run this command from the install you point them at. `self-uninstall` refuses to run against a root that does not look like a pyenv-native install, or against `/`, your home directory, or any parent of it.

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
$__pyenv_init = (pyenv init - pwsh) -join "`n"
if ($__pyenv_init) { Invoke-Expression $__pyenv_init }
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

## GUI companion

`pyenv-native` ships with **Pyenv Native GUI**, a desktop dashboard built with Tauri v2 for visual Python environment management.

### Launching the GUI

After a native Windows x64, Linux x64, or macOS install:

```text
pyenv gui
```

Development, on any platform (stops a running GUI, builds, and launches the fresh binary):

```text
cargo xtask gui              # debug build
cargo xtask gui --release
```

Or build and run directly:

```text
cargo build -p pyenv-gui
target/debug/pyenv-gui       # pyenv-gui.exe on Windows
```

### Views

| View | Purpose |
| ------ | --------- |
| **Dashboard** | Live overview of active Python version, managed venv, pyenv root, and doctor warnings with a jump to diagnostics |
| **Install Runtimes** | Search the CPython 2/3 and PyPy catalog and install new runtimes |
| **Installed Versions** | List installed versions; Package Explorer plus a More menu for global, local, and uninstall |
| **Virtual Environments** | Create, manage, and delete named managed venvs |
| **Shell Integration** | Shell profiles, host preflight, and doctor/self-heal |
| **Settings** | Configure registry mode, architecture, pip bootstrap, and venv policies |
| **About** | Version info, links, and update status |

### Self-Update from GUI

Click **Check for Updates** in the footer. The GUI uses the same self-update API as `pyenv self-update` and will prompt before applying.

### Platform availability

Native release bundles include `pyenv-gui` on **Windows x64**, **Linux x64**, and **macOS** (arm64 and Intel). Cross-compiled Windows ARM64, Linux ARM64, and Android bundles do not include the GUI.

Standalone GUI binaries and matching `.sha256` files are attached to GitHub Releases.

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

CPython names come from the live python.org release index, per-release source-artifact listings, and prerelease release metadata, then are cached for 24 hours, so later patch and prerelease revisions appear without a new pyenv-native build. Alpha, beta, and release-candidate entries are shown as prereleases; a stable prefix such as `3.16` does not select an alpha automatically, but an exact request such as `pyenv install 3.16.0a1` does. Windows uses the NuGet index and accepts its equivalent hyphenated prerelease names. PyPy uses the upstream PyPy version manifest. The compiled `known_versions.txt` is the base seed. Newly discovered names are merged into `PYENV_ROOT/cache/metadata/known_versions.txt`.

### Install a runtime

```powershell
pyenv install 3.14.7
pyenv install 3.12
pyenv install pypy3.11
```

Runtime archives from native providers are checksum-verified against **publisher metadata fetched at install time** (NuGet SHA-512 from `api.nuget.org` registration first, python.org release-file SHA-256, pypy.org checksums page). Download-URL `.sha512` sidecars are used for NuGet only when the nupkg host is `nuget.org` and the catalog cannot be reached. Hashes are not hardcoded per version. A failed verify deletes the cached download and aborts.

Linux/macOS **python-build** fallbacks prefetch the official CPython source tarball into `PYTHON_BUILD_CACHE_PATH`, verify the publisher SHA-256, then invoke `python-build`. CPython installs fail closed if prefetch or verify fails. Non-CPython python-build definitions still warn when no tarball is present.

### Preview an install plan before downloading or building

```powershell
pyenv install --dry-run 3.12
pyenv install --dry-run --json 3.13
```

### Set versions

```powershell
pyenv global 3.14.7
pyenv local 3.12.10
pyenv shell 3.12.10
```

### Manage named virtual environments

```powershell
pyenv venv create 3.13 api
pyenv venv list
pyenv venv info api
pyenv venv use api
pyenv local 3.14.7/envs/api
pyenv venv upgrade api 3.15
```

`venv upgrade` requires the target runtime to be installed first. It snapshots `pip freeze --all`, creates a replacement under the target runtime, and first restores the original requirement lines. If ordinary exact version pins are incompatible with the target Python, it recreates the staging environment and retries those pins as lower bounds (`>=`) so pip can choose a compatible release; direct URLs, VCS, editable, marker, and option lines remain unchanged. `pip check` is a hard gate. The original environment is not deleted if inventory, restoration, compatibility resolution, verification, or activation fails. With `--force`, an existing target venv is moved to a rollback path until the new environment is active. This command migrates pyenv-native managed venvs; it does not adopt an arbitrary standalone `.venv` directory.

Managed envs live under `PYENV_ROOT/venvs/<runtime>/<name>`.
That gives you predictable names, avoids hidden project-specific duplication, and lets
`.python-version` point directly at a managed env spec such as `3.14.7/envs/api`.

Compatibility aliases from upstream pyenv workflows are also supported:

```powershell
pyenv virtualenv 3.13 api
pyenv virtualenvs
pyenv virtualenv-prefix api
pyenv activate api
pyenv deactivate
```

Those aliases intentionally reuse the same managed-venv implementation, so the recommended
long-term workflow is still `pyenv venv ...` plus `.python-version`.

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
| `windows.registry_mode` | `disabled` | On Windows, `pep514` writes HKCU PEP-514 keys under `Software\Python\PyenvNative` so IDEs can discover managed interpreters. Official `PythonCore` keys are not modified. No-op on Linux/macOS. |
| `plugins.search_path` | `true` | When true, plugin discovery also walks PATH (pyenv-compatible). Set `false` to limit to `PYENV_ROOT/plugins` and `PYENV_PLUGIN_PATH`. |
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
- managed envs are user-facing named envs under `venvs/<runtime>/<name>`,
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
- Windows Store alias conflicts (Warn when the App Installer stub is ahead of pyenv shims; Info when shims already take precedence — `doctor --fix` / GUI Self-Healing removes the `python.exe` / `python3.exe` stubs; turning the aliases off in Settings is optional),
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

Repository automation is one cross-platform Rust tool, `cargo xtask` (see `xtask/`):

```sh
cargo xtask check                  # format, clippy, tests, version sync, policy checks, Python, a11y, audit
cargo xtask check --quick          # skip the slower GUI a11y and cargo-audit steps
cargo xtask version-check          # Cargo / PyPI / bootstrap / Tauri versions agree
cargo xtask set-version 0.7.0      # bump every manifest, then run `cargo build` to refresh Cargo.lock
cargo xtask bundle                 # build release binaries and write dist/pyenv-native-<os>-<arch>.{tar.gz,zip} + .sha256
cargo xtask bundle --help          # --target, --platform, --arch, --cargo cross, --no-gui, --skip-build
cargo xtask doctor                 # is Rust (and optionally the Android toolchain) set up? --require-android makes Android mandatory
cargo xtask gui                    # build and launch the desktop GUI
cargo xtask new-locale fr-CA       # seed a Fluent locale from en-US (see docs/I18N.md)
```

Enable the repo pre-push hook so `cargo xtask check` runs automatically:

```sh
cargo xtask install-hooks
```

Protect `main` from deletion and force-push by applying the repository ruleset in `.github/rulesets/protect-main.json`. This needs the GitHub CLI signed in as a repo admin, so run it yourself rather than from a cloud agent:

```sh
repo=$(gh repo view --json nameWithOwner --jq .nameWithOwner)
id=$(gh api "repos/$repo/rulesets" --jq '.[] | select(.name == "Protect main") | .id')
if [ -n "$id" ]; then
  gh api -X PUT "repos/$repo/rulesets/$id" --input .github/rulesets/protect-main.json
else
  gh api -X POST "repos/$repo/rulesets" --input .github/rulesets/protect-main.json
fi
```

### Build the native CLI

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\dev-cargo.ps1 build
```

### Test the native workspace

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\dev-cargo.ps1 test
```

On Windows, you can override the Rust ABI target when needed:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\dev-cargo.ps1 -TargetTriple x86_64-pc-windows-msvc test
```

### Test compatibility-aware venv migration

The migration smoke tests create local wheel fixtures whose Python requirement
boundary is derived from the requested target runtime. They cover exact restore,
lower-bound fallback, `pip check`, replacement of an existing target, and
source preservation after an incompatible retry. Both scripts use a temporary
`PYENV_ROOT` and symlink/junction existing runtimes, so the installed pyenv root
is not changed. The PowerShell script accepts `INSTALLED_PYENV_ROOT` when its
runtime fixtures live outside the default installed root.

```sh
./scripts/test-venv-migration.sh 3.14.3 3.15.2
```

```powershell
.\scripts\test-venv-migration.ps1 -SourceRuntime 3.14.3 -TargetRuntime 3.15.0
```

### Build a release bundle (any platform)

```sh
cargo xtask bundle --output-root ./dist
cargo xtask bundle --target aarch64-apple-darwin --output-root ./dist   # cross target
```

The same command builds the Windows `.zip` and the Linux/macOS/Android `.tar.gz`, plus a
`.sha256` sidecar. See `cargo xtask bundle --help` for `--platform`, `--arch`, `--no-gui`, and
`--cargo` (for `cross`).

### Build the Python install package

```sh
cargo xtask python test                       # unit tests
cargo xtask python build --python /path/to/python
cargo xtask python publish --check-only       # twine check; add --repository testpypi to upload
```

### Preview the MCP guide and client config

```powershell
cargo run -q -p pyenv-mcp -- guide
cargo run -q -p pyenv-mcp -- print-config
```

### Generate Homebrew packaging artifacts

```sh
cargo xtask package homebrew --github-repo imyourboyroy/pyenv-native --tag vX.Y.Z \
  --asset-root dist/linux --asset-root dist/macos
```

### Cut a release

```sh
cargo xtask release 0.7.0 --sync-version --dry-run   # preflight, print the git steps
cargo xtask release 0.7.0 --push-branch --push-tag --watch
```

The tag push triggers `.github/workflows/release.yml`, which builds every bundle and publishes
the GitHub Release.

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
pyenv latest [-k|--known] [-b|--bypass] [-f|--force] <prefix>
pyenv prefix [versions...]
pyenv versions [--bare] [--skip-aliases] [--skip-envs] [--executables]
```

### Discovery, install, and execution

```text
pyenv install --list [--known] [--family <family>] [--json] [pattern]
pyenv available [--known] [--family <family>] [--json] [pattern]
pyenv install [--dry-run] [--force] [--json] <version>...
pyenv venv <list|info|create|delete|rename|use|upgrade> [options]
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
pyenv hooks <hook|--complete>
pyenv completions <command> [arg1 arg2...]
pyenv doctor [--json] [--fix] [-f|--force]
pyenv preflight [--json]
pyenv environment [--json]
pyenv status [--json]
pyenv gui
pyenv prompt
pyenv self-update [--check] [--yes] [--force]
pyenv self-uninstall [--yes]
pyenv pip list|outdated|check|precheck|analyze|install|update ...
pyenv config path|show|get|set
```

Query and apply pip upgrades:

```text
pyenv pip outdated 3.14.7/envs/api --json
pyenv pip update 3.14.7/envs/api certifi cryptography
pyenv pip update 3.14.7/envs/api certifi==2026.7.22
pyenv pip update --all 3.14.7/envs/api
```

`--all` fails if the outdated scan cannot run. Outdated `pip` is upgraded first.

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
README.md                       ../README.md
INSTRUCTIONS.md                 ./INSTRUCTIONS.md
ARCHITECTURE.md                 ./ARCHITECTURE.md
MCP.md                          ./MCP.md
install.ps1 / install.sh        thin bootstraps: download, verify, run `pyenv self-install`
uninstall.ps1 / uninstall.sh    thin wrappers around `pyenv self-uninstall`
xtask/                          `cargo xtask`: bundle, installers, release, check, doctor, gui, new-locale, install-hooks, agent-skills
crates/                         Rust CLI, core runtime, GUI, and MCP server
  pyenv-cli/                    CLI dispatch and argument handling
  pyenv-core/                   version resolution, install backends, shims, diagnostics
  pyenv-gui/                    Tauri v2 desktop GUI companion
  pyenv-mcp/                    stdio MCP server and agent toolkit
docs/
  screenshots/                  GUI screenshots for documentation
packaging/                      Homebrew notes
python-package/                 PyPI / pipx bootstrap wrapper
scripts/                        publish, packaging, validation, and smoke-test helpers
```

---

## Final notes

`pyenv-native` is trying to be respectful to upstream `pyenv` while also being unafraid to improve the experience where native implementation details genuinely matter.

That means the design bias is:

- familiar where users expect familiarity,
- better where users benefit from improvement,
- and always clear enough that installation, usage, and cleanup are never mysterious.
