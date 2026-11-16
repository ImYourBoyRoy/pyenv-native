// ./scripts/gui-screenshots/capture.mjs
/**
 * Capture GUI view stills and an animated WebP tour for README / docs.
 *
 * Serves crates/enchi-gui/ui with a mocked Tauri invoke layer, then uses
 * Playwright (Chromium) to screenshot each primary view. Does not drive the
 * OS desktop, XTest, or GNOME Remote Desktop.
 *
 * Run from repo root:
 *   node ./scripts/gui-screenshots/capture.mjs
 *
 * Inputs: crates/enchi-gui/ui (HTML/CSS/JS), locales/<tag>/gui.ftl, Playwright from scripts/wcagate.
 * Outputs: docs/screenshots/*.webp (every view, the command lookup, venv rename, the dependency
 * check) plus animated_gui.webp.
 *
 * Review runs (nothing is written to docs/): set GUI_SCREENSHOT_OUT to a scratch directory and
 * combine GUI_COLOR_SCHEME=light, GUI_VIEWPORT=420x860, GUI_LOCALE=ar, GUI_MOCK=empty|issues|errors,
 * GUI_VIEWS=VENVs,Settings, GUI_PNG_ONLY=1, GUI_AXE=1 (axe-core on every frame), and GUI_CHROMIUM=<binary>.
 * PNG runs also capture the "scratch" frames (open menu, details, confirm and result dialogs).
 */
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '../..');
const uiRoot = path.join(repoRoot, 'crates/enchi-gui/ui');
const outDir = process.env.GUI_SCREENSHOT_OUT
  ? path.resolve(process.env.GUI_SCREENSHOT_OUT)
  : path.join(repoRoot, 'docs/screenshots');
const COLOR_SCHEME = process.env.GUI_COLOR_SCHEME === 'light' ? 'light' : 'dark';
const tmpDir = path.join(repoRoot, '.tmp-gui-screenshots');
const playwrightEntry = path.join(
  repoRoot,
  'scripts/wcagate/node_modules/playwright/index.mjs',
);

const WIDTH = 1024;
const HEIGHT = 720;
// GUI_VIEWPORT=420x860 captures a narrow layout; it writes PNGs only (the WebP tour is 1024x720).
const [VIEW_W, VIEW_H] = (process.env.GUI_VIEWPORT || `${WIDTH}x${HEIGHT}`).split('x').map(Number);
const PNG_ONLY = process.env.GUI_PNG_ONLY === '1' || VIEW_W !== WIDTH || VIEW_H !== HEIGHT;
const SUFFIX = process.env.GUI_SUFFIX || '';
const MIME = {
  '.css': 'text/css; charset=utf-8',
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.woff2': 'font/woff2',
  '.png': 'image/png',
  '.webp': 'image/webp',
};

const VIEWS = [
  { nav: 'view-dashboard', file: 'Dashboard', wait: waitDashboard },
  { nav: 'view-dashboard', file: 'Dashboard_Which', pre: runWhich, wait: waitWhich, extra: true },
  { nav: 'view-available', file: 'Available', wait: waitCatalog },
  { nav: 'view-installed', file: 'Installed_Versions', wait: waitInstalled },
  { nav: 'view-venvs', file: 'VENVs', wait: waitVenvs },
  { nav: 'view-venvs', file: 'VENV_Rename', pre: openRename, wait: waitRename, extra: true },
  { nav: 'view-venvs', file: 'VENV_Details', pre: openDetails, wait: waitDetails, extra: true, scratch: true },
  { nav: 'view-installed', file: 'Installed_Menu', pre: openRuntimeMenu, wait: waitRuntimeMenu, extra: true, scratch: true },
  { nav: 'view-installed', file: 'Packages_Check', pre: openPackageCheck, wait: waitPackageCheck, extra: true },
  { nav: 'view-installed', file: 'Upgrades', pre: openUpgradePlan, wait: waitUpgradePlan, keepTray: true },
  { nav: 'view-installed', file: 'Upgrades_Done', pre: applyUpgrades, wait: waitUpgradeDone, keepTray: true },
  { nav: 'view-venvs', file: 'Bulk_Update', pre: openBulkPlan, wait: waitBulkPlan, keepTray: true },
  { nav: 'view-venvs', file: 'Bulk_Update_Done', pre: applyBulk, wait: waitBulkDone, keepTray: true },
  { nav: 'view-shell', file: 'Shell', wait: waitShell },
  { nav: 'view-dashboard', file: 'Which_Missing', pre: runWhichMissing, wait: waitWhichMissing, extra: true, scratch: true },
  { nav: 'view-installed', file: 'Uninstall_Confirm', pre: openUninstall, wait: waitModal, extra: true, scratch: true },
  { nav: 'view-settings', file: 'Settings', wait: waitSettings },
  { nav: 'view-about', file: 'About', wait: waitAbout },
  // Last on purpose: the mocked rename really renames, which later frames would show.
  { nav: 'view-venvs', file: 'Rename_Done', pre: submitRename, wait: waitModal, extra: true, scratch: true },
];

