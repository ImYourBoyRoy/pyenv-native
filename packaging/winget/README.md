# pyenv-native Winget packaging

This folder holds generated or generator-backed Winget manifests for the Windows portable bundle.

## Generate manifests

```sh
cargo xtask package winget
```

Common overrides:

```sh
cargo xtask package winget \
  --github-repo imyourboyroy/pyenv-native \
  --tag vX.Y.Z \
  --manifest-version 1.12.0 \
  --validate
```

Pass `--bundle <zip>` once per architecture (with matching `--checksum <file>`) to emit a
multi-architecture installer manifest.

## Optional validation

If `winget` is available locally, the command can run a strict manifest validation pass:

```sh
cargo xtask package winget --validate
```

## Output layout

The command writes manifests under:

```text
packaging/winget/manifests/<first-letter>/<publisher>/<package>/<version>/
```

For the default package identifier:

```text
packaging/winget/manifests/i/ImYourBoyRoy/pyenv-native/<version>/
```
