# Migrating from pyenv-native to Enchi

pyenv-native is now **Enchi**. The tool, its behaviour, and its pyenv compatibility are the same; the names changed.

| Before | Now |
| --- | --- |
| `pyenv` (CLI) | `enchi` (`pyenv` stays as an alias) |
| `pyenv-mcp` | `enchi-mcp` (`pyenv-mcp` stays as an alias) |
| `pyenv-gui`, `pyenv gui` | `enchi-gui`, `enchi gui` |
| `PYENV_ROOT` | `ENCHI_ROOT` (`PYENV_ROOT` is still honoured) |
| `PYENV_NATIVE_RELEASE_BASE_URL` | `ENCHI_RELEASE_BASE_URL` (the old name is still read) |
| `~/.pyenv` for new installs | `~/.enchi` for **fresh** installs only |

The GitHub repository is still `imyourboyroy/pyenv-native` for now; its URLs, and the release download URLs the install scripts use, do not change.

## If you already have pyenv-native installed

Update the way you always did. You end up with `enchi` and keep working.

- **Script, Windows setup, or portable installs:** run `pyenv self-update` (or re-run `install.sh` / `install.ps1`). The old binary downloads the release asset it has always used, `pyenv-native-<platform>-<arch>`, and runs the `pyenv` binary inside it. Every release therefore also publishes those legacy archives: same contents as the `enchi-<platform>-<arch>` archives plus a `pyenv` copy of the CLI. The upgrade installs `enchi`, `enchi-mcp` (and `enchi-gui` where it ships) and keeps `pyenv` / `pyenv-mcp` / `pyenv-gui` as aliases in the install's `bin/` directory.
- **Package managers:** the package is now called `enchi` and replaces `pyenv-native` (see the table below), so the normal upgrade command swaps one for the other.
- **Your root stays where it is.** An existing `~/.pyenv` (Windows `%USERPROFILE%\.pyenv`) keeps being used: installed Pythons, managed venvs, shims, and config are untouched, and nothing is moved automatically. A later, opt-in `enchi migrate` that moves an existing root to `~/.enchi` is **planned** and does not exist yet. You never have to move it; only fresh installs default to `~/.enchi`.
- **Environment variables.** Set `ENCHI_ROOT` to choose a root. `PYENV_ROOT` is still accepted as a fallback (classic-pyenv compatibility). `PYENV_VERSION`, `PYENV_SHELL`, `PYENV_DIR`, `PYENV_LANG`, and the other `PYENV_*` variables keep their names. Set `ENCHI_RELEASE_BASE_URL` for mirrors and offline tests; `PYENV_NATIVE_RELEASE_BASE_URL` still works.
- **MCP configs** that launch `pyenv-mcp` keep working. To move to the new name, run `enchi-mcp print-config` and replace the old entry (the server is called `enchi`). The tools are the same.
- **Agent skills:** run `enchi agent-skills --agent all`. The skill is now named `enchi`; the Claude Code plugin is `enchi@enchi-skills` (`/plugin install enchi@enchi-skills`).
- **Scripts and CI** that call `pyenv ...` keep working through the alias. Prefer `enchi ...` in new code.

## If you came from classic (bash) pyenv

- Classic plugins (`pyenv-virtualenv`, `pyenv-update`, `pyenv-doctor`, `python-build`) are not needed: Enchi has `venv`, `self-update`, `doctor` and a native installer. They are harmless to leave in `$PYENV_ROOT/plugins`. A classic `.bash` hook may call `before_install`, `after_install`, `before_uninstall` or `after_uninstall`; Enchi accepts those calls and ignores them (it installs, uninstalls and removes dependent venvs itself), so such a hook no longer aborts the command. Any other failure in a hook still does.
- Classic `pyenv-virtualenv` environments under `versions/<runtime>/envs/<name>` are not managed by Enchi. To adopt one, copy it to `venvs/<runtime>/<name>` and run `enchi venv upgrade <runtime>/envs/<name> <runtime>` (it rebuilds the environment from a package snapshot and only replaces it if `pip check` passes).

## What changed for each distribution

| Channel | Before | Now | Notes |
| --- | --- | --- | --- |
| Release archives | `pyenv-native-<platform>-<arch>.tar.gz\|zip` | `enchi-<platform>-<arch>.tar.gz\|zip` | The legacy names are still published (with a `pyenv` copy of the CLI) so existing installs can update; they may be dropped in a later release. |
| Debian / Ubuntu / Termux | `pyenv-native_<version>_<arch>.deb` | `enchi_<version>_<arch>.deb` | `Replaces`, `Provides`, and `Conflicts: pyenv-native`; also `Conflicts: pyenv` (the unrelated classic package). `/usr/bin/pyenv` and `/usr/bin/pyenv-mcp` are symlinks. The install marker is `share/enchi/install-source`. |
| Fedora / RHEL / openSUSE | `pyenv-native-<version>-1.<arch>.rpm` | `enchi-<version>-1.<arch>.rpm` | `Obsoletes` and `Provides: pyenv-native`, so `dnf` / `zypper` upgrades replace the old package. Same `pyenv` symlinks. |
| macOS | `pyenv-native-macos-<arch>.pkg` / `.dmg`, `Pyenv Native.app` | `enchi-macos-<arch>.pkg` / `.dmg`, `Enchi.app` | New receipt identifier `com.imyourboyroy.enchi.pkg`; `pyenv` / `pyenv-mcp` are links in `/usr/local/bin`. Opening it over a pyenv-native package upgrades in place. A pyenv-native 0.7.x package install cannot update itself and tells you to download `pyenv-native-macos-<arch>.pkg`, so each release also publishes that exact name: the same package, byte for byte. `sudo enchi self-uninstall --system` removes the old package's receipt and `Pyenv Native.app` along with Enchi's. |
| Windows | `pyenv-native-windows-<arch>-setup.exe` | `enchi-windows-<arch>-setup.exe` | Upgrades an existing install in place (it finds the root from Apps & features, `ENCHI_ROOT` / `PYENV_ROOT`, or an existing root directory). |
| Linux desktop entry / Tauri id | `com.pyenv-native.gui` | `com.imyourboyroy.enchi` | |
| PyPI | `pyenv-native` (script `pyenv-native`) | `enchi` (script `enchi-bootstrap`) | The script is not called `enchi`, so it never shadows the real binary. `pyenv-native` becomes a final thin release that depends on `enchi` and forwards to `enchi-bootstrap` with a one-line notice. |
| Homebrew | formula `pyenv-native` (generated, no public tap) | formula `enchi` (generated, no public tap) | Installs `enchi` and `enchi-mcp`. |
| SBOM | `pyenv-native.cdx.json` | `enchi.cdx.json` | |

## What did not change

- Version selection: shell, then local `.python-version`, then parent directories, then global, then system.
- `.python-version` files, shims, managed venv layout, hook directories, and the `PYENV_*` variables the shell integration uses.
- Compatibility with classic pyenv workflows, and the rule that CPython catalogs come live from python.org.
- Checksums (`.sha256` for every asset, `SHA256SUMS` for the release) and build-provenance attestations.

## Troubleshooting

- `enchi: command not found` right after an update: open a new terminal so the shell picks up the new shims, or run `pyenv doctor` (the alias still works).
- Two roots, `~/.pyenv` and `~/.enchi`: set `ENCHI_ROOT` to the one you want and run `enchi doctor`.