/** Minimal Fluent reader for locales/<tag>/gui.ftl: `id = value` plus indented continuation lines. */
function parseFtl(file) {
  const messages = {};
  if (!fs.existsSync(file)) return messages;
  let id = null;
  for (const line of fs.readFileSync(file, 'utf8').split(/\r?\n/)) {
    const head = /^([a-z][a-z0-9-]*)\s*=\s?(.*)$/i.exec(line);
    if (head) {
      id = head[1];
      messages[id] = head[2].trim();
    } else if (id && /^\s+\S/.test(line)) {
      messages[id] = `${messages[id]}\n${line.trim()}`.trim();
    } else {
      id = null;
    }
  }
  return messages;
}

// GUI_LOCALE=de renders the UI from locales/de/gui.ftl (English fills any gap), like the real app.
const LOCALE = process.env.GUI_LOCALE || 'en-US';
function i18nBundle() {
  const messages = { ...parseFtl(path.join(repoRoot, 'locales/en-US/gui.ftl')) };
  if (LOCALE !== 'en-US') Object.assign(messages, parseFtl(path.join(repoRoot, `locales/${LOCALE}/gui.ftl`)));
  const rtl = ['ar', 'fa'].includes(LOCALE);
  const tags = fs.readdirSync(path.join(repoRoot, 'locales')).filter((tag) => fs.statSync(path.join(repoRoot, 'locales', tag)).isDirectory());
  return {
    lang: LOCALE,
    dir: rtl ? 'rtl' : 'ltr',
    native_name: LOCALE === 'en-US' ? 'English' : LOCALE,
    source: 'explicit',
    messages,
    locales: tags.map((tag) => ({ tag, native_name: tag === 'en-US' ? 'English' : tag, english_name: tag, rtl: ['ar', 'fa'].includes(tag) })),
  };
}

// GUI_AXE=1 also runs axe-core (colour contrast included) on every frame and prints violations;
// it needs scripts/gui-a11y/node_modules. Combine with GUI_COLOR_SCHEME=light|dark.
const AXE = process.env.GUI_AXE === '1';
const axeSource = AXE
  ? fs.readFileSync(path.join(repoRoot, 'scripts/gui-a11y/node_modules/axe-core/axe.min.js'), 'utf8')
  : '';
let axeFailures = 0;

// GUI_MOCK=empty|issues|errors renders empty lists, health warnings plus the setup banner, or
// failing backend calls (use with GUI_VIEWS; the frame waits are skipped).
const MOCK = process.env.GUI_MOCK || '';

// GUI_VIEWS=VENVs,Settings limits a run to those frames (file names without extension).
const ONLY = (process.env.GUI_VIEWS || '').split(',').map((v) => v.trim()).filter(Boolean);

