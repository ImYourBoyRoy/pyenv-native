# INSTRUCTIONS

## Purpose

This document is the full, detailed usage guide for Enchi (the new name of pyenv-native).

If `../README.md` is the fast public overview, this file is the clear step-by-step handbook for:

- installation,
- uninstallation,
- shell setup,
- runtime installation,
- version selection,
- configuration,
- troubleshooting,
- development workflows.

### Naming and install root in one minute

- The command is **`enchi`**; the MCP server is **`enchi-mcp`**; the desktop app is **`enchi-gui`** (`enchi gui`).
- A **fresh** install puts everything under `~/.enchi` (Windows `%USERPROFILE%\.enchi`). An **existing pyenv-native install keeps its root** (usually `~/.pyenv`); `enchi self-install` detects it and upgrades it in place. Nothing is moved automatically.
- `ENCHI_ROOT` selects the install root. The classic `PYENV_ROOT` is still honoured as a fallback, and `PYENV_VERSION`, `PYENV_SHELL`, `PYENV_DIR`, and the other `PYENV_*` variables keep their names.
- Installs made by pyenv-native keep a `pyenv` alias (and `pyenv-mcp`) next to the new binaries, so scripts and MCP configs keep working.

Coming from pyenv-native? See [MIGRATING.md](./MIGRATING.md).

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

Enchi supports multiple entry paths depending on what kind of machine and workflow you have.

### Option 1: latest published release with no clone

#### Windows PowerShell latest-release install

```powershell
$installer = Join-Path $env:TEMP 'enchi-install.ps1'; Invoke-WebRequest https://github.com/imyourboyroy/pyenv-native/releases/latest/download/install.ps1 -OutFile $installer; & $installer
```

#### Linux / macOS latest-release install and activate in the current shell

##### Bash

```sh
curl -fsSL https://github.com/imyourboyroy/pyenv-native/releases/latest/download/install.sh | sh && export PATH="$HOME/.enchi/bin:$PATH" && eval "$("$HOME/.enchi/bin/enchi" init - bash)"
```

##### Zsh

```sh
curl -fsSL https://github.com/imyourboyroy/pyenv-native/releases/latest/download/install.sh | sh && export PATH="$HOME/.enchi/bin:$PATH" && eval "$("$HOME/.enchi/bin/enchi" init - zsh)"
```

##### Fish

```fish
curl -fsSL https://github.com/imyourboyroy/pyenv-native/releases/latest/download/install.sh | sh; and if not contains -- "$HOME/.enchi/bin" $PATH; set -gx PATH "$HOME/.enchi/bin" $PATH; end; and "$HOME/.enchi/bin/enchi" init - fish | source
```

#### Linux / macOS latest-release install only

Use this form if you are fine opening a new shell afterward or reloading your profile manually:

```sh
curl -fsSL https://github.com/imyourboyroy/pyenv-native/releases/latest/download/install.sh | sh
```

On Android / Termux, the same installer now resolves to the dedicated Android ARM64 release artifact. Because `curl ... | sh` runs in a child shell, it cannot modify the already-open parent shell automatically. If you want `enchi` available in the current shell immediately after install, run `export PATH="$HOME/.enchi/bin:$PATH"` followed by `eval "$("$HOME/.enchi/bin/enchi" init - bash)"` (or swap `bash` for `zsh` on macOS). Reloading your profile with `. ~/.bashrc` still works too.

If you plan to build managed CPython versions inside Termux, install the recommended native prerequisites first:

```sh
pkg install clang make pkg-config \
  gdbm libandroid-posix-semaphore libandroid-support libbz2 libcrypt \
  libexpat libffi liblzma libsqlite ncurses ncurses-ui-libs \
  openssl readline zlib
```

Those packages are only build prerequisites. Enchi still downloads, builds, and manages the Python versions itself.

Google's built-in Android Terminal app is different from Termux: it runs a Debian Linux VM, so it should use the Linux ARM64 bundle instead of the Termux-specific Android artifact.

This is the simplest path when you just want Enchi installed quickly from GitHub.

