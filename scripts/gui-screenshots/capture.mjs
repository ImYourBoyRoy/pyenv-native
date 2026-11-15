// ./scripts/gui-screenshots/capture.mjs
/**
 * Capture GUI view stills and an animated WebP tour for README / docs.
 *
 * Serves crates/pyenv-gui/ui with a mocked Tauri invoke layer, then uses
 * Playwright (Chromium) to screenshot each primary view. Does not drive the
 * OS desktop, XTest, or GNOME Remote Desktop.
 *
 * Run from repo root:
 *   node ./scripts/gui-screenshots/capture.mjs
 *
 * Inputs: crates/pyenv-gui/ui (HTML/CSS/JS), Playwright from scripts/wcagate.
 * Outputs: docs/screenshots/{Dashboard,Available,Installed_Versions,VENVs,Shell,Settings,About,animated_gui}.webp
 */
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '../..');
const uiRoot = path.join(repoRoot, 'crates/pyenv-gui/ui');
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
  { nav: 'view-available', file: 'Available', wait: waitCatalog },
  { nav: 'view-installed', file: 'Installed_Versions', wait: waitInstalled },
  { nav: 'view-venvs', file: 'VENVs', wait: waitVenvs },
  { nav: 'view-installed', file: 'Upgrades', pre: openUpgradePlan, wait: waitUpgradePlan },
  { nav: 'view-installed', file: 'Upgrades_Done', pre: applyUpgrades, wait: waitUpgradeDone },
  { nav: 'view-venvs', file: 'Bulk_Update', pre: openBulkPlan, wait: waitBulkPlan },
  { nav: 'view-venvs', file: 'Bulk_Update_Done', pre: applyBulk, wait: waitBulkDone },
  { nav: 'view-shell', file: 'Shell', wait: waitShell },
  { nav: 'view-settings', file: 'Settings', wait: waitSettings },
  { nav: 'view-about', file: 'About', wait: waitAbout },
];

function mockTauriSource() {
  const status = {
    root: '/home/roy/.pyenv',
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
      path: '/home/roy/.pyenv/venvs/3.14.3/pondlets-play',
      python_path: '/home/roy/.pyenv/venvs/3.14.3/pondlets-play/bin/python',
      pip_path: '/home/roy/.pyenv/venvs/3.14.3/pondlets-play/bin/pip',
    },
    {
      name: 'server-manager',
      base_version: '3.14.3',
      spec: '3.14.3/envs/server-manager',
      path: '/home/roy/.pyenv/venvs/3.14.3/server-manager',
      python_path: '/home/roy/.pyenv/venvs/3.14.3/server-manager/bin/python',
      pip_path: '/home/roy/.pyenv/venvs/3.14.3/server-manager/bin/pip',
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
    { name: 'Shell init', status: 'OK', detail: 'Bash profile contains pyenv init.' },
    { name: 'Install root', status: 'OK', detail: 'Managed binaries are present under ~/.pyenv.' },
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
      { key: 'root', label: 'PYENV_ROOT', value: '/home/roy/.pyenv' },
    ],
    blocking_issues: [],
    warnings: [],
  };
  const cache = {
    total_bytes: 184549376,
    entries: [
      { name: 'All pyenv cache', path: '/home/roy/.pyenv/cache', bytes: 184549376, exists: true },
      { name: 'Python downloads / packages', path: '/home/roy/.pyenv/cache/packages', bytes: 150994944, exists: true },
      { name: 'python-build cache', path: '/home/roy/.pyenv/cache/python-build', bytes: 0, exists: false },
      { name: 'Metadata cache', path: '/home/roy/.pyenv/cache/metadata', bytes: 1048576, exists: true },
    ],
  };
  const installStatus = {
    is_installed: true,
    root: '/home/roy/.pyenv',
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

  Object.assign(handlers, {
    ui_report: () => null,
    plan_runtime_upgrades: () => JSON.stringify(upgradePlan),
    run_runtime_upgrades: async (args) => {
      for (const step of ['3.14.3 -> 3.14.10: installing 3.14.10', '3.14.3 -> 3.14.10: migrating venv 3.14.3/envs/pondlets-play']) {
        emit({ id: args.operationId, step });
        await sleep(120);
      }
      return JSON.stringify({ dry_run: false, items: [
        { installed: '3.14.3', latest: '3.14.10', status: 'upgraded', message: 'upgraded to 3.14.10',
          venvs: [{ spec: '3.14.3/envs/pondlets-play', ok: true, message: 'moved to 3.14.10' }],
          repinned: ['/home/roy/.pyenv/version'], removed_old: false },
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
  const conf = JSON.parse(fs.readFileSync(path.join(repoRoot, 'crates/pyenv-gui/tauri.conf.json'), 'utf8'));
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

async function waitInstalled(page) {
  await page.waitForFunction(() => document.querySelectorAll('#installed-list .version-card').length >= 2);
}

async function waitVenvs(page) {
  await page.waitForFunction(() => document.querySelectorAll('#venvs-list .version-card').length >= 2);
}

async function waitShell(page) {
  await page.waitForFunction(() => document.querySelectorAll('#shell-status-cards .shell-card').length >= 3);
  await page.waitForFunction(() => document.getElementById('platform-intel-verdict')?.textContent?.trim() === 'Ready');
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
    args: ['--font-render-hinting=none', '--disable-lcd-text'],
  });
  try {
    const page = await browser.newPage({
      viewport: { width: WIDTH, height: HEIGHT },
      deviceScaleFactor: 2,
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
    await waitDashboard(page);
    // Freeze motion through the app's own prefers-reduced-motion rules (inline styles are blocked by CSP).
    await page.emulateMedia({ reducedMotion: 'reduce' });

    const pngs = [];
    for (const view of VIEWS) {
      if (view.nav !== 'view-dashboard') {
        await page.locator(`.nav-btn[data-view="${view.nav}"]`).click();
      }
      if (view.pre) await view.pre(page);
      await page.evaluate(() => {
        document.querySelector('.content')?.scrollTo(0, 0);
        document.getElementById('main-content')?.scrollTo(0, 0);
      });
      await view.wait(page);
      await page.evaluate(() => {
        document.activeElement?.blur?.();
        document.querySelector('.content')?.scrollTo(0, 0);
      });
      await page.waitForTimeout(180);
      const pngPath = path.join(tmpDir, `${view.file}.png`);
      await page.screenshot({ path: pngPath, type: 'png' });
      pngs.push(pngPath);
      encodeWebp(pngPath, path.join(outDir, `${view.file}.webp`));
      console.log(`wrote ${view.file}.webp`);
    }

    encodeAnimation(pngs, path.join(outDir, 'animated_gui.webp'));
    console.log('wrote animated_gui.webp');
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
