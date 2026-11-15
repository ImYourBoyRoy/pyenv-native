# enchi (Python install package)

![PyPI Channel](https://img.shields.io/badge/channel-PyPI%20%2F%20pipx-3775A9?style=for-the-badge&logo=pypi&logoColor=white)
![Runtime](https://img.shields.io/badge/runtime-install%20package%20for%20native%20bundles-2563eb?style=for-the-badge)
![License](https://img.shields.io/badge/license-MIT-15803D?style=for-the-badge)

`enchi` on PyPI is the Python convenience package for installing the native Enchi release bundles.
Enchi was called `pyenv-native` before; the `pyenv-native` PyPI package is now a thin wrapper that
depends on this one.

Those bundles install:
- `enchi` (the CLI)
- `enchi-mcp` (the MCP server for coding agents)
- `enchi-gui` (desktop builds)

It exists for users who:
- already have Python installed,
- want a `pip` or `pipx` entrypoint,
- still want the real runtime to remain native.

The console script this package adds is **`enchi-bootstrap`**, not `enchi`, so it never shadows the
real `enchi` binary the bundle installs.

## Important principle

> This package installs Enchi.
> It does **not** replace Enchi with a Python implementation.

## Release selection behavior

By default, the package targets the **latest published GitHub release**.

## What it does

- downloads native release bundles (`enchi-<platform>-<arch>`),
- verifies checksums,
- reads bundle metadata,
- extracts the bundle,
- runs the bundled installer (`enchi self-install`),
- installs the companion `enchi-mcp` server when the bundle provides it,
- supports GitHub Release-based installs,
- works with Windows ZIP bundles and Linux/macOS `.tar.gz` bundles.

Where the install goes is decided by `enchi self-install`: a fresh install uses `~/.enchi`, and an
existing pyenv-native install keeps its `~/.pyenv` root. Pass `--install-root` to choose one.

## Quick start

### `pipx` latest release

```powershell
pipx install enchi
enchi-bootstrap install --github-repo imyourboyroy/pyenv-native
```

### `pip` latest release

```powershell
python -m pip install enchi
enchi-bootstrap install --github-repo imyourboyroy/pyenv-native
```

### POSIX latest release

```sh
python -m pip install enchi
enchi-bootstrap install --github-repo imyourboyroy/pyenv-native
```

(`imyourboyroy/pyenv-native` is the GitHub repository that publishes the releases; it keeps that
name until the repository itself is renamed.)

## Commands

```text
enchi-bootstrap verify <bundle-archive> [--checksum-path <bundle.sha256>]
enchi-bootstrap download [--bundle-url <url> | --release-base-url <url> | --github-repo <owner/repo>] [--tag <tag>]
enchi-bootstrap install [--bundle-path <bundle-archive> | --release-base-url <url> | --github-repo <owner/repo>] [--tag <tag>] [--install-root <dir>]
```

`python -m enchi_bootstrap` runs the same commands.

Environment variables: `ENCHI_BOOTSTRAP_RELEASE_BASE_URL` and `ENCHI_BOOTSTRAP_GITHUB_REPO` supply
the download source. The earlier `PYENV_NATIVE_BOOTSTRAP_*` names are still read as a fallback.

## Examples

Verify a local bundle:

```powershell
enchi-bootstrap verify .\dist\enchi-windows-x64.zip --checksum-path .\dist\enchi-windows-x64.zip.sha256
```

Install from a local bundle:

```powershell
enchi-bootstrap install --bundle-path .\dist\enchi-windows-x64.zip --checksum-path .\dist\enchi-windows-x64.zip.sha256
```

Install from the latest GitHub release into a chosen root:

```powershell
enchi-bootstrap install --github-repo imyourboyroy/pyenv-native --install-root ~\.enchi
```

Install a POSIX bundle:

```sh
enchi-bootstrap install --bundle-path ./dist/enchi-linux-x64.tar.gz --checksum-path ./dist/enchi-linux-x64.tar.gz.sha256 --shell bash
```

Releases published before the rename only carry `pyenv-native-<platform>-<arch>` bundles. When a
release has no `enchi-…` bundle, the GitHub download falls back to that name.

## Relationship to the main project

For the full project overview, install scripts, release-bundle flow, and command reference, see the main repository README:

- <https://github.com/imyourboyroy/pyenv-native>