function mockTauriSource() {
  const status = {
    root: '/home/roy/.enchi',
    active_versions: ['3.14.3'],
    global_versions: ['3.14.3'],
    origin: 'global',
    managed_venv: null,
  };
  const installed = ['3.14.3'];
  const venvs = [
    {
      name: 'pondlets-play',
      base_version: '3.14.3',
      spec: '3.14.3/envs/pondlets-play',
      path: '/home/roy/.enchi/venvs/3.14.3/pondlets-play',
      python_path: '/home/roy/.enchi/venvs/3.14.3/pondlets-play/bin/python',
      pip_path: '/home/roy/.enchi/venvs/3.14.3/pondlets-play/bin/pip',
    },
    {
      name: 'server-manager',
      base_version: '3.14.3',
      spec: '3.14.3/envs/server-manager',
      path: '/home/roy/.enchi/venvs/3.14.3/server-manager',
      python_path: '/home/roy/.enchi/venvs/3.14.3/server-manager/bin/python',
      pip_path: '/home/roy/.enchi/venvs/3.14.3/server-manager/bin/pip',
    },
  ];
  const catalog = [
    { family: 'CPython 3', family_slug: 'cpython', source: 'known', versions: ['3.14.3', '3.14.2', '3.13.7', '3.13.5', '3.12.11', '3.11.13', '3.10.18', '3.9.23'] },
    { family: 'CPython 2', family_slug: 'cpython', source: 'known', versions: ['2.7.18'] },
    { family: 'PyPy', family_slug: 'pypy', source: 'known', versions: ['pypy3.11-7.3.23', 'pypy3.10-7.3.19', 'pypy3.9-7.3.16'] },
  ];
  const config = {
    storage: {},
    windows: { registry_mode: 'disabled' },
    install: { arch: 'auto', bootstrap_pip: true },
    venv: { auto_create_base_venv: false, auto_use_base_venv: false },
    plugins: { search_path: true },
  };
  const doctor = [
    { name: 'Shims on PATH', status: 'OK', detail: 'Shim directory is first on PATH.' },
    { name: 'Shell init', status: 'OK', detail: 'Bash profile contains enchi init.' },
    { name: 'Install root', status: 'OK', detail: 'Managed binaries are present under ~/.enchi.' },
  ];
  const shells = [
    { name: 'PowerShell 7 (pwsh)', profile_path: '/home/roy/.config/powershell/Microsoft.PowerShell_profile.ps1', is_configured: false, active_in_path: true, is_installed: true, path_label: 'PATH Active' },
    { name: 'Zsh', profile_path: '/home/roy/.zshrc', is_configured: false, active_in_path: true, is_installed: true, path_label: 'PATH Active' },
    { name: 'Bash', profile_path: '/home/roy/.bashrc', is_configured: true, active_in_path: true, is_installed: true, path_label: 'PATH Active' },
    { name: 'Fish', profile_path: '/home/roy/.config/fish/config.fish', is_configured: false, active_in_path: true, is_installed: true, path_label: 'PATH Active' },
  ];
  const intel = {
    os: 'linux',
    arch: 'x86_64',
    os_pretty_name: 'Ubuntu 24.04 LTS',
    install_strategy: 'native-binary',
    ready_to_install: true,
    verdict: 'ready',
    summary: 'This host can install official CPython builds and PyPy without compiling from source.',
    facts: [
      { key: 'os', label: 'OS', value: 'Linux (Ubuntu 24.04 LTS)' },
      { key: 'arch', label: 'Architecture', value: 'x86_64' },
      { key: 'strategy', label: 'Install strategy', value: 'Native binaries' },
      { key: 'root', label: 'ENCHI_ROOT', value: '/home/roy/.enchi' },
    ],
    blocking_issues: [],
    warnings: [],
  };
  const cache = {
    total_bytes: 184549376,
    entries: [
      { name: 'All Enchi cache', path: '/home/roy/.enchi/cache', bytes: 184549376, exists: true },
      { name: 'Python downloads / packages', path: '/home/roy/.enchi/cache/packages', bytes: 150994944, exists: true },
      { name: 'python-build cache', path: '/home/roy/.enchi/cache/python-build', bytes: 0, exists: false },
      { name: 'Metadata cache', path: '/home/roy/.enchi/cache/metadata', bytes: 1048576, exists: true },
    ],
  };
  const installStatus = {
    is_installed: true,
    root: '/home/roy/.enchi',
    is_portable: false,
    cli_on_path: true,
    profiles_configured: true,
    shims_on_gui_path: true,
    needs_shell_setup: false,
    platform: 'linux',
  };

  return `
(() => {
  const status = ${JSON.stringify(status)};
  const installed = ${JSON.stringify(installed)};
  const venvs = ${JSON.stringify(venvs)};
  const catalog = ${JSON.stringify(catalog)};
  const config = ${JSON.stringify(config)};
  const doctor = ${JSON.stringify(doctor)};
  const shells = ${JSON.stringify(shells)};
  const intel = ${JSON.stringify(intel)};
  const cache = ${JSON.stringify(cache)};
  const installStatus = ${JSON.stringify(installStatus)};
  const i18n = ${JSON.stringify(i18nBundle())};

  const MOCK = ${JSON.stringify(MOCK)};
  if (MOCK === 'empty') { installed.length = 0; venvs.length = 0; status.active_versions = []; status.global_versions = []; status.origin = 'system'; }
  if (MOCK === 'issues') {
    doctor.push({ name: 'Shims on PATH', status: 'WARN', detail: 'The shim directory is not on PATH in this session.' });
    doctor.push({ name: 'Compiler toolchain', status: 'FAIL', detail: 'No C compiler was found; source builds will fail.' });
    installStatus.needs_shell_setup = true;
    installStatus.profiles_configured = false;
  }
  const failing = (name) => () => { throw new Error('enchi: ' + name + ' failed: could not read ' + status.root + '/versions (permission denied)'); };

  const handlers = {
    check_install_status: () => installStatus,
    get_status: () => JSON.stringify(status),
    get_installed_versions: () => JSON.stringify(installed),
    get_managed_venvs: () => JSON.stringify(venvs),
    get_available_versions: () => JSON.stringify(catalog),
    get_config: () => JSON.stringify(config),
    run_doctor: () => doctor,
    get_shell_statuses: () => shells,
    get_platform_intelligence: () => intel,
    get_cache_stats: () => cache,
    get_app_version: () => '0.2.38',
    i18n_bundle: () => i18n,
    set_ui_language: () => i18n,
    get_outdated_packages: () => JSON.stringify([]),
  };

  const listeners = [];
  const emit = (payload) => listeners.forEach((cb) => cb({ payload }));
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

  const upgradePlan = [
    { installed: '3.14.3', latest: '3.14.10', status: 'available', message: '3.14.10 is available', venvs: [], repinned: [], removed_old: false },
    { installed: '3.13.7', latest: '3.13.7', status: 'up_to_date', message: 'already the newest patch', venvs: [], repinned: [], removed_old: false },
  ];
  const outdatedVenvs = [
    { spec: '3.14.3/envs/pondlets-play', kind: 'venv', status: 'planned', message: '2 package(s) would be updated', conflicts_before: 0, conflicts_after: 0, updated: [],
      outdated: [{ name: 'requests', version: '2.31.0', latest_version: '2.32.3' }, { name: 'pip', version: '26.2.2', latest_version: '26.3' }] },
    { spec: '3.14.3/envs/server-manager', kind: 'venv', status: 'planned', message: '1 package(s) would be updated', conflicts_before: 0, conflicts_after: 0, updated: [],
      outdated: [{ name: 'qh3', version: '1.9.4', latest_version: '2.1.0' }] },
    { spec: '3.14.3', kind: 'runtime', status: 'up_to_date', message: 'all packages are current', conflicts_before: 0, conflicts_after: 0, updated: [], outdated: [] },
  ];

  const pipPackages = [
    { name: 'certifi', version: '2025.8.3' },
    { name: 'pip', version: '26.2.2' },
    { name: 'requests', version: '2.31.0' },
    { name: 'urllib3', version: '2.2.1' },
  ];
  const prefixOf = (version) => '/home/roy/.enchi/versions/' + version;

  Object.assign(handlers, {
    ui_report: () => null,
    get_pip_packages: () => JSON.stringify(pipPackages),
    check_pip_conflicts: () => JSON.stringify([
      { package: 'requests', requirement: 'urllib3<2', installed: '2.2.1', message: 'requests 2.31.0 has requirement urllib3<2,>=1.21.1, but you have urllib3 2.2.1.' },
    ]),
    get_runtime_prefixes: (args) => (args.versions || []).filter((v) => v !== 'system').map((v) => ({ version: v, path: prefixOf(v) })),
    open_managed_folder: () => null,
    which_command: (args) => {
      const command = String(args.command || '').trim();
      if (command === 'python' || command === 'python3') {
        return { command, found: true, path: prefixOf('3.14.3') + '/bin/python', message: '', also_in: ['3.13.7', '3.14.3'] };
      }
      return {
        command, found: false, path: null, also_in: [],
        message: 'enchi: ' + command + ': command not found',
      };
    },
    plan_venv_rename: (args) => {
      const name = String(args.newName || '').trim();
      if (venvs.some((v) => v.name === name)) throw new Error('enchi: a managed venv named \u0060' + name + '\u0060 already exists (3.14.3/envs/' + name + ')');
      return 'Would rename managed venv ' + args.spec + ' -> 3.14.3/envs/' + name;
    },
    rename_venv: (args) => {
      const name = String(args.newName || '').trim();
      const found = venvs.find((v) => v.spec === args.spec);
      if (found) { found.name = name; found.spec = '3.14.3/envs/' + name; found.path = '/home/roy/.enchi/venvs/3.14.3/' + name; }
      return {
        old_spec: args.spec, new_spec: '3.14.3/envs/' + name,
        message: 'Renamed managed venv ' + args.spec + ' -> 3.14.3/envs/' + name,
        hints: ['update any \u0060.python-version\u0060 files that referenced the old venv spec.'],
        warnings: ['activate scripts, script shebangs, and (on Windows) console-script launchers inside the venv still reference the old path; prefer \u0060enchi venv upgrade\u0060 or recreate the venv if entry points such as \u0060pip\u0060 stop working.'],
      };
    },
    plan_runtime_upgrades: () => JSON.stringify(upgradePlan),
    run_runtime_upgrades: async (args) => {
      for (const step of ['3.14.3 -> 3.14.10: installing 3.14.10', '3.14.3 -> 3.14.10: migrating venv 3.14.3/envs/pondlets-play']) {
        emit({ id: args.operationId, step });
        await sleep(120);
      }
      return JSON.stringify({ dry_run: false, items: [
        { installed: '3.14.3', latest: '3.14.10', status: 'upgraded', message: 'upgraded to 3.14.10',
          venvs: [{ spec: '3.14.3/envs/pondlets-play', ok: true, message: 'moved to 3.14.10' }],
          repinned: ['/home/roy/.enchi/version'], removed_old: false },
        upgradePlan[1],
      ] });
    },
    plan_bulk_package_updates: () => JSON.stringify({ dry_run: true, targets: outdatedVenvs,
      summary: { targets: 3, up_to_date: 1, updated: 0, packages_updated: 0, rolled_back: 0, failed: 0, skipped: 0 } }),
    run_bulk_package_updates: async (args) => {
      emit({ id: args.operationId, step: '[1/3] 3.14.3/envs/pondlets-play' });
      await sleep(120);
      emit({ id: args.operationId, step: '[2/3] 3.14.3/envs/server-manager' });
      await sleep(120);
      const done = outdatedVenvs.slice(0, 2).map((t) => ({ ...t, status: 'updated', updated: t.outdated.map((o) => o.name), message: t.outdated.length + ' package(s) updated' }));
      return JSON.stringify({ dry_run: false, targets: [...done, outdatedVenvs[2]],
        summary: { targets: 3, up_to_date: 1, updated: 2, packages_updated: 3, rolled_back: 0, failed: 0, skipped: 0 } });
    },
  });

  if (MOCK === 'errors') {
    handlers.get_installed_versions = failing('versions');
    handlers.get_managed_venvs = failing('venv list');
    handlers.get_status = failing('status');
    handlers.get_available_versions = failing('catalog');
    handlers.get_cache_stats = failing('cache');
    handlers.which_command = failing('which');
  }

  async function invoke(cmd, args) {
    if (!(cmd in handlers)) {
      throw new Error('Unmocked Tauri command: ' + cmd);
    }
    return handlers[cmd](args || {});
  }

  window.__TAURI__ = {
    core: { invoke },
    invoke,
    event: { listen: async (_name, cb) => { listeners.push(cb); return () => { const i = listeners.indexOf(cb); if (i >= 0) listeners.splice(i, 1); }; } },
  };
  localStorage.setItem('pyenv-workspace-dir', '/home/roy/projects/pondlets-play');

  const originalFetch = window.fetch.bind(window);
  window.fetch = async (input, init) => {
    const url = String(input);
    if (url.includes('api.github.com/repos/') && url.includes('/releases/latest')) {
      return new Response(JSON.stringify({ tag_name: 'v0.2.38' }), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      });
    }
    return originalFetch(input, init);
  };
})();
`;
}

