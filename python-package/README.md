# pyenv-native

![PyPI Channel](https://img.shields.io/badge/channel-PyPI%20%2F%20pipx-3775A9?style=for-the-badge&logo=pypi&logoColor=white)
![Runtime](https://img.shields.io/badge/runtime-install%20package%20for%20native%20bundles-2563eb?style=for-the-badge)
![License](https://img.shields.io/badge/license-MIT-15803D?style=for-the-badge)

`pyenv-native` on PyPI is the Python convenience package for installing the native `pyenv-native` release bundles.

Those bundles install both:
- `pyenv`
- `pyenv-mcp`

It exists for users who:
- already have Python installed,
- want a `pip` or `pipx` entrypoint,
- still want the real runtime to remain native.

## Important principle

> This package installs `pyenv-native`.
> It does **not** replace `pyenv-native` with a Python implementation.

## Release selection behavior

By default, the package targets the **latest published GitHub release**.

## What it does

- downloads native release bundles,
- verifies checksums,
- reads bundle metadata,
- extracts the bundle,
- runs the bundled installer,
- installs the companion `pyenv-mcp` server when the bundle provides it,
- supports GitHub Release-based installs,
- works with Windows ZIP bundles and Linux/macOS `.tar.gz` bundles.

## Quick start

### `pipx` latest release

```powershell
pipx install pyenv-native
pyenv-native install --github-repo imyourboyroy/pyenv-native --install-root ~\.pyenv
```

### `pip` latest release

```powershell
python -m pip install pyenv-native
pyenv-native install --github-repo imyourboyroy/pyenv-native --install-root ~\.pyenv
```

### POSIX latest release

```sh
python -m pip install pyenv-native
pyenv-native install --github-repo imyourboyroy/pyenv-native --install-root ~/.pyenv
```

## Commands

```text
pyenv-native verify <bundle-archive> [--checksum-path <bundle.sha256>]
pyenv-native download [--bundle-url <url> | --release-base-url <url> | --github-repo <owner/repo>] [--tag <tag>]
pyenv-native install [--bundle-path <bundle-archive> | --release-base-url <url> | --github-repo <owner/repo>] [--tag <tag>] [--install-root <dir>]
```

## Examples

Verify a local bundle:

```powershell
pyenv-native verify .\dist\pyenv-native-windows-x64.zip --checksum-path .\dist\pyenv-native-windows-x64.zip.sha256
```

Install from a local bundle:

```powershell
pyenv-native install --bundle-path .\dist\pyenv-native-windows-x64.zip --checksum-path .\dist\pyenv-native-windows-x64.zip.sha256 --install-root ~\.pyenv
```

Install from the latest GitHub release:

```powershell
pyenv-native install --github-repo imyourboyroy/pyenv-native --install-root ~\.pyenv
```

Install a POSIX bundle:

```sh
pyenv-native install --bundle-path ./dist/pyenv-native-linux-x64.tar.gz --checksum-path ./dist/pyenv-native-linux-x64.tar.gz.sha256 --install-root ~/.pyenv --shell bash
```

## Relationship to the main project

For the full project overview, install scripts, release-bundle flow, and command reference, see the main repository README:

- <https://github.com/imyourboyroy/pyenv-native>
