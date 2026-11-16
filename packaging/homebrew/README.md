# Homebrew packaging

This folder contains generator-backed Homebrew packaging assets for Enchi.

## Current scope

- Homebrew support is **prepared**, not yet publicly published.
- The generator consumes Linux/macOS release checksum files for the `enchi-<os>-<arch>.tar.gz` assets (the legacy `pyenv-native-*` copies are ignored) and emits a formula named `enchi` (class `Enchi`) that installs `enchi` and `enchi-mcp`.
- Public tap publishing can happen later without changing the release bundle format.

## Generate a formula locally

After you have Linux/macOS release assets and their `.sha256` files available, run:

```sh
cargo xtask package homebrew --tag vX.Y.Z
```

`--github-repo` defaults to the repository that publishes releases (`imyourboyroy/pyenv-native` until it is renamed).

Default asset scan roots:

- `dist/`
- `dist/linux/`
- `dist/macos/`

Override them with repeated `--asset-root <dir>` flags.

## Output

The generated formula is written to:

```text
packaging/homebrew/Formula/enchi.rb
```

## Suggested later publish flow

1. Produce or download the release assets for Linux/macOS.
2. Generate the formula with `cargo xtask package homebrew`.
3. Commit or copy the generated formula into your Homebrew tap repository.
4. Open the tap PR / push the tap update.

## Notes

- The formula intentionally targets the portable binary bundles instead of rebuilding the Rust binary inside Homebrew.
- The release bundles remain the single source of truth for end-user installation assets.