function appCsp() {
  const conf = JSON.parse(fs.readFileSync(path.join(repoRoot, 'crates/enchi-gui/tauri.conf.json'), 'utf8'));
  return conf.app.security.csp;
}

function startServer() {
  return new Promise((resolve, reject) => {
    const server = http.createServer((req, res) => {
      const urlPath = decodeURIComponent((req.url || '/').split('?')[0]);
      const rel = urlPath === '/' ? 'index.html' : urlPath.replace(/^\/+/, '');
      const filePath = path.normalize(path.join(uiRoot, rel));
      if (!filePath.startsWith(uiRoot)) {
        res.writeHead(403);
        res.end('forbidden');
        return;
      }
      fs.readFile(filePath, (err, data) => {
        if (err) {
          res.writeHead(404);
          res.end('not found');
          return;
        }
        res.writeHead(200, {
          'Content-Type': MIME[path.extname(filePath)] || 'application/octet-stream',
          // Serve with the same CSP the Tauri window enforces so style regressions show up here.
          'Content-Security-Policy': appCsp(),
        });
        res.end(data);
      });
    });
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      resolve({ server, origin: `http://127.0.0.1:${port}` });
    });
  });
}

async function waitDashboard(page) {
  await page.waitForFunction(() => document.getElementById('active-version')?.textContent?.trim() === '3.14.3');
  await page.waitForFunction(() => document.getElementById('setup-banner')?.style.display === 'none');
}

