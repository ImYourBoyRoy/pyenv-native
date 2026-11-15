# ROADMAP

Working name: **Enchi** (CLI `enchi`, MCP server `enchi-mcp`) — *Python Runtime & Environment Manager*.
The name is pending trademark, domain and non-English-meaning checks. Registry availability
(PyPI, crates.io, npm, Homebrew, local `PATH`) was clean on 2026-11-15; apt, Winget, Scoop, AUR,
nixpkgs and GitHub org availability were not checked.

This document is a plan, not a promise. Items move as we learn. Each phase lists what "done" means.

---

## Positioning

> One tool for Python versions and environments, with a CLI, GUI and agent interface.

| Pillar | What it means |
|---|---|
| Compatibility | `.python-version`, local/global/shell precedence, shims, hooks work like pyenv's |
| Speed | prebuilt interpreters by default, [uv](https://docs.astral.sh/uv/) acceleration where available |
| Only Enchi | fleet-wide upgrade, update, audit, snapshot and rollback across every runtime and venv |

### Relationship to competitors

- **pyenv**: we keep its mental model and add native Windows, prebuilt installs, structured output,
  a GUI and an agent surface.
- **uv**: we do **not** build a resolver, `add`/`lock`/`sync` or a lockfile format. uv owns the
  per-project dependency workflow. Enchi owns the machine-level view uv does not have (all runtimes,
  all venvs, bulk operations, audit across everything, snapshots) and **uses uv as an optional
  backend** for speed. Reading `uv.lock`/`pylock.toml` is in scope; writing them is not (except
  `pylock.toml` export from snapshots).
- Astral is being acquired by OpenAI. Enchi must never *require* uv or any single upstream. Every
  upstream (python-build-standalone, uv) is optional, pluggable, mirrorable and checksum-verified.

---

## Principles

1. Plan first, act on request. Anything destructive supports `--dry-run` and, over MCP, `apply: true`.
2. Fail closed on integrity. No unverified downloads, ever.
3. Every command has a stable `--json` form. The CLI, GUI and MCP call the same core functions.
4. Nothing is required that the user did not opt into (uv, network telemetry, plugins).
5. Existing installs keep working. No migration may move a user's runtimes or venvs.

---

## Phase 0 — Quick GUI pass (before backend work) — done

Scope: only what makes the redesign shippable and brand-ready. No new backend features.

- ✅ Commit the `gui-redesign` work on its own branch.
- ✅ Brand slot: product name in `ui/brand.js` (`{ $brand }` in every locale), logo in `ui/brand/mark.svg`,
  accent in `tokens.css`. See [GUI.md](GUI.md#branding). Window icons, the Tauri bundle id and repo links stay manual.
- ✅ Translated the 34 English placeholder strings plus 16 missing Activity/toast strings into all 13
  locales. These are machine translations and have not had a native-speaker review.
- ✅ Activity tray wired into install, uninstall, venv create/delete/upgrade, pip upgrade and requirements installs.
  These calls show elapsed time and an outcome but no step progress; the backend only reports steps for
  the newer upgrade and bulk-update commands. Moves to Phase 7.

**Done when:** `cargo xtask check` and `cargo xtask policy` pass, screenshots regenerated, GUI smoke
tour (`PYENV_GUI_SMOKE=1`) clean.

## Phase 1 — Rename to Enchi and bridge release

Mechanical, high-risk-if-sloppy. Done first so every later feature is written under the final name.
See [Rename and migration](#rename-and-migration) below.

**Done when:** a real `pyenv-native` 0.7.x install on each OS updates itself to Enchi through the
normal `pyenv self-update`, keeps all runtimes and venvs, and `pyenv` still resolves.

## Phase 2 — Prebuilt interpreters

- New install provider for [python-build-standalone](https://github.com/astral-sh/python-build-standalone)
  (PBS): consume its published release archives and checksums over HTTP. It is a set of artifacts,
  not a library; we do not link or vendor it.
- Setting `install.source`:
  - `auto` (default): prebuilt when a *matching* artifact exists and verifies, otherwise compile.
  - `prebuilt`: prebuilt only; fail with a clear message if none matches.
  - `source`: always compile via `python-build` (today's behavior).
  - Per-command override `--source <auto|prebuilt|source>` (alias `--build` for `source`), env var,
    config key, and a GUI select.
- Matching: OS, CPU architecture, libc (glibc vs musl), free-threaded flag, and microarchitecture
  variants where PBS publishes them. Never pick an artifact that cannot run on the host.
- Prefer the `install_only`/`install_only_stripped` archives. Fully static musl builds cannot load
  extension modules, so they are not used for general installs.
- Integrity: verify against the release checksum file; pin the expected hash in the install record.
  Mirrors via an env override (same pattern as `PYENV_NATIVE_RELEASE_BASE_URL`).
- Offline-safe: ship a metadata snapshot with the binary and merge live data into the cache, like
  `known_versions.txt` does today. Cache negative results and retry with backoff.
- Windows keeps NuGet unless PBS proves strictly better there (arm64, free-threaded).

**Done when:** `enchi install 3.14` completes in seconds on Linux and macOS, `source` still works,
the chosen provider is recorded in the version's metadata and shown by `enchi versions --json`.

## Phase 3 — Optional uv backend

- Detection: `uv` on `PATH` or in Enchi's tools directory; version recorded.
- `enchi uv install` downloads a checksum-verified uv release (no `curl | sh`). GUI offers
  "Install uv for faster installs and extended features".
- Setting `backend.uv`: `auto` (use if present), `off`, `require`.
- Used for: package installs, bulk updates, venv creation, resolution checks. pip is the fallback.
- Guardrails: always invoke uv with an explicit interpreter and `UV_PYTHON_DOWNLOADS=never`, so uv
  never fetches or manages a Python behind Enchi's back.
- Interop: list uv-managed Pythons and `.venv` directories in inventory (read-only) and let a user
  adopt them.

**Done when:** `enchi pip update-all` runs through uv when present, produces identical results to the
pip path on the e2e suite, and everything still works with uv absent.

## Phase 4 — Project awareness

Enchi understands a project without becoming a project manager.

- Read: `pyproject.toml` (`requires-python`, `[tool.enchi]`), `.python-version`, `uv.lock`,
  `pylock.toml`, `requirements*.txt`, and detect poetry/pdm/hatch/pipenv by their files.
- `enchi project status`: required vs active runtime, venv present, lock drift, outdated count,
  audit result, one verdict.
- `enchi project sync`: ensure the right runtime and a managed venv, then install from the lock via
  the best backend. It never rewrites the user's lock.
- `enchi run <cmd>`: resolve the runtime and venv, then exec. Delegates to uv when configured.
- MCP: extend `resolve_project_environment` to return the same verdict; add `project_sync` (plan-first).

**Done when:** pointing Enchi at a fresh clone of a uv, poetry and plain-requirements project each
yields a correct status and a working `sync`.

## Phase 5 — Fleet audit and health

- `enchi audit`: check every installed package in every venv and runtime against the OSV database.
  Offline-friendly cache, `--json`, severity filter, saved ignores. Unlike `uv audit`, which reads one
  lockfile, this covers the whole machine.
- `enchi doctor` additions: end-of-life CPython versions, runtimes no venv uses, orphaned or broken
  venvs, shim drift, disk usage, stale caches.
- `enchi prune`: plan-first removal of unused runtimes, orphaned venvs and caches.
- Reuse the existing bulk-update rollback model for any fix action.

**Done when:** one command answers "is anything on this machine vulnerable, end-of-life or broken?"
and can fix what is safe to fix, with rollback.

## Phase 6 — Snapshots and reproducibility

- `enchi snapshot create [name]`: record runtime identity (version, provider, build tag, artifact
  hash, platform) and the full package set with hashes for each venv. Interchange format is
  PEP 751 `pylock.toml` plus a small Enchi manifest for the runtime.
- `enchi snapshot restore <name>`: install the runtime if missing, recreate the venv, install the
  exact hashed set, verify. Built in a temporary directory and swapped in atomically.
- Automatic snapshot before `upgrade`, `pip update-all`, and `prune`; `enchi rollback` generalizes
  the rollback that bulk update already has.
- `enchi export --format pylock|requirements` and a minimum package age option (like uv's
  `--exclude-newer`) for reproducible updates.
- Honest limit: this is Python-only. Unlike conda it does not capture system libraries or non-Python
  binaries; snapshots record the platform and warn on restore elsewhere. Packages from VCS or local
  paths are recorded without hashes and flagged.

**Done when:** snapshot on machine A, restore on machine B (same OS/arch) yields byte-identical
package sets, and a failed bulk update can be undone with one command.

## Phase 6b — Root migration (`~/.pyenv` → `~/.enchi`)

Opt-in, never automatic. The bridge release (Phase 1) moves nothing: existing installs keep their
root and Enchi records where it is. This phase adds a proper way to move, built on snapshots
(Phase 6), prebuilt installs (Phase 2) and the uv backend (Phase 3), which is why it comes after them.

Why venvs cannot simply be moved: `pyvenv.cfg`, script shebangs, activation scripts, editable-install
`.pth` files and console-script launchers all contain absolute paths. A venv is rebuilt, not moved.

`enchi migrate` (plan by default; `--apply` to act; `--json` throughout):

1. **Inventory and plan.** Classify everything under the old root and say *why*:
   - runtimes: **move** (atomic rename on the same filesystem, copy and verify across filesystems) when
     relocatable; **reinstall** when not (source builds bake the prefix into build data), using the
     provider the user picks;
   - venvs: **rebuild**; packages come from an automatic snapshot of each one;
   - config, hooks, plugins, version file, caches, logs: **move** (caches may be dropped);
   - anything Enchi did not create (a classic pyenv checkout, foreign plugins): **leave and warn**.
   Also report disk space needed, estimated time, and anything that cannot be reproduced (editable or
   VCS installs, unhashed packages).
2. **Safety net first.** Snapshot every environment into the new root, refuse to start if there is not
   enough space, and journal every step so a crash resumes instead of corrupting.
3. **Move runtimes, rebuild venvs.** Each rebuilt venv is verified (same package set, imports, `pip
   check`) before it replaces anything. A venv that fails stays in the old root and is reported; it
   never blocks the others.
4. **Rewire.** Rewrite the managed shell-profile block and the `/etc/profile.d` snippet, regenerate
   shims, update the recorded root. Lines the user wrote by hand (for example their own
   `export PYENV_ROOT=...`) are reported, not edited.
5. **Compatibility link.** Leave `~/.pyenv` as a symlink (junction on Windows) to the new root, so
   IDE settings, cron jobs, systemd units, Dockerfiles and CI that hard-code old interpreter paths
   keep working. Opt out with `--no-compat-link`.
6. **Keep the old root until told.** Nothing is deleted. `enchi migrate --rollback` restores the
   profile and recorded root; `enchi migrate --cleanup` removes the backup once the user is happy.

Honest limits: migration is cosmetic plus consistency, so it is never pushed, only offered (a hint in
`enchi doctor` and a GUI banner). Running shells and processes that hold old paths must be restarted.
Project-local `.venv` directories the user made themselves are not Enchi's and are not touched.

**Done when:** a real 0.7.x install with several runtimes (including one source-built) and venvs
migrates on Linux, macOS and Windows; every venv imports its packages from the new location; a forced
mid-run failure resumes cleanly; rollback restores the original state.

## Phase 7 — Backend polish and GUI association

- `--json` and `--dry-run` on every older command; consistent `--force` semantics. **Mostly done:**
  the scriptable commands accept `--json` (uniform `enchi.result/1` envelope, no prompts in machine
  mode), and `uninstall`, `venv create|delete|rename|upgrade` and `pip install|update` have
  `--dry-run`. Still open: `--dry-run` for `self-uninstall`, and a `--json` form for `self-install`.
- Catalog robustness: cache negative results, retry with backoff, clear UX when a publisher omits a
  checksum.
- MCP tools for every new capability, plan-first, annotated, with progress events.
- Then, and only then, build the GUI pages for audit, snapshots, project status, prebuilt/source
  setting and uv install, calling the same core functions.
- Windows and macOS verification of everything above (currently only Linux is exercised end to end).

## Later / undecided

- **Tool runner** (`enchi tool run ruff`): isolated CLI apps, like pipx/uvx. Thin wrapper that
  delegates to uv when present, otherwise an Enchi-managed venv per tool. Enchi will not ship its own
  linter or formatter; at most it detects a configured tool and offers to run it.
- Mirror/offline bundles for air-gapped machines.
- Shared package cache and hardlinked venv creation (mostly comes free with the uv backend).

---

## Rename and migration

How existing users receive Enchi, based on how the updater works today
(`crates/enchi-core/src/self_update`):

1. The updater calls `GET /repos/imyourboyroy/pyenv-native/releases/latest`, compares the tag to its
   own version, downloads `pyenv-native-<platform>-<arch>.tar.gz|zip` plus its `.sha256` sidecar,
   extracts it, and runs the bundle's **`pyenv self-install`**.
2. So the git branch is irrelevant to old installs. **Releases are what move users.**

Consequences for the bridge release:

- **Version numbers must keep increasing** (for example 0.8.0 or 1.0.0). A reset to 0.1.0 would look
  older to every old updater and nobody would be offered the update.
- **Publish the old asset names** (`pyenv-native-<platform>-<arch>.*` with `.sha256`) on the bridge
  release, containing a `pyenv` executable the old updater can run. Also publish the new
  `enchi-<platform>-<arch>.*` assets; the new updater uses those.
- The bundled `pyenv self-install` performs the migration: install `enchi`/`enchi-mcp`, keep `pyenv`
  and `enchi-mcp` as aliases, rewrite the managed shell-profile block to call `enchi init`, update the
  Windows uninstall-registry entry and desktop shortcut, and register the MCP server under the new name
  while leaving the old one working.
- **The bridge release does not move the root directory.** Existing installs keep `~/.pyenv` (venvs
  contain absolute paths). New installs default to `~/.enchi`. Enchi reads `ENCHI_ROOT`, then
  `PYENV_ROOT`, then the recorded location. Moving is a separate, opt-in `enchi migrate`
  (Phase 6b) that rebuilds venvs properly.
- Rename the GitHub repository only after the bridge release is out, and verify in a throwaway repo
  that the API and download redirects work for old updaters. Never create a new repo under the old
  name, which would break the redirect.
- Other channels: publish a final `pyenv-native` PyPI release that depends on `enchi`; ship `.deb`/
  `.rpm` packages named `enchi` with `Replaces`/`Conflicts`/`Provides: pyenv-native`; macOS `.pkg`
  installs over the old version. Re-check which other channels (Homebrew, Winget, Scoop) exist.

Branching: cut an `enchi` branch from the committed `gui-redesign` work, do the rename there, and
when the bridge release is validated merge it into `main` (merge, not replace, so history survives).
Tag the last pre-rename commit and keep a `legacy/0.7` branch for security fixes.

Test matrix before shipping the bridge: fresh 0.7.2 install → `self-update` on Linux, macOS and
Windows; with and without `pyenv` shims in use; with a venv and a global version set; via PyPI,
shell installer, `.deb`, and macOS `.pkg`.

---

## Open questions

- Final name: trademark, domain, and non-English-meaning checks for Enchi.
- Whether to switch Windows installs to PBS as well as, or instead of, NuGet.
- Where snapshots live by default (`$ROOT/snapshots` vs. committed to the project).
- Policy if PBS or uv change license, ownership or release cadence (mirror and pin strategy).
