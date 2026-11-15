# Enchi GUI Companion

The **Enchi GUI** is a desktop dashboard built with Tauri v2 for managing Python environments visually.

> [!IMPORTANT]
> **Status:** Ships on **Windows x64**, **Linux x64**, and **macOS** (arm64/x64) native release bundles. Cross-compiled bundles (Windows ARM64, Linux ARM64, Android) do **not** include the GUI.

## Features

- **Dashboard**: Live view of your active Python version, managed venvs, and install root, including a status light for pending pip updates. Doctor warnings appear as a click-through banner.
- **Where does this command come from?** (Dashboard): type `python`, `pip`, `pytest`, … to see the executable Enchi resolves for the project folder, with a copy button and the other runtimes that also provide it (`enchi which` / `enchi whence`). A miss shows the core's explanation.
- **Project folder**: The folder bar stays visible on every page. **Make Local** writes `.python-version` there; **Pin to another folder…** in a card’s **More** menu opens a picker without changing the bar.
- **Pip Package Explorer**: Drawer focused on a target interpreter to browse installed dependencies, run **Check dependencies** (`pip check`), audit updates, statically pre-check requirements.txt constraints, and scan codebase imports.
- **Codebase Import Analyzer**: Statically parses Python source in the active workspace, flags missing third-party imports, and can install them progressively.
- **Runtime upgrades**: **Check for upgrades** on Installed Runtimes lists the newest patch for each installed CPython. One button installs them, moves the managed venvs onto the new patch, and updates your pins; old runtimes are kept.
- **Update all packages**: on Virtual Environments, plans package updates across every runtime and venv, then applies them with `pip check` rollback per environment. Progress shows in the Activity tray.
- **Pip Updates**: Audits packages against PyPI with a multiselect checklist. The gold update button stays dimmed until at least one package is selected. Outdated `pip` is updated first.
- **Conflict Pre-checker**: Resolves local files or HTTPS URLs (GitHub blob links translated to raw) before `pip install`.
- **Visual Management**: Browse and install CPython 2.x / 3.x and PyPy. Latest patch per line is the default; Python 2 stays in the catalog for legacy projects. Alpha, beta, and release-candidate builds remain visible with a channel badge instead of being presented as stable releases.
- **Install locations**: every managed runtime on Installed Versions shows where it lives (`enchi prefix`) with a **Copy** button; **More > Open folder** shows it in the file manager.
- **Venv Manager**: Create, list, rename, delete, and migrate named virtual environments onto another installed runtime. Cards keep **Package Explorer** plus a **More** menu (global/local, rename, details, open folder, migrate, delete). **Rename…** opens an inline field that validates the name (letters, numbers, `.`, `_`, `-`) and previews the change with the core's `venv rename --dry-run` before you apply it; the stale entry-point warning the core returns is shown after a rename, and renaming the global default reminds you to point it at the new name. **Details** lists the location, Python, and pip paths with copy buttons. Choose an installed target such as Python 3.15.0 to migrate a managed 3.14 environment; the GUI calls the same compatibility-aware, `pip check`-gated `venv upgrade` path as the CLI. Global badges and Make Global use the canonical `base/envs/name` spec (aliases such as `venv:name` still match). `venv use` is covered by Make Global / Make Local; shell-scoped `enchi shell` remains CLI-only.
- **Diagnostics & Self-Healing**: Runs doctor/preflight checks and can apply automated repairs. Warnings use the same `WARN` status as the CLI. Dock, Start Menu, and `.desktop` launches do not inherit interactive shell PATH. At startup the GUI prepends `ENCHI_ROOT/bin` and `shims` onto **its own process PATH** so doctor, pip, and runtime lookups work inside the app. Shell Integration cards still describe **terminal / launch PATH** (profile configured vs not, and whether shims were already on PATH when the GUI started). If prepend cannot put those dirs on process PATH and profiles already have `enchi init`, doctor reports Info rather than Warn and Self-Healing will not try to “repair” desktop PATH. On Windows, a Microsoft Store / App Installer `python.exe` alias is **Warn** when it sits ahead of enchi shims on the **terminal launch PATH**. **Self-Healing deletes those `python.exe` / `python3.exe` stubs** — that is the functional fix. Windows has no public API to flip the Settings toggles, so **Open App execution aliases** is optional insurance if you want App Installer `python.exe` / `python3.exe` shown Off (Windows 11: Settings > Apps > Advanced app settings > App execution aliases; Windows 10: Settings > Apps > App execution aliases).
- **Settings**: Interface language (`ui.language`, including Match system), architecture, pip bootstrap, and optional companion base-venv flags (`venv.auto_create_base_venv` and `venv.auto_use_base_venv` default **off**). On Windows, `windows.registry_mode=pep514` writes HKCU PEP-514 keys under `Software\Python\PyenvNative` (not `PythonCore`). The Windows settings block is hidden on Linux/macOS. Language can also be changed from the bottom of the sidebar.
- **Self-Update**: Check for and install Enchi updates from the UI.