async function waitCatalog(page) {
  await page.waitForFunction(() => document.querySelectorAll('#available-list .version-card').length >= 6);
}

async function openUpgradePlan(page) {
  await page.locator('#btn-check-upgrades').click();
}
async function waitUpgradePlan(page) {
  await page.waitForFunction(() => document.querySelectorAll('#upgrade-panel .maint-row').length >= 2);
}
async function applyUpgrades(page) {
  await page.locator('#upgrade-panel .maint-actions .btn-primary').click();
}
async function waitUpgradeDone(page) {
  await page.waitForFunction(() => /upgraded to 3\.14\.10/.test(document.getElementById('upgrade-panel')?.textContent || ''));
}
async function openBulkPlan(page) {
  await page.locator('#btn-bulk-update').click();
}
async function waitBulkPlan(page) {
  await page.waitForFunction(() => document.querySelectorAll('#bulk-panel .maint-row').length >= 2);
}
async function applyBulk(page) {
  await page.locator('#bulk-panel .maint-actions .btn-primary').click();
}
async function waitBulkDone(page) {
  await page.waitForFunction(() => /package\(s\) updated in 2 environment/.test(document.getElementById('bulk-panel')?.textContent || ''));
}

async function runWhich(page) {
  await page.locator('#which-input').fill('python');
  await page.locator('#btn-which').click();
}
async function waitWhich(page) {
  await page.waitForSelector('#which-result:not([hidden]) .path-text');
}
async function openCardMenu(page, listId) {
  await page.locator(`#${listId} .version-card`).first().locator('.card-menu-toggle').click();
}
async function openRename(page) {
  await openCardMenu(page, 'venvs-list');
  await page.locator('#venvs-list .version-card').first().locator('[data-action="venv-rename"]').click();
  await page.locator('#venvs-list .version-card').first().locator('.rename-form input').fill('pondlets');
}
async function waitRename(page) {
  await page.waitForSelector('#venvs-list .rename-preview[data-kind="ok"]');
}
async function submitRename(page) {
  await openRename(page);
  await waitRename(page);
  await page.locator('#venvs-list .version-card').first().locator('.rename-form button[type="submit"]').click();
}
async function runWhichMissing(page) {
  await page.locator('#which-input').fill('no-such-tool');
  await page.locator('#btn-which').click();
}
async function waitWhichMissing(page) {
  await page.waitForSelector('#which-result[data-kind="missing"]');
}
async function openUninstall(page) {
  await page.waitForFunction(() => document.querySelectorAll('#installed-list .version-card').length >= 2);
  await page.locator('#installed-list .version-card').nth(1).locator('.card-menu-toggle').click();
  await page.locator('#installed-list [data-action="uninstall"]').click();
}
async function waitModal(page) {
  await page.waitForSelector('.modal-box');
}
async function openDetails(page) {
  await openCardMenu(page, 'venvs-list');
  await page.locator('#venvs-list .version-card').first().locator('[data-action="venv-details"]').click();
}
async function waitDetails(page) {
  await page.waitForFunction(() => document.querySelectorAll('#venvs-list .version-panel:not([hidden]) .detail-row').length >= 3);
}
async function openRuntimeMenu(page) {
  await page.waitForFunction(() => document.querySelectorAll('#installed-list .version-card').length >= 2);
  await page.locator('#installed-list .version-card').nth(1).locator('.card-menu-toggle').click();
}
async function waitRuntimeMenu(page) {
  await page.waitForSelector('#installed-list .card-menu.is-open [data-action="open-folder"]');
}
async function openPackageCheck(page) {
  await page.locator('#installed-list .version-card').nth(1).locator('[data-action="package-explorer"]').click();
  await page.waitForFunction(() => document.querySelectorAll('#drawer-package-list tr').length >= 3);
  await page.locator('#btn-pip-check').click();
}
async function waitPackageCheck(page) {
  await page.waitForSelector('#pip-check-result:not([hidden]) .result-list');
}