The `$HOME/.enchi/bin` paths above are for a fresh install. If you are upgrading an existing pyenv-native install, its root stays `~/.pyenv`, so substitute `$HOME/.pyenv/bin` (the installer prints the root it used).

These entrypoints are interactive by default. They print a preflight summary, show the install root and integration changes, then ask for confirmation before they proceed. For automation, add `-Yes` on Windows or `--yes` on Linux/macOS.

### Option 2: pin a specific published release

#### Windows PowerShell pinned install

```powershell
$tag = 'vX.Y.Z'; $installer = Join-Path $env:TEMP 'enchi-install.ps1'; Invoke-WebRequest "https://github.com/imyourboyroy/pyenv-native/releases/download/$tag/install.ps1" -OutFile $installer; & $installer -Tag $tag -Force
```

#### Linux / macOS pinned install

```sh
tag='vX.Y.Z'; curl -fsSL "https://github.com/imyourboyroy/pyenv-native/releases/download/${tag}/install.sh" | sh -s -- --tag "$tag"
```

Add `-InstallRoot` / `--install-root` only to choose a location; without it the installer picks `~/.enchi` for a fresh machine and keeps an existing install's root.

### Option 3: use the PyPI / `pipx` install package

This is useful when Python already exists on the machine and you want a Python-native entrypoint that still installs the native runtime.

#### `pipx` latest-release install

```powershell
pipx install enchi
enchi-bootstrap install --github-repo imyourboyroy/pyenv-native
```

#### `pip` latest-release install

```sh
python -m pip install enchi
enchi-bootstrap install --github-repo imyourboyroy/pyenv-native
```

The distribution is `enchi`; its console script is `enchi-bootstrap` (not `enchi`, so it never shadows the real binary). The old `pyenv-native` PyPI package is now a thin wrapper that depends on `enchi` and forwards to `enchi-bootstrap`. (`imyourboyroy/pyenv-native` is the GitHub repository that publishes releases; it keeps that name until the repository itself is renamed.)

### Option 4: install from a local bundle

This is useful for offline or staged release validation.

## Native installers

Releases attach installers for each platform. They are built by `cargo xtask installer <kind>` from the same release bundles, so every executable is compiled once.

| Kind | Asset | Owner of updates |
| --- | --- | --- |
| Windows setup | `enchi-windows-<arch>-setup.exe` | Enchi (`enchi self-update`) |
| Debian / Ubuntu | `enchi_<version>_<arch>.deb` | `apt` |
| Fedora / RHEL / openSUSE | `enchi-<version>-1.<arch>.rpm` | `dnf` / `zypper` |
| macOS (Apple silicon, Intel) | `enchi-macos-<arm64\|x64>.pkg`, `.dmg` | a newer `.pkg` installed over it |
| Termux | `enchi_<version>_aarch64.deb` | `pkg` |

### Windows setup

A per-user NSIS setup (no administrator rights). It unpacks the release bundle to a temporary folder and runs `enchi self-install` from there, so it produces exactly the layout `install.ps1` does (`%USERPROFILE%\.enchi`, user `PATH`, PowerShell profile). Consequences:

- **Upgrades are in place.** The setup finds the existing root (the Apps & features entry, then `ENCHI_ROOT`) and upgrades it; installed Python versions and virtual environments are kept. Installs made by earlier releases, by `install.ps1`, or by `enchi self-update` are all upgraded the same way.
- **`enchi self-update` keeps working** after a setup-based install, because the layout did not change.
- **Apps & features** is populated by `enchi self-install` itself (HKCU, so no elevation), which means script installs and updates get the entry too. Its uninstall command is `enchi self-uninstall`; the setup ships no second uninstaller.
- Silent install: `enchi-windows-x64-setup.exe /S` (optionally `/D=C:\path\to\root`, which must be last).
- PowerShell 7 is not installed by the setup (`--install-powershell7 false`); it works from Windows PowerShell 5.1 and `cmd` too.

### macOS package

`enchi-macos-arm64.pkg` (Apple silicon) and `enchi-macos-x64.pkg` (Intel) are flat installer packages; the matching `.dmg` is a disk image that carries the same `.pkg` plus a read-me. They require macOS 11 or newer and install system-wide, like the Linux packages:

- `enchi` and `enchi-mcp` in `/usr/local/bin` (already on every user's `PATH`, with `pyenv` and `pyenv-mcp` as compatibility links) and the desktop app as `/Applications/Enchi.app`.
- Python versions, virtual environments, and shims stay per user in `~/.enchi`. Add `eval "$(enchi init - zsh)"` to `~/.zshrc` to put the shims on `PATH`; the installer's last page says so.
- **Upgrade** by installing a newer `.pkg` over the old one. `enchi self-update` declines on a package install (the file `/usr/local/share/enchi/install-source` says `pkg`) and prints that instruction.
- **Uninstall** with `sudo enchi self-uninstall --system`, which removes exactly what the installer receipt lists and forgets the receipt (`pkgutil --forget`). Run `enchi self-uninstall` first (no sudo) if you also want `~/.enchi` and the shell integration gone.
- **Signing.** Packages are ad-hoc signed unless the maintainer supplies Apple identities (`APPLE_CODESIGN_IDENTITY`, `APPLE_INSTALLER_IDENTITY`); they are not notarized, so a package downloaded in a browser may need Control-click → Open the first time. Downloading with `curl` does not trigger that prompt. Verify against `SHA256SUMS` first.
- CI installs the package, runs `enchi`, upgrades over it, mounts the `.dmg`, and uninstalls on both an Apple silicon and an Intel runner before every release.

```bash
sudo installer -pkg enchi-macos-arm64.pkg -target /
```

### Debian, RPM, and Termux packages

System packages install `enchi` and `enchi-mcp` (plus `enchi-gui`, its launcher and icons where the bundle has them, and `pyenv` / `pyenv-mcp` compatibility symlinks) under the system prefix: `/usr` on Debian and Fedora, `$PREFIX` on Termux. A file in `/etc/profile.d` (Termux: `$PREFIX/etc/profile.d`) puts `~/.enchi/shims` on `PATH` for login shells; for `enchi shell` and completions add `eval "$(enchi init - bash)"` to your shell profile as usual.

```bash
sudo apt install ./enchi_<version>_amd64.deb
sudo dnf install ./enchi-<version>-1.x86_64.rpm
pkg install ./enchi_<version>_aarch64.deb      # Termux
```

- **Python versions, shims, and venvs stay per user** in `~/.enchi` (or `ENCHI_ROOT`); an existing `~/.pyenv` from pyenv-native keeps being used. The package only provides the program.
- **The package manager owns updates.** Packages ship `share/enchi/install-source`; `enchi self-update` reads it, declines, and prints the right command (`sudo apt install --only-upgrade enchi`, `sudo dnf upgrade enchi`, `pkg upgrade enchi`). The GUI update check reports the same.
- The package is named `enchi` and replaces `pyenv-native`: the `.deb` declares `Replaces`/`Provides`/`Conflicts: pyenv-native` and the `.rpm` `Obsoletes`/`Provides: pyenv-native`, so `apt`/`dnf` upgrades swap the old package for the new one. It also declares `Conflicts: pyenv` (the unrelated classic `pyenv` package, which would fight over `/usr/bin/pyenv`). The `.deb` carries a `libc6` floor read from the binary itself, so installing on a too-old distribution fails cleanly instead of crashing at run time.
- **Supported distributions.** The Linux x64 bundle is built on Ubuntu 22.04 (glibc 2.35) and the release fails if the executables ever need more (`--max-glibc 35`). The `.deb` declares `libc6 (>= 2.N)` and the `.rpm` a matching `libc.so.6(GLIBC_2.N)` requirement from the binaries' real symbol versions, so unsupported systems refuse the install. CI installs and runs the packages on Ubuntu 22.04/24.04, Debian 12/13, Rocky 9, Fedora, Amazon Linux 2023, and openSUSE Leap 15.6. The ARM64 packages are static (musl) and have no glibc requirement.
- The packages are reproducible for a given commit (`SOURCE_DATE_EPOCH` or the commit time is used for file timestamps).

**Moving from a script install to a package:** run `enchi self-uninstall --keep-data` (removes the program, shims, and shell blocks; keeps `versions/`, `venvs/`, and caches), install the package, then open a new shell. Python versions installed under `~/.enchi/versions` are picked up as they are.

### Verifying a release

Each asset has a `.sha256` sidecar, the release carries a combined `SHA256SUMS`, a CycloneDX SBOM (`enchi.cdx.json`), and GitHub build-provenance attestations:

```bash
sha256sum -c SHA256SUMS --ignore-missing
gh attestation verify enchi-linux-x64.tar.gz --repo imyourboyroy/pyenv-native
```

### Building them locally

```bash
cargo xtask bundle --output-root dist
cargo xtask installer deb  --bundle dist/enchi-linux-x64.tar.gz
cargo xtask installer rpm  --bundle dist/enchi-linux-x64.tar.gz
cargo xtask installer termux --bundle dist/enchi-android-arm64.tar.gz
cargo xtask installer nsis --bundle dist/enchi-windows-x64.zip   # needs makensis
cargo xtask checksums dist
```

Debian and RPM packages are written in Rust, so they build on any runner without `dpkg-deb` or `rpmbuild`. The NSIS setup needs `makensis` (`apt install nsis`, `choco install nsis`) and can be cross-built from Linux.

## Updating Enchi in place

Once Enchi is installed as a portable root-managed install (scripts, the Windows setup, Homebrew-seeded roots), you can update it directly. Installs owned by a system package (`.deb`, `.rpm`, Termux, the macOS `.pkg`) are updated by the package manager instead; see [Native installers](#native-installers).

```text
enchi self-update
enchi self-update --check
enchi self-update --tag vX.Y.Z
```

Use `--yes` for unattended automation.
Compatibility alias: `enchi update` is also supported for upstream-style workflows.

### Important install note

All first-class install paths now install both:

- `enchi`
- `enchi-mcp`

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
8. run basic sanity checks like `enchi --version`, `enchi root`, and `enchi commands`.

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
powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1 -BundleUrl file:///C:/path/to/dist/enchi-windows-x64.zip -ChecksumUrl file:///C:/path/to/dist/enchi-windows-x64.zip.sha256 -InstallRoot $HOME\.enchi -Force
```

#### Linux / macOS local-bundle install

```sh
sh ./install.sh --bundle-url file:///tmp/enchi-linux-x64.tar.gz --checksum-url file:///tmp/enchi-linux-x64.tar.gz.sha256 --install-root ~/.enchi --force
```

---

## What the installer does

Installing is split in two so the part that touches your machine is ordinary, tested Rust:

1. **`install.sh` / `install.ps1`** are thin bootstraps. They detect the platform, pick the release/tag, download the bundle and its `.sha256` sidecar, **verify the checksum**, extract it, and run the bundled binary. They contain no install logic.
2. **`enchi self-install`** (inside the bundle) does the real work:
   - existing-install detection and install-root safety checks,
   - copying `enchi`, `enchi-mcp`, and `enchi-gui` (when present), replacing binaries atomically (and, on Windows, even while they are running),
   - shell/profile integration through one delimited `# >>> enchi init >>>` block that can be updated or removed cleanly,
   - persistent user PATH on Windows (registry `REG_EXPAND_SZ` is preserved),
   - Linux desktop launcher and icons for the GUI,
   - shim refresh and post-install sanity checks against the installed binaries,
   - an install log under `<install-root>/logs/`,
   - confirmation before install unless `--yes`/`--force` is given.

The same command powers every entry point: the web bootstraps, `enchi self-update`, the GUI's setup step, the Python bootstrap package, and Homebrew. Run it from an installed copy to repair shell integration:

```sh
enchi self-install --yes            # re-apply PATH/profile integration and refresh shims
enchi self-install --help           # every option
```

`enchi self-update` downloads and verifies the target release bundle, then runs the bundle's `enchi self-install` to swap the binaries in place. For mirrors or offline testing, set `ENCHI_RELEASE_BASE_URL` (the earlier `PYENV_NATIVE_RELEASE_BASE_URL` is still read) to a directory that holds the bundle and its `.sha256` (`https://`, or `http://` on loopback).

### Default behavior highlights

- installs under `ENCHI_ROOT` (or `PYENV_ROOT`), an existing install's root, or the platform default managed root (`~/.enchi`),
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
$uninstaller = Join-Path $env:TEMP 'enchi-uninstall.ps1'; Invoke-WebRequest https://github.com/imyourboyroy/pyenv-native/releases/latest/download/uninstall.ps1 -OutFile $uninstaller; & $uninstaller -RemoveRoot
```

### Linux / macOS web uninstall

```sh
curl -fsSL https://github.com/imyourboyroy/pyenv-native/releases/latest/download/uninstall.sh | sh -s -- --remove-root
```

### Uninstall from the installed binary

```sh
enchi self-uninstall                  # remove everything, including installed Pythons and venvs
enchi self-uninstall --keep-data      # remove only Enchi and its shell integration
enchi self-uninstall --yes            # no prompt (required when not interactive)
```

The web uninstallers (`uninstall.sh` / `uninstall.ps1`) simply run this command from the install you point them at. `self-uninstall` refuses to run against a root that does not look like an Enchi install, or against `/`, your home directory, or any parent of it.

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
$__pyenv_init = (enchi init - pwsh) -join "`n"
if ($__pyenv_init) { Invoke-Expression $__pyenv_init }
```

### CMD shell init

```cmd
FOR /F "delims=" %i IN ('enchi init - cmd') DO @%i
```

### Bash shell init

```sh
eval "$(enchi init - bash)"
```

### Zsh shell init

```sh
eval "$(enchi init - zsh)"
```

### Fish shell init

```fish
enchi init - fish | source
```

### POSIX sh shell init

```sh
eval "$(enchi init - sh)"
```

---

## MCP / AI agent integration

Enchi includes **`enchi-mcp`**, a stdio MCP server built directly on `enchi-core`.

This exists so agents and MCP-capable IDEs can use structured tools instead of scraping the human CLI.

### Quick commands

```text
enchi-mcp
enchi-mcp guide
enchi-mcp print-config
```

### What these do

- `enchi-mcp` starts the stdio MCP server.
- `enchi-mcp guide` emits a structured JSON onboarding blob with install instructions, workflow guidance, tool summaries, and example inputs.
- `enchi-mcp print-config` emits a ready-to-paste MCP client config block.

### Why the guide matters

If you are working with a smaller or less-capable model, the best first move is usually to give it the JSON from:

```text
enchi-mcp guide
```

That single blob is designed to teach the model:

- how to install Enchi,
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

Enchi ships with **Enchi GUI**, a desktop dashboard built with Tauri v2 for visual Python environment management.

### Launching the GUI

After a native Windows x64, Linux x64, or macOS install:

```text
enchi gui
```

Development, on any platform (stops a running GUI, builds, and launches the fresh binary):

```text
cargo xtask gui              # debug build
cargo xtask gui --release
```

Or build and run directly:

```text
cargo build -p enchi-gui
target/debug/enchi-gui       # enchi-gui.exe on Windows
```

### Views

| View | Purpose |
| ------ | --------- |
| **Dashboard** | Live overview of active Python version, managed venv, install root, and doctor warnings with a jump to diagnostics |
| **Install Runtimes** | Search the CPython 2/3 and PyPy catalog and install new runtimes |
| **Installed Versions** | List installed versions; Package Explorer plus a More menu for global, local, and uninstall |
| **Virtual Environments** | Create, manage, and delete named managed venvs |
| **Shell Integration** | Shell profiles, host preflight, and doctor/self-heal |
| **Settings** | Configure registry mode, architecture, pip bootstrap, and venv policies |
| **About** | Version info, links, and update status |

### Self-Update from GUI

Click **Check for Updates** in the footer. The GUI uses the same self-update API as `enchi self-update` and will prompt before applying.

### Platform availability

Native release bundles include `enchi-gui` on **Windows x64**, **Linux x64**, and **macOS** (arm64 and Intel). Cross-compiled Windows ARM64, Linux ARM64, and Android bundles do not include the GUI.

Standalone GUI binaries and matching `.sha256` files are attached to GitHub Releases.

---

## Common workflows

### List installable runtimes

```powershell
enchi install --list
enchi install --list --family cpython 3.13
enchi install --list --family pypy --json
enchi install --list --known --family pypy pypy3.11
enchi available 3
enchi available 3.12
```

CPython names come from the live python.org release index, per-release source-artifact listings, and prerelease release metadata, then are cached for 24 hours, so later patch and prerelease revisions appear without a new Enchi build. Alpha, beta, and release-candidate entries are shown as prereleases; a stable prefix such as `3.16` does not select an alpha automatically, but an exact request such as `enchi install 3.16.0a1` does. Windows uses the NuGet index and accepts its equivalent hyphenated prerelease names. PyPy uses the upstream PyPy version manifest. The compiled `known_versions.txt` is the base seed. Newly discovered names are merged into `ENCHI_ROOT/cache/metadata/known_versions.txt`.

### Install a runtime

```powershell
enchi install 3.14.7
enchi install 3.12
enchi install pypy3.11
```

Runtime archives from native providers are checksum-verified against **publisher metadata fetched at install time** (NuGet SHA-512 from `api.nuget.org` registration first, python.org release-file SHA-256, pypy.org checksums page). Download-URL `.sha512` sidecars are used for NuGet only when the nupkg host is `nuget.org` and the catalog cannot be reached. Hashes are not hardcoded per version. A failed verify deletes the cached download and aborts.

Linux/macOS **python-build** fallbacks prefetch the official CPython source tarball into `PYTHON_BUILD_CACHE_PATH`, verify the publisher SHA-256, then invoke `python-build`. CPython installs fail closed if prefetch or verify fails. Non-CPython python-build definitions still warn when no tarball is present.

### Preview an install plan before downloading or building

```powershell
enchi install --dry-run 3.12
enchi install --dry-run --json 3.13
```

### Set versions

```powershell
enchi global 3.14.7
enchi local 3.12.10
enchi shell 3.12.10
```

### Manage named virtual environments

```powershell
enchi venv create 3.13 api
enchi venv list
enchi venv info api
enchi venv use api
enchi local 3.14.7/envs/api
enchi venv upgrade api 3.15
```

`venv upgrade` requires the target runtime to be installed first. It snapshots `pip freeze --all`, creates a replacement under the target runtime, and first restores the original requirement lines. If ordinary exact version pins are incompatible with the target Python, it recreates the staging environment and retries those pins as lower bounds (`>=`) so pip can choose a compatible release; direct URLs, VCS, editable, marker, and option lines remain unchanged. `pip check` is a hard gate. The original environment is not deleted if inventory, restoration, compatibility resolution, verification, or activation fails. With `--force`, an existing target venv is moved to a rollback path until the new environment is active. This command migrates Enchi managed venvs; it does not adopt an arbitrary standalone `.venv` directory.

Managed envs live under `ENCHI_ROOT/venvs/<runtime>/<name>`.
That gives you predictable names, avoids hidden project-specific duplication, and lets
`.python-version` point directly at a managed env spec such as `3.14.7/envs/api`.

Compatibility aliases from upstream pyenv workflows are also supported:

```powershell
enchi virtualenv 3.13 api
enchi virtualenvs
enchi virtualenv-prefix api
enchi activate api
enchi deactivate
```

Those aliases intentionally reuse the same managed-venv implementation, so the recommended
long-term workflow is still `enchi venv ...` plus `.python-version`.

### Inspect selection and resolution

```powershell
enchi version
enchi version --bare
enchi version-origin
enchi version-name
enchi which python
enchi whence python
enchi prefix
enchi versions
```

### Run commands through the selected runtime

```powershell
enchi exec python -V
enchi exec pip --version
```

### Refresh shims

```powershell
enchi rehash
enchi shims --short
```

### Remove runtimes

```powershell
enchi uninstall -f 3.12.10
```

---

## Version-selection behavior

Enchi follows the expected selection precedence:

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
enchi config show
```

### Get a single config key

```powershell
enchi config get storage.versions_dir
```

### Set a config value

```powershell
enchi config set venv.auto_create_base_venv true
enchi config set storage.versions_dir D:\PythonRuntimes
```

### Config keys

| Key | Default | Purpose |
| --- | --- | --- |
| `storage.versions_dir` | `<ENCHI_ROOT>/versions` | Managed runtimes directory |
| `storage.cache_dir` | `<ENCHI_ROOT>/cache` | Download and metadata cache |
| `windows.registry_mode` | `disabled` | On Windows, `pep514` writes HKCU PEP-514 keys under `Software\Python\PyenvNative` so IDEs can discover managed interpreters. Official `PythonCore` keys are not modified. No-op on Linux/macOS. |
| `plugins.search_path` | `true` | When true, plugin discovery also walks PATH (pyenv-compatible). Set `false` to limit to `ENCHI_ROOT/plugins` and `PYENV_PLUGIN_PATH`. |
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
enchi config set venv.auto_create_base_venv true
enchi install 3.12
```

### Managed env policy

Named managed envs are different from the optional companion base venv:

- companion base venvs are runtime-scoped internals,
- managed envs are user-facing named envs under `venvs/<runtime>/<name>`,
- managed envs can be written directly into `.python-version`,
- `enchi venv create` refuses ambiguous name collisions so a short name like `api` stays predictable.

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
enchi doctor
enchi doctor --json
enchi doctor --fix
```

`doctor` helps surface issues around:

- root detection,
- shim visibility,
- system Python visibility,
- Windows Store alias conflicts (Warn when the App Installer stub is ahead of enchi shims; Info when shims already take precedence — `doctor --fix` / GUI Self-Healing removes the `python.exe` / `python3.exe` stubs; turning the aliases off in Settings is optional),
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
enchi help
enchi help install
enchi commands
enchi hooks rehash
enchi completions install --family
```

---

## Development workflows

Repository automation is one cross-platform Rust tool, `cargo xtask` (see `xtask/`):

```sh
cargo xtask check                  # format, clippy, tests, version sync, policy checks, Python, a11y, audit
cargo xtask check --quick          # skip the slower GUI a11y and cargo-audit steps
cargo xtask version-check          # Cargo / PyPI (enchi and the legacy wrapper) / bootstrap / Tauri versions agree
cargo xtask set-version 0.7.0      # bump every manifest, then run `cargo build` to refresh Cargo.lock
cargo xtask bundle                 # build release binaries and write dist/enchi-<os>-<arch>.{tar.gz,zip} + .sha256,
                                   # plus the legacy dist/pyenv-native-<os>-<arch>.{tar.gz,zip} + .sha256
cargo xtask bundle --help          # --target, --platform, --arch, --cargo cross, --no-gui, --skip-build, --no-legacy-compat
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

### The legacy `pyenv-native-*` archives

Every release also publishes `pyenv-native-<platform>-<arch>.tar.gz|zip` (and `.sha256`) with the same contents as the `enchi-*` archives plus a copy of the CLI named `pyenv` (`pyenv.exe` on Windows). Already-installed pyenv-native binaries download exactly that asset name and run `<bundle>/pyenv self-install` from it, which is how an existing install updates into Enchi. `cargo xtask bundle` writes both by default (`--legacy-compat`, on by default; `--no-legacy-compat` or `--legacy-compat=false` skips them), and `XTASK_LEGACY_COMPAT=false` does the same for the release workflow. The bundle manifest of the legacy archive lists `"legacy_executable": "pyenv"`. New installs and the package formats use the `enchi-*` archives only.

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
`ENCHI_ROOT` and symlink/junction existing runtimes, so the installed enchi root
is not changed. The PowerShell script accepts `INSTALLED_ENCHI_ROOT` when its
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
cargo run -q -p enchi-mcp -- guide
cargo run -q -p enchi-mcp -- print-config
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
enchi root
enchi version-file [dir]
enchi version-file-read <file>
enchi version-file-write [-f|--force] <file> <version> [...]
enchi version-origin
enchi version-name [-f]
enchi version [--bare]
enchi global [--unset] [versions...]
enchi local [-f] [--unset] [versions...]
enchi shell [versions...]
enchi latest [-k|--known] [-b|--bypass] [-f|--force] <prefix>
enchi prefix [versions...]
enchi versions [--bare] [--skip-aliases] [--skip-envs] [--executables]
```

### Discovery, install, and execution

```text
enchi install --list [--known] [--family <family>] [--json] [pattern]
enchi available [--known] [--family <family>] [--json] [pattern]
enchi install [--dry-run] [--force] [--json] <version>...
enchi venv <list|info|create|delete|rename|use|upgrade> [options]
enchi uninstall [-f] <version>...
enchi which [--nosystem] [--skip-advice] <command>
enchi whence [--path] <command>
enchi exec <command> [args...]
enchi rehash
enchi shims [--short]
enchi init [-|--path] [--no-push-path] [--no-rehash] [pwsh|cmd|bash|zsh|fish|sh]
```

### Help, hooks, diagnostics, and configuration

```text
enchi help [--usage] [command]
enchi commands [--sh|--no-sh]
enchi hooks <hook|--complete>
enchi completions <command> [arg1 arg2...]
enchi doctor [--json] [--fix] [-f|--force]
enchi preflight [--json]
enchi environment [--json]
enchi status [--json]
enchi gui
enchi prompt
enchi self-update [--check] [--yes] [--force]
enchi self-uninstall [--yes]
enchi pip list|outdated|check|precheck|analyze|install|update ...
enchi config path|show|get|set
```

Query and apply pip upgrades:

```text
enchi pip outdated 3.14.7/envs/api --json
enchi pip update 3.14.7/envs/api certifi cryptography
enchi pip update 3.14.7/envs/api certifi==2026.7.22
enchi pip update --all 3.14.7/envs/api
```

`--all` fails if the outdated scan cannot run. Outdated `pip` is upgraded first.

### Python install package commands

```text
enchi-bootstrap verify <bundle-archive> [--checksum-path <bundle.sha256>]
enchi-bootstrap download [--bundle-url <url> | --release-base-url <url> | --github-repo <owner/repo>] [--tag <tag>]
enchi-bootstrap install [--bundle-path <bundle-archive> | --release-base-url <url> | --github-repo <owner/repo>] [--tag <tag>] [--install-root <dir>]
```

### MCP companion commands

```text
enchi-mcp
enchi-mcp guide
enchi-mcp print-config
```

Internal helper commands such as `sh-shell`, `sh-rehash`, and `sh-cmd` exist for shell integration, but they are intentionally not part of the normal end-user surface.

---

## Project structure

```text
README.md                       ../README.md
INSTRUCTIONS.md                 ./INSTRUCTIONS.md
ARCHITECTURE.md                 ./ARCHITECTURE.md
MCP.md                          ./MCP.md
install.ps1 / install.sh        thin bootstraps: download, verify, run `enchi self-install`
uninstall.ps1 / uninstall.sh    thin wrappers around `enchi self-uninstall`
xtask/                          `cargo xtask`: bundle, installers, release, check, doctor, gui, new-locale, install-hooks, agent-skills
crates/                         Rust CLI, core runtime, GUI, and MCP server
  enchi-cli/                    CLI dispatch and argument handling
  enchi-core/                   version resolution, install backends, shims, diagnostics
  enchi-gui/                    Tauri v2 desktop GUI companion
  enchi-mcp/                    stdio MCP server and agent toolkit
docs/
  screenshots/                  GUI screenshots for documentation
packaging/                      Homebrew notes
python-package/                 PyPI `enchi` package (pipx bootstrap, `enchi-bootstrap`)
python-package-legacy/          final PyPI `pyenv-native` release: depends on `enchi`, forwards to `enchi-bootstrap`
scripts/                        publish, packaging, validation, and smoke-test helpers
```

---

## Final notes

Enchi is trying to be respectful to upstream `pyenv` while also being unafraid to improve the experience where native implementation details genuinely matter.

That means the design bias is:

- familiar where users expect familiarity,
- better where users benefit from improvement,
- and always clear enough that installation, usage, and cleanup are never mysterious.