## Visual Tour

![Enchi GUI Animation](./screenshots/animated_gui.webp)

<details>
<summary><b>View Screen Gallery</b></summary>
<br />

| Dashboard | Installed Versions | Virtual Envs |
| :---: | :---: | :---: |
| ![Dashboard](./screenshots/Dashboard.webp) | ![Installed Versions](./screenshots/Installed_Versions.webp) | ![Virtual Envs](./screenshots/VENVs.webp) |

| Install Runtimes | Shell Integration | Settings |
| :---: | :---: | :---: |
| ![Available](./screenshots/Available.webp) | ![Shell](./screenshots/Shell.webp) | ![Settings](./screenshots/Settings.webp) |

| Runtime upgrades | Update all packages |
| :---: | :---: |
| ![Runtime upgrades](./screenshots/Upgrades.webp) | ![Update all packages](./screenshots/Bulk_Update_Done.webp) |

| Where does this command come from? | Rename a venv | Check dependencies |
| :---: | :---: | :---: |
| ![Command lookup](./screenshots/Dashboard_Which.webp) | ![Rename a venv](./screenshots/VENV_Rename.webp) | ![Check dependencies](./screenshots/Packages_Check.webp) |

![About](./screenshots/About.webp)

Regenerate from the repo root with `node ./scripts/gui-screenshots/capture.mjs` (Playwright + ffmpeg). The script serves the UI with a mocked Tauri layer and the real locale catalogs. Useful knobs: `GUI_COLOR_SCHEME=light`, `GUI_VIEWPORT=420x860` (narrow layout, PNG only), `GUI_LOCALE=de|ar|…`, `GUI_MOCK=empty|issues|errors`, `GUI_VIEWS=VENVs,Settings`, `GUI_SCREENSHOT_OUT=<dir>`, `GUI_CHROMIUM=<binary>`, and `GUI_AXE=1` to run axe-core (contrast included) on every frame.

</details>

## Installation & Launch

Native release bundles for **Windows x64**, **Linux x64**, and **macOS** include `enchi-gui` next to `enchi`. After install:

```text
enchi gui
```

Standalone GUI binaries (and matching `.sha256` files) are also attached to each GitHub Release. Verify the checksum before running a standalone download.

### Development (any platform)

```bash
cargo xtask gui            # build, replace a running GUI, launch
cargo xtask gui --release
```

### Building from source (macOS/Linux)

```bash
cargo build --release -p enchi-gui
```

Linux needs `webkit2gtk-4.1` and other Tauri system dependencies.

### Accessibility check

Static markup (jsdom + axe, contrast off):

```bash
pnpm --dir scripts/gui-a11y install --frozen-lockfile
pnpm --dir scripts/gui-a11y run check
```

Rendered GUI evidence gate (WCAGate, WCAG 2.2 AA, contrast on). This does **not** invent a WCAG percentage:

```bash
npm --prefix scripts/wcagate install
npm --prefix scripts/wcagate run wcagate:doctor
npm --prefix scripts/wcagate run wcagate:prepare
npm --prefix scripts/wcagate run wcagate:audit
```

## Branding

Everything brand-specific in the UI lives in three places, so a rename is a small change:

| What | Where |
|---|---|
| Product name and command shown in the UI | `crates/enchi-gui/ui/brand.js` (`Brand.name`, `Brand.command`), plus `BRAND_ARGS` in `crates/enchi-core/src/i18n/catalog.rs` |
| Logo (sidebar and About page) | `crates/enchi-gui/ui/brand/mark.svg` |
| Accent colour | `--accent` and `--accent-strong` in `crates/enchi-gui/ui/styles/tokens.css` |

Locale strings use the `{ $brand }` and `{ $command }` placeholders instead of the literal name, in every language. The window
icons (`crates/enchi-gui/icons`), the Tauri bundle identifier and the repository links in `app.js` are separate and must be
changed by hand.

---

The GUI is a companion to the CLI. Changes in the GUI are reflected in the terminal and vice versa.