async function waitInstalled(page) {
  await page.waitForFunction(() => document.querySelectorAll('#installed-list .version-card').length >= 2);
}

async function waitVenvs(page) {
  await page.waitForFunction(() => document.querySelectorAll('#venvs-list .version-card').length >= 2);
}

async function waitShell(page) {
  await page.waitForFunction(() => document.querySelectorAll('#shell-status-cards .shell-card').length >= 3);
  await page.waitForFunction(() => document.getElementById('platform-intel-verdict')?.classList.contains('ready'));
  await page.evaluate(() => document.getElementById('btn-run-doctor')?.click());
  await page.locator('#doctor-healthy-card').waitFor({ state: 'visible' });
}

async function waitSettings(page) {
  await page.waitForFunction(() => document.querySelectorAll('#cache-stats .cache-row').length >= 1);
}

async function waitAbout(page) {
  await page.waitForSelector('#view-about', { state: 'visible' });
}

function encodeWebp(pngPath, webpPath, extraArgs = []) {
  const result = spawnSync(
    'ffmpeg',
    ['-y', '-i', pngPath, '-vf', `scale=${WIDTH}:${HEIGHT}`, '-c:v', 'libwebp', '-quality', '82', '-compression_level', '6', ...extraArgs, webpPath],
    { stdio: 'inherit' },
  );
  if (result.status !== 0) {
    throw new Error(`ffmpeg failed for ${webpPath}`);
  }
}

