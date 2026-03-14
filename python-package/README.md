# pyenv-native-bootstrap

![PyPI Channel](https://img.shields.io/badge/channel-PyPI%20%2F%20pipx-3775A9?style=for-the-badge&logo=pypi&logoColor=white)
![Runtime](https://img.shields.io/badge/runtime-bootstrap%20for%20native%20bundles-2563eb?style=for-the-badge)
![License](https://img.shields.io/badge/license-MIT-15803D?style=for-the-badge)

`pyenv-native-bootstrap` is the Python convenience package for installing the native `pyenv-native` release bundles.

It exists for users who:
- already have Python installed,
- want a `pip` or `pipx` entrypoint,
- still want the real runtime to remain native.

## Important principle

> This package bootstraps `pyenv-native`.
> It does **not** replace `pyenv-native` with a Python implementation.

## Release selection behavior

By default, the bootstrap package targets the **latest published GitHub release**.
Use `--tag <vX.Y.Z>` only when you want to pin a specific published release.

## What it does

- downloads native release bundles,
- verifies checksums,
- reads bundle metadata,
- extracts the bundle,
- runs the bundled installer,
- supports GitHub Release-based installs,
- works with Windows ZIP bundles and Linux/macOS `.tar.gz` bundles.

## Quick start

### `pipx` latest release

```powershell
pipx install pyenv-native-bootstrap
pyenv-native-bootstrap install --github-repo imyourboyroy/pyenv-native --install-root ~\.pyenv
```

### `pip` latest release

```powershell
python -m pip install pyenv-native-bootstrap
pyenv-native-bootstrap install --github-repo imyourboyroy/pyenv-native --install-root ~\.pyenv
```

### POSIX latest release

```sh
python -m pip install pyenv-native-bootstrap
pyenv-native-bootstrap install --github-repo imyourboyroy/pyenv-native --install-root ~/.pyenv
```

### Pinned release

```powershell
pyenv-native-bootstrap install --github-repo imyourboyroy/pyenv-native --tag vX.Y.Z --install-root ~\.pyenv
```

## Commands

```text
pyenv-native-bootstrap verify <bundle-archive> [--checksum-path <bundle.sha256>]
pyenv-native-bootstrap download [--bundle-url <url> | --release-base-url <url> | --github-repo <owner/repo>] [--tag <tag>]
pyenv-native-bootstrap install [--bundle-path <bundle-archive> | --release-base-url <url> | --github-repo <owner/repo>] [--tag <tag>] [--install-root <dir>]
```

## Examples

Verify a local bundle:

```powershell
pyenv-native-bootstrap verify .\dist\pyenv-native-windows-x64.zip --checksum-path .\dist\pyenv-native-windows-x64.zip.sha256
```

Install from a local bundle:

```powershell
pyenv-native-bootstrap install --bundle-path .\dist\pyenv-native-windows-x64.zip --checksum-path .\dist\pyenv-native-windows-x64.zip.sha256 --install-root ~\.pyenv
```

Install from the latest GitHub release:

```powershell
pyenv-native-bootstrap install --github-repo imyourboyroy/pyenv-native --install-root ~\.pyenv
```

Install from a pinned GitHub release:

```powershell
pyenv-native-bootstrap install --github-repo imyourboyroy/pyenv-native --tag vX.Y.Z --install-root ~\.pyenv
```

Install a POSIX bundle:

```sh
pyenv-native-bootstrap install --bundle-path ./dist/pyenv-native-linux-x64.tar.gz --checksum-path ./dist/pyenv-native-linux-x64.tar.gz.sha256 --install-root ~/.pyenv --shell bash
```

## Relationship to the main project

For the full project overview, install scripts, release-bundle flow, and command reference, see the main repository README:

- <https://github.com/imyourboyroy/pyenv-native>