function encodeAnimation(pngs, webpPath) {
  const listPath = path.join(tmpDir, 'concat.txt');
  const lines = [];
  for (const png of pngs) {
    lines.push(`file '${png.replace(/'/g, "'\\''")}'`);
    lines.push('duration 1.6');
  }
  lines.push(`file '${pngs[pngs.length - 1].replace(/'/g, "'\\''")}'`);
  fs.writeFileSync(listPath, `${lines.join('\n')}\n`);
  const result = spawnSync(
    'ffmpeg',
    [
      '-y', '-f', 'concat', '-safe', '0', '-i', listPath,
      '-vf', `scale=${WIDTH}:${HEIGHT}:flags=lanczos`,
      '-fps_mode', 'passthrough',
      '-c:v', 'libwebp', '-loop', '0', '-quality', '72', '-compression_level', '6',
      webpPath,
    ],
    { stdio: 'inherit' },
  );
  if (result.status !== 0) {
    throw new Error('ffmpeg failed for animated_gui.webp');
  }
}

async function main() {
  if (!fs.existsSync(playwrightEntry)) {
    throw new Error('Playwright is missing. Install deps in scripts/wcagate first.');
  }
  const { chromium } = await import(pathToFileURL(playwrightEntry).href);
  fs.mkdirSync(outDir, { recursive: true });
  fs.rmSync(tmpDir, { recursive: true, force: true });
  fs.mkdirSync(tmpDir, { recursive: true });

  const { server, origin } = await startServer();
  const browser = await chromium.launch({
    // GUI_CHROMIUM points at a specific Chromium/headless-shell binary when the one Playwright
    // expects is not installed.
    executablePath: process.env.GUI_CHROMIUM || undefined,
    args: ['--font-render-hinting=none', '--disable-lcd-text'],
  });
  try {
    const page = await browser.newPage({
      viewport: { width: VIEW_W, height: VIEW_H },
      deviceScaleFactor: Number(process.env.GUI_SCALE) || 2,
      colorScheme: COLOR_SCHEME,
    });
    const cspViolations = [];
    page.on('console', (msg) => {
      if (/Content Security Policy|Refused to/i.test(msg.text())) cspViolations.push(msg.text());
    });
    await page.addInitScript(mockTauriSource());
    await page.goto(`${origin}/index.html`, { waitUntil: 'networkidle' });
    await page.waitForFunction(() => document.fonts?.status === 'loaded' || document.fonts?.status === 'loading');
    await page.evaluate(() => document.fonts.ready);
    if (!MOCK) await waitDashboard(page);
    else await page.waitForTimeout(800);
    // Freeze motion through the app's own prefers-reduced-motion rules (inline styles are blocked by CSP).
    await page.emulateMedia({ reducedMotion: 'reduce' });

    const pngs = [];
    // `scratch` frames exist for PNG review runs (menus, details); they are not published as docs images.
    for (const view of VIEWS.filter((v) => (!ONLY.length || ONLY.includes(v.file)) && (!v.scratch || PNG_ONLY))) {
      // Frames start from a clean slate: no open drawer, menu, or leftover panel state.
      await page.keyboard.press('Escape');
      if (!view.keepTray) {
        // Finished background tasks from the maintenance frames should not cover the next page.
        await page.evaluate(() => document.querySelectorAll('.activity .act-x').forEach((btn) => btn.click()));
        await page.waitForTimeout(300);
      }
      await page.evaluate(() => {
        if (document.getElementById('drawer-package-manager')?.classList.contains('open')) {
          document.getElementById('btn-close-drawer')?.click();
        }
      });
      if (view.nav !== 'view-dashboard' || view.pre) {
        await page.locator(`.nav-btn[data-view="${view.nav}"]`).click();
      }
      if (view.pre) await view.pre(page);
      await page.evaluate(() => {
        document.querySelector('.content')?.scrollTo(0, 0);
        document.getElementById('main-content')?.scrollTo(0, 0);
      });
      if (MOCK) await page.waitForTimeout(700);
      else await view.wait(page);
      await page.evaluate(() => {
        if (!document.querySelector('.card-menu.is-open')) document.activeElement?.blur?.();
        document.querySelector('.content')?.scrollTo(0, 0);
      });
      await page.waitForTimeout(180);
      if (AXE) {
        // Runtime.evaluate is not subject to the page CSP, so axe can be injected this way.
        await page.evaluate(axeSource);
        const found = await page.evaluate(async () => {
          const result = await window.axe.run(document.body, {
            runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'] },
          });
          return result.violations.map((v) => ({
            id: v.id, impact: v.impact, count: v.nodes.length,
            sample: v.nodes.slice(0, 3).map((n) => `${n.target.join(' ')} :: ${(n.any[0] || n.all[0] || n.none[0] || {}).message || ''}`),
          }));
        });
        if (found.length) axeFailures += found.length;
        console.log(`axe ${view.file}: ${found.length ? JSON.stringify(found, null, 1) : 'ok'}`);
      }
      const pngPath = path.join(tmpDir, `${view.file}${SUFFIX}.png`);
      await page.screenshot({ path: pngPath, type: 'png' });
      if (PNG_ONLY) {
        fs.copyFileSync(pngPath, path.join(outDir, `${view.file}${SUFFIX}.png`));
        console.log(`wrote ${view.file}${SUFFIX}.png`);
        continue;
      }
      if (!view.extra) pngs.push(pngPath);
      encodeWebp(pngPath, path.join(outDir, `${view.file}.webp`));
      console.log(`wrote ${view.file}.webp`);
    }

    if (!PNG_ONLY) {
      encodeAnimation(pngs, path.join(outDir, 'animated_gui.webp'));
      console.log('wrote animated_gui.webp');
    }
    if (AXE && axeFailures) console.log(`axe: ${axeFailures} violation group(s) in total`);
    if (cspViolations.length) {
      throw new Error(`CSP violations (${cspViolations.length}):\n${cspViolations.slice(0, 8).join('\n')}`);
    }
  } finally {
    await browser.close();
    server.close();
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
