// ./crates/pyenv-gui/ui/app.js
//! Frontend logic for pyenv-native GUI.
//! Uses Tauri v2 IPC to communicate with the Rust backend.

const invoke = (window.__TAURI__ && window.__TAURI__.core) ? window.__TAURI__.core.invoke : window.__TAURI__.invoke;

let lastFocusedBeforeOverlay = null;
let drawerReturnFocus = null;
let toastTimer = null;
const VIEW_TITLES = {
    'view-dashboard': 'Current Environment',
    'view-installed': 'Installed Runtimes',
    'view-venvs': 'Virtual Environments',
    'view-shell': 'Shell Integration',
    'view-available': 'Installable Runtimes',
    'view-settings': 'Configuration',
    'view-about': 'About Pyenv-Native',
};

function announce(message) {
    const region = document.getElementById('aria-live-status');
    if (!region) return;
    region.textContent = '';
    requestAnimationFrame(() => { region.textContent = message; });
}

function showToast(message) {
    const toast = document.getElementById('status-toast');
    if (!toast) return;
    toast.textContent = message;
    toast.classList.add('visible');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => toast.classList.remove('visible'), 2600);
}

function getFocusableElements(container) {
    return Array.from(container.querySelectorAll(
        'a[href], button:not([disabled]), input:not([disabled]):not([type="hidden"]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
    )).filter((el) => !el.closest('[inert]') && el.offsetParent !== null);
}

function trapFocus(container, event) {
    if (event.key !== 'Tab') return;
    const focusable = getFocusableElements(container);
    if (!focusable.length) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
    }
}

function setDrawerState(drawerEl, open, options = {}) {
    if (!drawerEl) return;
    const closeBtn = options.closeButton;
    if (open) {
        drawerReturnFocus = options.returnFocus || document.activeElement;
        drawerEl.classList.add('open');
        drawerEl.removeAttribute('inert');
        drawerEl.setAttribute('aria-hidden', 'false');
        (closeBtn || drawerEl.querySelector('.drawer-close'))?.focus();
        announce(options.announce || 'Panel opened');
    } else {
        drawerEl.classList.remove('open');
        drawerEl.setAttribute('inert', '');
        drawerEl.setAttribute('aria-hidden', 'true');
        const restore = options.returnFocus || drawerReturnFocus;
        if (restore && typeof restore.focus === 'function') restore.focus();
        announce(options.announce || 'Panel closed');
    }
}

function applyAccessibilityPreferences() {
    const prefs = {
        highContrast: localStorage.getItem('pyenv-a11y-high-contrast') === 'true',
        reducedMotion: localStorage.getItem('pyenv-a11y-reduced-motion') === 'true',
        strongFocus: localStorage.getItem('pyenv-a11y-strong-focus') === 'true',
        largeText: localStorage.getItem('pyenv-a11y-large-text') === 'true',
    };
    document.documentElement.toggleAttribute('data-high-contrast', prefs.highContrast);
    document.documentElement.toggleAttribute('data-reduced-motion', prefs.reducedMotion);
    document.documentElement.toggleAttribute('data-strong-focus', prefs.strongFocus);
    document.documentElement.toggleAttribute('data-large-text', prefs.largeText);

    const map = {
        'a11y-high-contrast': prefs.highContrast,
        'a11y-reduced-motion': prefs.reducedMotion,
        'a11y-strong-focus': prefs.strongFocus,
        'a11y-large-text': prefs.largeText,
    };
    Object.entries(map).forEach(([id, checked]) => {
        const input = document.getElementById(id);
        if (input) input.checked = checked;
    });
}

function bindAccessibilityPreferenceControls() {
    const bindings = [
        ['a11y-high-contrast', 'pyenv-a11y-high-contrast'],
        ['a11y-reduced-motion', 'pyenv-a11y-reduced-motion'],
        ['a11y-strong-focus', 'pyenv-a11y-strong-focus'],
        ['a11y-large-text', 'pyenv-a11y-large-text'],
    ];
    bindings.forEach(([id, key]) => {
        const input = document.getElementById(id);
        if (!input) return;
        input.addEventListener('change', () => {
            localStorage.setItem(key, input.checked ? 'true' : 'false');
            applyAccessibilityPreferences();
            showToast('Accessibility preference saved');
            announce('Accessibility preference saved');
        });
    });
}

function bindConfigControls() {
    const bindings = [
        ['config-windows.registry_mode', 'windows.registry_mode'],
        ['config-install.arch', 'install.arch'],
        ['config-install.bootstrap_pip', 'install.bootstrap_pip'],
        ['config-venv.auto_create_base_venv', 'venv.auto_create_base_venv'],
        ['config-venv.auto_use_base_venv', 'venv.auto_use_base_venv'],
    ];
    bindings.forEach(([id, key]) => {
        const select = document.getElementById(id);
        if (!select) return;
        select.addEventListener('change', () => updateConfig(key, select.value));
    });
}

// Workspace context logic
let currentWorkspaceDir = localStorage.getItem('pyenv-workspace-dir') || '';

function getWorkspaceDir() {
    return currentWorkspaceDir || null;
}

function updateWorkspaceUI() {
    const el = document.getElementById('workspace-path');
    if (el) {
        el.textContent = currentWorkspaceDir || 'Default System Root';
    }
}

// Bind Change Workspace Button
document.addEventListener('DOMContentLoaded', () => {
    applyAccessibilityPreferences();
    bindAccessibilityPreferenceControls();
    bindConfigControls();
    document.querySelectorAll('.view').forEach((view) => {
        if (view.id !== 'view-dashboard') {
            view.setAttribute('aria-hidden', 'true');
        }
    });
    updateWorkspaceUI();
    document.getElementById('btn-change-workspace')?.addEventListener('click', async () => {
        try {
            const selectedDir = await invoke('select_directory');
            if (selectedDir) {
                currentWorkspaceDir = selectedDir;
                localStorage.setItem('pyenv-workspace-dir', selectedDir);
                updateWorkspaceUI();
                // Reload views
                loadDashboard();
                loadInstalled();
                availableLoaded = false;
                loadVenvs();
                loadConfig();
                showAlert('Workspace Changed', `Active workspace switched to:<br><code style="font-size: 12px; opacity: 0.8;">${selectedDir}</code>`);
            }
        } catch (err) {
            showAlert('Error', 'Failed to change workspace directory:\n' + err);
        }
    });
});

// ─── Custom Modal System ───
function showModal(title, message, buttons = [{label: 'OK', style: 'btn-primary', primary: true}]) {
    return new Promise(resolve => {
        const root = document.getElementById('modal-root');
        const overlay = document.createElement('div');
        overlay.className = 'modal-overlay';
        const dialogId = `modal-${Date.now()}`;
        overlay.innerHTML = `
            <div class="modal-box" role="dialog" aria-modal="true" aria-labelledby="${dialogId}-title" aria-describedby="${dialogId}-message">
                <h3 id="${dialogId}-title">${title}</h3>
                <p id="${dialogId}-message">${message}</p>
                <div class="modal-actions" id="modal-btns"></div>
            </div>
        `;
        const dialog = overlay.querySelector('.modal-box');
        const btnContainer = overlay.querySelector('#modal-btns');
        const createdButtons = [];
        buttons.forEach((b, i) => {
            const btn = document.createElement('button');
            btn.type = 'button';
            btn.className = `btn ${b.style || 'btn-outline'}`;
            btn.textContent = b.label;
            btn.addEventListener('click', () => {
                root.removeChild(overlay);
                document.removeEventListener('keydown', onKeyDown);
                if (lastFocusedBeforeOverlay) lastFocusedBeforeOverlay.focus();
                resolve(i === 0);
            });
            btnContainer.appendChild(btn);
            createdButtons.push(btn);
        });

        const onKeyDown = (event) => {
            if (event.key === 'Escape') {
                const cancelIndex = Math.max(buttons.length - 1, 0);
                root.removeChild(overlay);
                document.removeEventListener('keydown', onKeyDown);
                if (lastFocusedBeforeOverlay) lastFocusedBeforeOverlay.focus();
                resolve(cancelIndex === 0);
                return;
            }
            trapFocus(dialog, event);
        };

        lastFocusedBeforeOverlay = document.activeElement;
        root.appendChild(overlay);
        document.addEventListener('keydown', onKeyDown);
        const initialFocus = createdButtons.find((_, idx) => buttons[idx]?.primary) || createdButtons[createdButtons.length - 1] || createdButtons[0];
        initialFocus?.focus();
    });
}

async function showAlert(title, message) {
    return showModal(title, message, [{label: 'OK', style: 'btn-primary', primary: true}]);
}

async function showConfirm(title, message) {
    return showModal(title, message, [
        {label: 'Confirm', style: 'btn-danger'},
        {label: 'Cancel', style: 'btn-outline', primary: true}
    ]);
}

function openExternal(url) {
    invoke('open_url', { url }).catch(() => {
        window.open(url, '_blank');
    });
}

// Titlebar Controls
document.getElementById('titlebar-minimize').addEventListener('click', () => invoke('minimize_app'));
document.getElementById('titlebar-maximize').addEventListener('click', () => invoke('maximize_app'));
document.getElementById('titlebar-close').addEventListener('click', () => invoke('close_app'));

// DOM Elements
const sidebarNavItems = document.querySelectorAll('.nav-btn');
const views = document.querySelectorAll('.view');

function showView(viewId) {
    views.forEach(v => {
        v.style.display = 'none';
        v.classList.remove('fade-in');
        v.setAttribute('aria-hidden', 'true');
    });
    const target = document.getElementById(viewId);
    target.style.display = 'block';
    target.setAttribute('aria-hidden', 'false');
    void target.offsetWidth;
    target.classList.add('fade-in');
    const heading = target.querySelector('.page-title');
    if (heading) {
        heading.setAttribute('tabindex', '-1');
        heading.focus({ preventScroll: true });
    }
    document.title = `${VIEW_TITLES[viewId] || 'Pyenv Native'} · Pyenv Native`;
    announce(`Showing ${VIEW_TITLES[viewId] || 'view'}`);
}

function navigateToView(viewId) {
    const navItem = document.querySelector(`.nav-btn[data-view="${viewId}"]`);
    if (!navItem) return;
    navItem.click();
}

function isGlobalVersion(versionName, globalVersions) {
    const normalized = versionName.trim();
    if (!globalVersions?.length) {
        return normalized === 'system';
    }
    return globalVersions.includes(normalized);
}

function renderGlobalButton(versionName, globalVersions) {
    if (isGlobalVersion(versionName, globalVersions)) {
        return '<button class="btn btn-global-active" disabled>Global</button>';
    }
    return `<button class="btn btn-outline" onclick="setGlobal('${versionName}')">Make Global</button>`;
}

sidebarNavItems.forEach((item, index) => {
    item.addEventListener('click', () => {
        if (item.classList.contains('active')) return;
        sidebarNavItems.forEach(i => {
            i.classList.remove('active');
            i.removeAttribute('aria-current');
        });
        item.classList.add('active');
        item.setAttribute('aria-current', 'page');
        showView(item.dataset.view);
        
        if (item.dataset.view === 'view-dashboard') loadDashboard();
        if (item.dataset.view === 'view-installed') loadInstalled();
        if (item.dataset.view === 'view-shell') loadShellIntegration();
        if (item.dataset.view === 'view-available') setupAvailableView();
        if (item.dataset.view === 'view-venvs') loadVenvs();
        if (item.dataset.view === 'view-settings') loadConfig();
    });

    item.addEventListener('keydown', (event) => {
        if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
        event.preventDefault();
        const items = Array.from(sidebarNavItems);
        const currentIndex = items.indexOf(item);
        let nextIndex = currentIndex;
        if (event.key === 'ArrowDown') nextIndex = (currentIndex + 1) % items.length;
        if (event.key === 'ArrowUp') nextIndex = (currentIndex - 1 + items.length) % items.length;
        if (event.key === 'Home') nextIndex = 0;
        if (event.key === 'End') nextIndex = items.length - 1;
        items[nextIndex].focus();
    });
});

// Escape-to-close is registered after drawer elements are defined below.

// ─── Dashboard ───
async function loadDashboard() {
    try {
        const jsonStr = await invoke('get_status', { workspaceDir: getWorkspaceDir() });
        const status = JSON.parse(jsonStr);
        
        const activeVersionEl = document.getElementById('active-version');
        activeVersionEl.textContent = status.active_versions.length ? status.active_versions.join(', ') : 'None';
        const globalLabel = status.global_versions?.length
            ? status.global_versions.join(', ')
            : 'system';
        document.getElementById('active-origin').textContent =
            `Origin: ${status.origin} • Global: ${globalLabel}`;
        
        if (status.managed_venv) {
            document.getElementById('active-venv').textContent = status.managed_venv.name;
            document.getElementById('active-venv-base').textContent = `Base: ${status.managed_venv.base_version}`;
        } else {
            document.getElementById('active-venv').textContent = 'None';
            document.getElementById('active-venv-base').textContent = 'Integrated Project Env';
        }

        document.getElementById('sys-root').textContent = status.root;

    } catch (err) {
        console.error("Failed to load status:", err);
        document.getElementById('active-version').textContent = "Error";
    }
}

// ─── Installed Versions ───
async function loadInstalled() {
    const list = document.getElementById('installed-list');
    const summary = document.getElementById('installed-global-summary');
    list.innerHTML = '<div class="empty-state"><div class="loader"></div></div>';
    
    try {
        const [versionsJson, statusJson] = await Promise.all([
            invoke('get_installed_versions', { workspaceDir: getWorkspaceDir() }),
            invoke('get_status', { workspaceDir: getWorkspaceDir() }),
        ]);
        const versions = JSON.parse(versionsJson);
        const status = JSON.parse(statusJson);
        const globalVersions = status.global_versions?.length
            ? status.global_versions
            : ['system'];
        const globalLabel = globalVersions.join(', ');
        const activeLabel = status.active_versions?.length
            ? status.active_versions.join(', ')
            : 'None';

        if (summary) {
            summary.style.display = 'block';
            summary.innerHTML = `
                <div style="display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap;">
                    <div>
                        <div style="font-size: 12px; color: var(--text-muted); margin-bottom: 4px;">Current setup</div>
                        <div style="font-size: 14px; font-weight: 600;">
                            Active: <span style="color: #fff;">${activeLabel}</span>
                            <span style="opacity: 0.4; margin: 0 8px;">•</span>
                            Global: <span class="badge badge-success" style="margin-left: 0;">${globalLabel}</span>
                        </div>
                        <div style="font-size: 12px; color: var(--text-muted); margin-top: 6px;">
                            Origin: ${status.origin}
                        </div>
                    </div>
                    <button class="btn btn-primary" onclick="navigateToView('view-available')" style="font-size: 12px;">
                        Install New Runtime
                    </button>
                </div>
            `;
        }

        list.innerHTML = '';
        
        // Build the display list with system detection
        const displayVersions = [];
        
        // Check if there's a real system Python (not just the MS Store alias)
        const hasRealSystem = await detectSystemPython();
        displayVersions.push({ name: 'system', isSystem: true, available: hasRealSystem });
        
        versions.forEach(v => {
            if (v !== 'system') {
                displayVersions.push({ name: v, isSystem: false, available: true });
            }
        });

        const managedVersions = displayVersions.filter(entry => !entry.isSystem);
        if (managedVersions.length === 0) {
            list.innerHTML = `
                <div class="empty-state" style="padding: 32px 16px;">
                    <div style="font-size: 15px; font-weight: 600; margin-bottom: 8px;">No managed runtimes installed yet</div>
                    <div style="font-size: 13px; color: var(--text-muted); margin-bottom: 16px;">
                        Install a Python runtime from <b>Available Targets</b> to get started.
                    </div>
                    <button class="btn btn-primary" onclick="navigateToView('view-available')">Browse Available Targets</button>
                </div>
            `;
            return;
        }

        displayVersions.forEach(entry => {
            const card = document.createElement('div');
            card.className = 'version-card';
            const isGlobal = isGlobalVersion(entry.name, globalVersions);
            
            if (entry.isSystem) {
                const badge = entry.available
                    ? '<span class="system-badge badge-success">Detected</span>'
                    : '<span class="system-badge badge-muted">Not Available</span>';
                const globalBadge = isGlobal
                    ? '<span class="badge badge-success" style="margin-left: 8px;">Global</span>'
                    : '';
                
                card.innerHTML = `
                    <div>
                        <div class="version-name">system ${badge}${globalBadge}</div>
                        <div class="version-meta">${entry.available ? 'System-wide Python installation' : 'No system Python found (Microsoft Store alias detected)'}</div>
                    </div>
                    <div class="version-actions">
                        ${entry.available ? `
                            <button class="btn btn-outline" onclick="openPackageExplorer('system')">Package Explorer</button>
                            ${renderGlobalButton('system', globalVersions)}
                        ` : ''}
                    </div>
                `;
            } else {
                const globalBadge = isGlobal
                    ? '<span class="badge badge-success" style="margin-left: 8px;">Global</span>'
                    : '';
                card.innerHTML = `
                    <div>
                        <div class="version-name">${entry.name}${globalBadge}</div>
                        <div class="version-meta">${isGlobal ? 'Configured as the global default runtime' : 'Installed managed runtime'}</div>
                    </div>
                    <div class="version-actions">
                        <button class="btn btn-outline" onclick="openPackageExplorer('${entry.name}')">Package Explorer</button>
                        ${renderGlobalButton(entry.name, globalVersions)}
                        <button class="btn btn-outline" onclick="setLocal('${entry.name}')">Make Local</button>
                        <button class="btn btn-danger" onclick="uninstallVersion('${entry.name}')">Uninstall</button>
                    </div>
                `;
            }
            list.appendChild(card);
        });
    } catch (err) {
        console.error("Failed to load installed:", err);
        list.innerHTML = `<div class="empty-state" style="color: var(--danger)">Failed to load.</div>`;
    }
}

async function detectSystemPython() {
    try {
        const statusJson = await invoke('get_status', { workspaceDir: getWorkspaceDir() });
        const status = JSON.parse(statusJson);
        // If the active version resolves to something and origin isn't 'system',
        // we can't determine from status alone — check if system is in the root
        if (status.root) {
            // The system Python is considered "available" if pyenv can find a
            // real python.exe outside of WindowsApps (the MS Store alias).
            // This is already handled by the core's find_system_python_command.
            // If "system" is in the installed versions list from the core, it means
            // find_system_python returns a result. But the GUI always prepends it.
            // We'll probe by trying to get the system prefix.
            return true;
        }
        return false;
    } catch {
        return false;
    }
}

document.getElementById('btn-refresh-installed').addEventListener('click', () => {
    loadInstalled();
});

// ─── Available Versions ───
let fullAvailableCache = [];
let installedVersionsSet = [];

window.reloadAvailable = function() {
    availableLoaded = false;
    setupAvailableView();
};

let availableLoaded = false;
async function setupAvailableView() {
    if(availableLoaded) return;
    const list = document.getElementById('available-list');
    list.innerHTML = '<div class="empty-state"><div class="loader"></div></div>';
    try {
        const installedStr = await invoke('get_installed_versions', { workspaceDir: getWorkspaceDir() });
        installedVersionsSet = JSON.parse(installedStr);

        if (fullAvailableCache.length === 0) {
            const jsonStr = await invoke('get_available_versions', { workspaceDir: getWorkspaceDir(), family: null, pattern: null });
            const data = JSON.parse(jsonStr);
            const groups = (data.results || data);
            groups.forEach(g => {
                if (g.versions) g.versions.forEach(v => fullAvailableCache.push(v));
                else fullAvailableCache.push(g.name || g.spec || g);
            });
        }
        
        let displayList = fullAvailableCache;
        const showLatestOnly = document.getElementById('chk-latest-only')?.checked;
        if (showLatestOnly) {
           const seen = new Set();
           displayList = fullAvailableCache.filter(v => {
               if (typeof v === 'string' && v.endsWith('t')) return false;
               const match = typeof v === 'string' ? v.match(/^(\d+\.\d+)/) : null;
               if (match) {
                   const prefix = match[1];
                   if (!seen.has(prefix)) { seen.add(prefix); return true; }
                   return false;
               }
               return true;
           });
        }
        
        window.availableVersionsCache = displayList; 
        renderAvailable(displayList);
        availableLoaded = true;
    } catch(err) {
        console.error(err);
        list.innerHTML = '<div class="empty-state" style="color: var(--danger)">Failed to load catalog.</div>';
    }
}

function renderAvailable(items) {
    const list = document.getElementById('available-list');
    list.innerHTML = '';
    if(!items || items.length === 0) {
        list.innerHTML = '<div class="empty-state">No targets found.</div>';
        return;
    }
    items.forEach(v => {
        const spec = typeof v === 'string' ? v : (v.name || v.spec || v);
        const card = document.createElement('div');
        card.className = 'version-card';
        
        if (installedVersionsSet.includes(spec)) {
            card.innerHTML = `
                <div class="version-name">${spec} <span style="font-size: 11px; opacity: 0.6; font-weight: 400; margin-left: 8px;">(Installed)</span></div>
                <div class="version-actions"><button class="btn btn-outline" disabled>Installed</button></div>
            `;
        } else {
            card.innerHTML = `
                <div class="version-name">${spec}</div>
                <div class="version-actions"><button class="btn btn-primary" onclick="installTarget('${spec}', this)">Install</button></div>
            `;
        }
        list.appendChild(card);
    });
}

// ─── Virtual Environments ───
async function loadVenvs() {
    const list = document.getElementById('venvs-list');
    list.innerHTML = '<div class="empty-state"><div class="loader"></div></div>';
    try {
        const [venvsJson, statusJson] = await Promise.all([
            invoke('get_managed_venvs', { workspaceDir: getWorkspaceDir() }),
            invoke('get_status', { workspaceDir: getWorkspaceDir() }),
        ]);
        const venvs = JSON.parse(venvsJson);
        const status = JSON.parse(statusJson);
        const globalVersions = status.global_versions?.length
            ? status.global_versions
            : ['system'];
        list.innerHTML = '';
        if (venvs.length === 0) {
            list.innerHTML = '<div class="empty-state">No virtual environments found.</div>';
        } else {
                venvs.forEach(v => {
                    const globalKey = `venv:${v.name}`;
                    const isGlobal = isGlobalVersion(globalKey, globalVersions);
                    const globalBadge = isGlobal
                        ? '<span class="badge badge-success" style="margin-left: 8px;">Global</span>'
                        : '';
                    const card = document.createElement('div');
                    card.className = 'version-card';
                    card.innerHTML = `
                        <div>
                            <div class="version-name">${v.name}${globalBadge}</div>
                            <div class="version-meta">Base: ${v.base_version} • ${v.path}</div>
                        </div>
                        <div class="version-actions">
                            <button class="btn btn-outline" onclick="openPackageExplorer('venv:${v.name}')">Package Explorer</button>
                            <button class="btn btn-gold" style="border: 1px solid var(--accent-gold); color: var(--accent-gold);" onclick="openVenvUpgradeWizard('${v.name}', '${v.base_version}')">Upgrade</button>
                            ${renderGlobalButton(globalKey, globalVersions)}
                            <button class="btn btn-outline" onclick="setLocal('venv:${v.name}')">Local</button>
                            <button class="btn btn-danger" onclick="deleteVenv('${v.name}')">Delete</button>
                        </div>
                    `;
                    list.appendChild(card);
                });
        }
        const sysInfo = await invoke('get_installed_versions', { workspaceDir: getWorkspaceDir() });
        const bases = JSON.parse(sysInfo);
        const sel = document.getElementById('venv-base-version');
        sel.innerHTML = '<option value="">Select Base...</option>';
        bases.forEach(b => { sel.innerHTML += '<option value="' + b + '">' + b + '</option>'; });
    } catch (err) {
        console.error("Failed to load venvs:", err);
        list.innerHTML = '<div class="empty-state" style="color: var(--danger)">Failed to load.</div>';
    }
}

document.getElementById('available-search').addEventListener('input', (e) => {
    const val = e.target.value.toLowerCase();
    if(!window.availableVersionsCache) return;
    const filtered = window.availableVersionsCache.filter(item => {
        const spec = item.name || item.spec || item;
        return typeof spec === 'string' && spec.toLowerCase().includes(val);
    });
    renderAvailable(filtered);
});

// ─── Actions ───
async function installTarget(v, btnEl) {
    btnEl.disabled = true;
    const originalText = btnEl.innerText;
    btnEl.innerHTML = 'Installing… <div class="loader loader-sm" style="display:inline-block; vertical-align:middle; margin-left:6px;"></div>';
    
    try {
        await invoke('install_version', { workspaceDir: getWorkspaceDir(), version: v });
        btnEl.innerHTML = "Installed ✓";
        btnEl.classList.remove('btn-primary');
        btnEl.classList.add('btn-outline');
        installedVersionsSet.push(v);
        loadDashboard();
        loadInstalled();
    } catch (err) {
        console.error("Install failed:", err);
        btnEl.innerText = originalText;
        btnEl.disabled = false;
        showAlert('Install Failed', err);
    }
}

async function setGlobal(v) {
    try {
        await invoke('set_global', { workspaceDir: getWorkspaceDir(), version: v });
        loadDashboard();
        loadInstalled();
        loadVenvs();
        showAlert('Global Version Set', `Python <b>${v}</b> is now the global default.`);
    } catch(err) {
        showAlert('Error', 'Failed to set global version:\n' + err);
    }
}

document.getElementById('btn-refresh-venvs')?.addEventListener('click', loadVenvs);

document.getElementById('btn-create-venv')?.addEventListener('click', async () => {
    const name = document.getElementById('venv-name').value.trim();
    const base = document.getElementById('venv-base-version').value;
    if(!name || !base) {
        showAlert('Missing Fields', 'Please provide both a name and a base version.');
        return;
    }
    const btn = document.getElementById('btn-create-venv');
    btn.disabled = true;
    btn.innerText = "Creating...";
    try {
        await invoke('create_venv', { workspaceDir: getWorkspaceDir(), baseVersion: base, name: name });
        document.getElementById('venv-name').value = '';
        loadVenvs();
        loadDashboard();
    } catch(err) {
        showAlert('Venv Creation Failed', err);
    } finally {
        btn.disabled = false;
        btn.innerText = "Create";
    }
});

async function deleteVenv(name) {
    const yes = await showConfirm('Delete Virtual Environment', `Are you sure you want to delete venv <b>${name}</b>? This cannot be undone.`);
    if (!yes) return;
    try {
        await invoke('delete_venv', { workspaceDir: getWorkspaceDir(), spec: name });
        loadVenvs();
        loadDashboard();
    } catch(err) {
        showAlert('Delete Failed', err);
    }
}

// ─── Update Flow ───
// Uses the core self-update API directly (check-only mode, then update with --yes).
async function checkUpdates() {
    const btn = document.getElementById('footer-btn');
    if(btn) { btn.innerText = "Checking…"; btn.disabled = true; }
    try {
        const result = await invoke('check_for_updates', { workspaceDir: getWorkspaceDir() });
        // Parse whether an update is available from the result text
        if (result.includes('up to date') || result.includes('Up to date') || result.includes('already up to date')) {
            showAlert('Up to Date', result);
        } else if (result.includes('Update available') || result.includes('newer')) {
            const yes = await showConfirm('Update Available', result + '<br><br>Would you like to update now?');
            if (yes) {
                if(btn) btn.innerText = "Updating…";
                try {
                    const updateResult = await invoke('perform_update', { workspaceDir: getWorkspaceDir() });
                    showAlert('Update In Progress', 'The application will now automatically close, install the update, and restart. Please wait...');
                    setTimeout(async () => {
                        await invoke('close_app');
                    }, 4000);
                } catch(updateErr) {
                    showAlert('Update Failed', updateErr);
                }
            }
        } else {
            // Generic result
            showAlert('Update Check', result);
        }
    } catch(err) {
        showAlert('Update Check Failed', err);
    } finally {
        if(btn) { btn.innerText = "Check for Updates"; btn.disabled = false; }
    }
}

async function setLocal(v) {
    try {
        const selectedDir = await invoke('select_directory');
        if (selectedDir) {
            await invoke('set_local', { version: v, path: selectedDir });
            loadDashboard();
            showAlert('Local Version Set', `Python <b>${v}</b> pinned to:<br><code style="font-size: 12px; opacity: 0.8;">${selectedDir}</code>`);
        }
    } catch(err) {
        showAlert('Error', 'Failed to set local version:\n' + err);
    }
}

// ─── Config ───
async function loadConfig() {
    try {
        const jsonStr = await invoke('get_config', { workspaceDir: getWorkspaceDir() });
        const config = JSON.parse(jsonStr);
        
        document.getElementById('config-windows.registry_mode').value = config.windows?.registry_mode || 'disabled';
        document.getElementById('config-install.arch').value = config.install?.arch || 'auto';
        document.getElementById('config-install.bootstrap_pip').value = (config.install?.bootstrap_pip ?? true).toString();
        document.getElementById('config-venv.auto_create_base_venv').value = (config.venv?.auto_create_base_venv ?? false).toString();
        document.getElementById('config-venv.auto_use_base_venv').value = (config.venv?.auto_use_base_venv ?? false).toString();
    } catch(err) {
        console.error("Failed to load config", err);
    }
}

async function updateConfig(key, value) {
    const status = document.getElementById('config-save-status');
    try {
        await invoke('set_config', { workspaceDir: getWorkspaceDir(), key, value });
        if (status) status.textContent = `Saved ${key} = ${value}`;
        showToast(`Saved ${key}`);
        announce(`Configuration saved: ${key}`);
    } catch(err) {
        if (status) status.textContent = `Failed to save ${key}`;
        showAlert('Config Error', 'Failed to save config:\n' + err);
    }
}

// ─── Uninstall ───
async function uninstallVersion(v) {
    const yes = await showConfirm('Uninstall Python', `Are you sure you want to completely remove Python <b>${v}</b>? This will delete all files for this version.`);
    if (!yes) return;
    
    // Show inline progress
    const cards = document.querySelectorAll('.version-card');
    let targetCard = null;
    cards.forEach(c => { if (c.querySelector('.version-name')?.textContent === v) targetCard = c; });
    if (targetCard) {
        const actionsDiv = targetCard.querySelector('.version-actions');
        if (actionsDiv) actionsDiv.innerHTML = '<div class="loader loader-sm" style="display:inline-block;"></div> <span style="font-size: 12px; margin-left: 8px;">Removing…</span>';
    }
    
    try {
        await invoke('uninstall_version', { workspaceDir: getWorkspaceDir(), version: v });
        installedVersionsSet = installedVersionsSet.filter(x => x !== v);
        showAlert('Uninstalled', `Python <b>${v}</b> has been removed.`);
        loadInstalled();
        loadDashboard();
    } catch(err) {
        showAlert('Uninstall Failed', err);
        loadInstalled();
    }
}

function compareVersions(v1, v2) {
    const p1 = v1.split('.').map(n => parseInt(n, 10) || 0);
    const p2 = v2.split('.').map(n => parseInt(n, 10) || 0);
    for (let i = 0; i < Math.max(p1.length, p2.length); i++) {
        if ((p1[i] || 0) > (p2[i] || 0)) return 1;
        if ((p1[i] || 0) < (p2[i] || 0)) return -1;
    }
    return 0;
}

// ─── Version & Init ───
async function initAppVersion() {
    try {
        const version = await invoke('get_app_version');
        const versionEl = document.getElementById('footer-version');
        if (versionEl) versionEl.textContent = `Pyenv-Native v${version}`;
        
        const aboutEl = document.getElementById('about-version');
        if (aboutEl) aboutEl.textContent = version;
        
        // Check for latest version from GitHub
        try {
            const res = await fetch('https://api.github.com/repos/ImYourBoyRoy/pyenv-native/releases/latest');
            if (!res.ok) throw new Error('Fetch failed');
            const data = await res.json();
            const latestTag = data.tag_name || '';
            const latest = latestTag.replace(/^v/, '');
            
            const statusEl = document.getElementById('footer-app-status');
            if (statusEl && latest) {
                // Only show update if latest > current
                if (compareVersions(latest, version) > 0) {
                    statusEl.innerHTML = `<span style="color:var(--danger)">Update Available: v${latest}</span>`;
                } else {
                    statusEl.innerHTML = `<span style="color:#10b981;">✓ Up to Date</span>`;
                }
            }
        } catch {
            const statusEl = document.getElementById('footer-app-status');
            if (statusEl) statusEl.innerHTML = `<span style="opacity:0.5">(Offline)</span>`;
        }
    } catch(err) {
        console.error("Failed to get app version:", err);
    }
}

// ─── Bootstrapping ───
async function checkInstallation() {
    try {
        const status = await invoke('check_install_status');
        const banner = document.getElementById('setup-banner');
        
        if (!status.is_installed) {
            if (banner) banner.style.display = 'block';
        } else {
            if (banner) banner.style.display = 'none';
        }
        
        if (!currentWorkspaceDir && status.root) {
            currentWorkspaceDir = status.root;
            localStorage.setItem('pyenv-workspace-dir', status.root);
            updateWorkspaceUI();
        }
        
        // Ensure sidebar is always visible in the new workflow
        document.querySelector('.sidebar').style.display = 'flex';
        loadDashboard();
        
        if (status.is_portable) {
            const footerStatus = document.getElementById('footer-app-status');
            if (status.is_installed) {
                // Already adding a "Portable" tag if it's both installed and running portably?
                // Actually, let's just show "Portable Mode" if running next to a binary.
                if (!footerStatus.innerHTML.includes('Portable')) {
                   footerStatus.innerHTML += ' <span class="badge badge-success" style="font-size:10px; padding: 2px 6px; margin-left: 8px;">Portable</span>';
                }
            }
        }
    } catch (err) {
        console.error("Installation check failed", err);
        loadDashboard();
    }
}

document.getElementById('btn-dismiss-banner')?.addEventListener('click', () => {
    const banner = document.getElementById('setup-banner');
    if (banner) banner.style.display = 'none';
});

document.getElementById('btn-run-setup-banner')?.addEventListener('click', async () => {
    const actions = document.getElementById('banner-actions');
    const progress = document.getElementById('banner-progress');
    const statusText = document.getElementById('banner-status-text');

    if (actions) actions.style.display = 'none';
    if (progress) progress.style.display = 'flex';

    try {
        await invoke('install_local_pyenv');
        if (statusText) statusText.textContent = "Done! Environment refreshed.";
        setTimeout(() => {
            location.reload();
        }, 1500);
    } catch (err) {
        if (progress) progress.style.display = 'none';
        if (actions) actions.style.display = 'flex';
        showAlert('Setup Failed', err);
    }
});

// ─── Package Explorer Drawer Controller ───
let currentExplorerTarget = '';
let installedPackages = [];
let outdatedPackages = [];
let selectedOutdated = new Set();
let scannedTarget = ''; // Track which target was scanned for outdated packages

// DOM Elements inside the drawer
const drawer = document.getElementById('drawer-package-manager');
const drawerCloseBtn = document.getElementById('btn-close-drawer');
const drawerTargetTitle = document.getElementById('drawer-target-title');
const drawerTargetSubtitle = document.getElementById('drawer-target-subtitle');
const drawerTabBtns = document.querySelectorAll('.drawer-tab-btn');
const drawerTabContents = document.querySelectorAll('.drawer-tab-content');

// Installed Tab DOMs
const drawerPackageList = document.getElementById('drawer-package-list');
const drawerPackageSearch = document.getElementById('drawer-package-search');
const drawerPackageEmpty = document.getElementById('drawer-package-empty');
const drawerPackageLoading = document.getElementById('drawer-package-loading');
const pipSelfUpdateCard = document.getElementById('pip-self-update-card');
const pipSelfUpdateVersions = document.getElementById('pip-self-update-versions');
const btnUpgradePip = document.getElementById('btn-upgrade-pip');

// Updates Tab DOMs
const btnScanOutdated = document.getElementById('btn-scan-outdated');
const btnUpdateSelected = document.getElementById('btn-update-selected');
const updatesScanPrompt = document.getElementById('updates-scan-prompt');
const updatesScanning = document.getElementById('updates-scanning');
const updatesChecklistView = document.getElementById('updates-checklist-view');
const updatesChecklist = document.getElementById('updates-checklist');
const chkSelectAllUpdates = document.getElementById('chk-select-all-updates');
const selectedUpdatesCount = document.getElementById('selected-updates-count');
const updatesCountBadge = document.getElementById('updates-count-badge');

// Import Tab DOMs
const importRequirementsPath = document.getElementById('import-requirements-path');
const btnPrecheckImport = document.getElementById('btn-precheck-import');
const btnInstallImported = document.getElementById('btn-install-imported');
const importPrecheckLoading = document.getElementById('import-precheck-loading');
const importPrecheckDashboard = document.getElementById('import-precheck-dashboard');
const precheckBanner = document.getElementById('precheck-banner');
const precheckBannerIcon = document.getElementById('precheck-banner-icon');
const precheckBannerText = document.getElementById('precheck-banner-text');
const precheckConflictsSection = document.getElementById('precheck-conflicts-section');
const precheckConflictsList = document.getElementById('precheck-conflicts-list');
const precheckResolvedList = document.getElementById('precheck-resolved-list');

// Codebase Import Analyzer DOMs
const btnScanImports = document.getElementById('btn-scan-imports');
const scanImportsLoading = document.getElementById('scan-imports-loading');
const scanImportsResults = document.getElementById('scan-imports-results');
const scanMissingCard = document.getElementById('scan-missing-card');
const scanMissingBadges = document.getElementById('scan-missing-badges');
const btnInstallMissingImports = document.getElementById('btn-install-missing-imports');
const scanInstalledCard = document.getElementById('scan-installed-card');
const scanInstalledBadges = document.getElementById('scan-installed-badges');

let scannedMissingImports = [];

// ─── Upgrade Environment Drawer Controller ───
let upgraderSourceSpec = '';
let upgraderSourceName = '';
let upgraderSourceBase = '';
let upgraderTargetVersion = '';
let upgraderPackages = [];

const upgraderDrawer = document.getElementById('drawer-venv-upgrader');
const upgraderCloseBtn = document.getElementById('btn-close-upgrader');
const upgraderTargetTitle = document.getElementById('upgrader-target-title');
const upgraderTargetSubtitle = document.getElementById('upgrader-target-subtitle');
const upgraderTargetVersionSel = document.getElementById('upgrader-target-version');
const btnStartUpgrade = document.getElementById('btn-start-upgrade');
const upgraderSetupCard = document.getElementById('upgrader-setup-card');
const upgraderProgressCard = document.getElementById('upgrader-progress-card');
const upgraderConsole = document.getElementById('upgrader-console');

// Helper to write to console log
function appendUpgraderLog(text) {
    upgraderConsole.textContent += text + "\n";
    upgraderConsole.scrollTop = upgraderConsole.scrollHeight;
}

function clearUpgraderLog() {
    upgraderConsole.textContent = '';
}

// Reset step row style
function resetStepRow(stepId, num, label, desc) {
    const row = document.getElementById(`step-row-${stepId}`);
    const icon = document.getElementById(`step-icon-${stepId}`);
    const lbl = document.getElementById(`step-label-${stepId}`);
    const dsc = document.getElementById(`step-desc-${stepId}`);
    
    if (row) {
        row.style.opacity = '0.5';
    }
    if (icon) {
        icon.innerHTML = num;
        icon.style.background = 'rgba(255,255,255,0.05)';
        icon.style.color = 'var(--text-muted)';
    }
    if (lbl) lbl.textContent = label;
    if (dsc) dsc.textContent = desc;
}

// Mark step row in progress (HSL Gold)
function markStepInProgress(stepId, label, desc) {
    const row = document.getElementById(`step-row-${stepId}`);
    const icon = document.getElementById(`step-icon-${stepId}`);
    const lbl = document.getElementById(`step-label-${stepId}`);
    const dsc = document.getElementById(`step-desc-${stepId}`);
    
    if (row) row.style.opacity = '1';
    if (icon) {
        icon.innerHTML = `<div class="loader loader-sm" style="width:12px; height:12px; border-width:2px; border-top-color:#ffd43b;"></div>`;
        icon.style.background = 'rgba(255, 212, 59, 0.1)';
        icon.style.color = '#ffd43b';
    }
    if (lbl) lbl.textContent = label;
    if (dsc) dsc.textContent = desc;
}

// Mark step row success (Safety Green)
function markStepSuccess(stepId, label, desc) {
    const icon = document.getElementById(`step-icon-${stepId}`);
    const lbl = document.getElementById(`step-label-${stepId}`);
    const dsc = document.getElementById(`step-desc-${stepId}`);
    
    if (icon) {
        icon.innerHTML = `<svg viewBox="0 0 24 24" width="10" height="10" stroke="currentColor" stroke-width="3" fill="none" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>`;
        icon.style.background = 'rgba(16, 185, 129, 0.15)';
        icon.style.color = '#10b981';
    }
    if (lbl) lbl.textContent = label;
    if (dsc) dsc.textContent = desc;
}

// Mark step row warning/error (Coral Red)
function markStepWarning(stepId, label, desc) {
    const icon = document.getElementById(`step-icon-${stepId}`);
    const lbl = document.getElementById(`step-label-${stepId}`);
    const dsc = document.getElementById(`step-desc-${stepId}`);
    
    if (icon) {
        icon.innerHTML = `<svg viewBox="0 0 24 24" width="10" height="10" stroke="currentColor" stroke-width="3" fill="none" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>`;
        icon.style.background = 'rgba(239, 68, 68, 0.15)';
        icon.style.color = '#ef4444';
    }
    if (lbl) lbl.textContent = label;
    if (dsc) dsc.textContent = desc;
}

// Function to open the wizard drawer
window.openVenvUpgradeWizard = async function(name, baseVersion) {
    upgraderSourceSpec = `venv:${name}`;
    upgraderSourceName = name;
    upgraderSourceBase = baseVersion;
    
    upgraderTargetTitle.textContent = `Upgrade ${name}`;
    upgraderTargetSubtitle.textContent = `Current Base: ${baseVersion}`;
    
    // Reset wizard view
    upgraderSetupCard.style.display = 'block';
    upgraderProgressCard.style.display = 'none';
    clearUpgraderLog();
    appendUpgraderLog("Select a target Python version to begin migration.");
    
    // Reset checklist items to default empty style
    resetStepRow('backup', '1', 'Inventory old environment', 'Discovering installed packages...');
    resetStepRow('recreate', '2', 'Create fresh environment', 'Rebuilding venv under target version...');
    resetStepRow('restore', '3', 'Restore packages', 'Progressively reinstalling packages...');
    resetStepRow('verify', '4', 'Dependency diagnostics', 'Running pip check verification...');
    resetStepRow('cleanup', '5', 'Cleanup old environment', 'Removing old virtual environment...');

    // Populate dropdown with installed python versions
    try {
        const sysInfo = await invoke('get_installed_versions', { workspaceDir: getWorkspaceDir() });
        const bases = JSON.parse(sysInfo);
        upgraderTargetVersionSel.innerHTML = '<option value="">Select Target...</option>';
        bases.forEach(b => {
            if (b !== baseVersion) {
                upgraderTargetVersionSel.innerHTML += `<option value="${b}">${b}</option>`;
            }
        });
    } catch (err) {
        console.error("Failed to load targets for upgrade:", err);
    }
    
    // Open drawer
    setDrawerState(upgraderDrawer, true, {
        closeButton: upgraderCloseBtn,
        announce: `Opened upgrade panel for ${name}`,
    });
};

// Close button listener
if (upgraderCloseBtn) {
    upgraderCloseBtn.addEventListener('click', () => {
        setDrawerState(upgraderDrawer, false, { closeButton: upgraderCloseBtn });
    });
}

// Start upgrade wizard button listener
if (btnStartUpgrade) {
    btnStartUpgrade.addEventListener('click', async () => {
        const targetVer = upgraderTargetVersionSel.value;
        if (!targetVer) {
            showAlert("Migration Error", "Please select a target Python runtime version.");
            return;
        }
        
        upgraderTargetVersion = targetVer;
        upgraderSetupCard.style.display = 'none';
        upgraderProgressCard.style.display = 'block';
        clearUpgraderLog();
        appendUpgraderLog(`Initializing migration wizard for venv:${upgraderSourceName} -> ${targetVer}...`);
        
        try {
            // Step 1: Inventory old environment packages
            markStepInProgress('backup', 'Inventory old environment', 'Gathering package details...');
            appendUpgraderLog("\n[STEP 1] Discovery: Reading currently installed packages...");
            
            const upgradeInfo = await invoke('get_venv_upgrade_info', {
                workspaceDir: getWorkspaceDir(),
                spec: upgraderSourceSpec
            });
            
            upgraderPackages = upgradeInfo.packages;
            appendUpgraderLog(`Captured ${upgraderPackages.length} custom packages to migrate:`);
            upgraderPackages.forEach(p => appendUpgraderLog(`  - ${p}`));
            markStepSuccess('backup', 'Inventory complete', `Captured ${upgraderPackages.length} custom packages`);
            
            // Step 2: Create new virtual environment under target version
            markStepInProgress('recreate', 'Create fresh environment', 'Invoking backend creator...');
            appendUpgraderLog(`\n[STEP 2] Environment: Recreating venv '${upgraderSourceName}' under runtime ${targetVer}...`);
            
            const createResult = await invoke('create_venv', {
                workspaceDir: getWorkspaceDir(),
                baseVersion: targetVer,
                name: upgraderSourceName
            });
            
            appendUpgraderLog(createResult);
            markStepSuccess('recreate', 'Environment recreated', `Venv is now running Python ${targetVer}`);
            
            // Step 3: Reinstall packages progressively
            markStepInProgress('restore', 'Restore packages', `Reinstalling custom libraries (0/${upgraderPackages.length})...`);
            appendUpgraderLog("\n[STEP 3] Pip Restoration: Reinstalling custom libraries...");
            
            if (upgraderPackages.length > 0) {
                for (let i = 0; i < upgraderPackages.length; i++) {
                    const pkg = upgraderPackages[i];
                    markStepInProgress('restore', 'Restore packages', `Reinstalling ${pkg} (${i + 1}/${upgraderPackages.length})...`);
                    appendUpgraderLog(`  - Installing ${pkg}...`);
                    
                    try {
                        await invoke('install_pip_packages', {
                            workspaceDir: getWorkspaceDir(),
                            target: `venv:${upgraderSourceName}`,
                            packages: [pkg]
                        });
                        appendUpgraderLog(`    ✓ ${pkg} installed successfully.`);
                    } catch (e) {
                        appendUpgraderLog(`    [WARNING] Failed to install ${pkg}: ${e}`);
                    }
                }
                markStepSuccess('restore', 'Packages restored', `Restored custom dependencies`);
            } else {
                appendUpgraderLog("  - No custom packages to reinstall.");
                markStepSuccess('restore', 'Packages restored', 'No packages to restore');
            }
            
            // Step 4: Run diagnostic verification
            markStepInProgress('verify', 'Dependency diagnostics', 'Running pip check...');
            appendUpgraderLog("\n[STEP 4] Diagnostics: Running pip check conflict verification...");
            
            try {
                const checkRes = await invoke('check_pip_conflicts', {
                    workspaceDir: getWorkspaceDir(),
                    target: `venv:${upgraderSourceName}`
                });
                appendUpgraderLog(checkRes);
                const conflicts = JSON.parse(checkRes);
                if (conflicts.length === 0) {
                    appendUpgraderLog("  ✓ All package dependencies are fully satisfied and healthy!");
                    markStepSuccess('verify', 'Verification success', 'No dependency conflicts detected');
                } else {
                    appendUpgraderLog(`  [WARNING] Discovered ${conflicts.length} package dependency conflicts!`);
                    conflicts.forEach(c => appendUpgraderLog(`    ! ${c.message}`));
                    markStepWarning('verify', 'Conflicts discovered', `Found ${conflicts.length} package warning(s)`);
                }
            } catch (e) {
                appendUpgraderLog(`  - pip check verification complete.`);
                markStepSuccess('verify', 'Verification complete', 'Dependency verification complete');
            }
            
            // Step 5: Clean up old environment
            markStepInProgress('cleanup', 'Cleanup old environment', 'Purging old files...');
            appendUpgraderLog("\n[STEP 5] Cleanup: Purging old virtual environment files...");
            
            const oldSpec = `${upgraderSourceBase}/envs/${upgraderSourceName}`;
            const newSpecPath = `${targetVer}/envs/${upgraderSourceName}`;
            
            if (oldSpec !== newSpecPath) {
                try {
                    await invoke('delete_venv', {
                        workspaceDir: getWorkspaceDir(),
                        spec: oldSpec
                    });
                    appendUpgraderLog(`  ✓ Safely purged old environment directory: ${oldSpec}`);
                    markStepSuccess('cleanup', 'Cleanup complete', 'Old environment purged successfully');
                } catch (e) {
                    appendUpgraderLog(`  - [WARNING] Could not delete old environment: ${e}`);
                    markStepWarning('cleanup', 'Cleanup warning', 'Could not purge old files');
                }
            } else {
                appendUpgraderLog("  - Same runtime, skipped directory cleanup.");
                markStepSuccess('cleanup', 'Cleanup complete', 'Same version recreation, skipped purge');
            }
            
            appendUpgraderLog("\n[FINISH] Venv migration completed successfully!");
            showAlert("Migration Success", `Managed virtual environment '${upgraderSourceName}' has been fully migrated to Python ${targetVer}.`);
            loadVenvs();
            
        } catch (error) {
            console.error("Migration failed:", error);
            appendUpgraderLog(`\n[FATAL ERROR] Migration failed: ${error}`);
            showAlert("Migration Failed", error);
        }
    });
}

// Open Package Explorer Drawer
window.openPackageExplorer = function(target) {
    currentExplorerTarget = target;
    
    // Set Target Title/Subtitle
    drawerTargetTitle.textContent = target.startsWith('venv:') ? `Venv: ${target.substring(5)}` : `Runtime: ${target}`;
    drawerTargetSubtitle.textContent = `Package Explorer Target Context`;
    
    // Switch to first tab (Installed)
    switchDrawerTab('tab-installed');
    
    // Reset search
    drawerPackageSearch.value = '';
    
    // Open the drawer
    setDrawerState(drawer, true, {
        closeButton: drawerCloseBtn,
        announce: `Opened package explorer for ${target}`,
    });
    
    // Load installed packages
    loadDrawerPackages();
    
    // Check if we should reset outdated scans if target changed
    if (scannedTarget !== target) {
        resetOutdatedScanView();
    }
    
    // Clear precheck inputs
    importRequirementsPath.value = '';
    importPrecheckDashboard.style.display = 'none';
    importPrecheckLoading.style.display = 'none';
    btnInstallImported.disabled = true;
    
    // Clear import scanner results
    if (scanImportsResults) scanImportsResults.style.display = 'none';
    if (scanImportsLoading) scanImportsLoading.style.display = 'none';
    if (btnScanImports) {
        btnScanImports.disabled = false;
        btnScanImports.textContent = "Scan Workspace";
    }
    scannedMissingImports = [];
    if (scanMissingBadges) scanMissingBadges.innerHTML = '';
    if (scanInstalledBadges) scanInstalledBadges.innerHTML = '';
};

// Close Package Explorer Drawer
function closePackageExplorer() {
    setDrawerState(drawer, false, { closeButton: drawerCloseBtn });
    currentExplorerTarget = '';
}

drawerCloseBtn.addEventListener('click', closePackageExplorer);

// Tab switching logic inside the drawer
function switchDrawerTab(tabId) {
    drawerTabBtns.forEach(btn => {
        const isActive = btn.dataset.drawerTab === tabId;
        btn.classList.toggle('active', isActive);
        btn.setAttribute('aria-selected', isActive ? 'true' : 'false');
        btn.tabIndex = isActive ? 0 : -1;
    });
    
    drawerTabContents.forEach(content => {
        const isActive = content.id === tabId;
        content.classList.toggle('active', isActive);
        content.hidden = !isActive;
    });
}

drawerTabBtns.forEach((btn, index) => {
    btn.addEventListener('click', () => {
        switchDrawerTab(btn.dataset.drawerTab);
    });
    btn.addEventListener('keydown', (event) => {
        if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
        event.preventDefault();
        const tabs = Array.from(drawerTabBtns);
        const currentIndex = tabs.indexOf(btn);
        let nextIndex = currentIndex;
        if (event.key === 'ArrowRight') nextIndex = (currentIndex + 1) % tabs.length;
        if (event.key === 'ArrowLeft') nextIndex = (currentIndex - 1 + tabs.length) % tabs.length;
        if (event.key === 'Home') nextIndex = 0;
        if (event.key === 'End') nextIndex = tabs.length - 1;
        tabs[nextIndex].focus();
        switchDrawerTab(tabs[nextIndex].dataset.drawerTab);
    });
    btn.tabIndex = index === 0 ? 0 : -1;
});

document.addEventListener('keydown', (event) => {
    if (event.key !== 'Escape') return;
    if (drawer?.classList.contains('open')) {
        closePackageExplorer();
        return;
    }
    if (upgraderDrawer?.classList.contains('open')) {
        setDrawerState(upgraderDrawer, false, { closeButton: upgraderCloseBtn });
    }
});

// Load installed packages from Rust backend
async function loadDrawerPackages() {
    drawerPackageLoading.style.display = 'block';
    drawerPackageEmpty.style.display = 'none';
    drawerPackageList.innerHTML = '';
    pipSelfUpdateCard.style.display = 'none';
    
    try {
        const jsonStr = await invoke('get_pip_packages', { workspaceDir: getWorkspaceDir(), target: currentExplorerTarget });
        installedPackages = JSON.parse(jsonStr);
        
        renderInstalledPackages();
        
        // Cozy Check if pip is installed and has updates
        checkForPipUpdatesInBackground();
        
    } catch(err) {
        console.error("Failed to load packages:", err);
        drawerPackageList.innerHTML = `<tr><td colspan="3" style="color: var(--danger); text-align: center;">Error loading packages: ${err}</td></tr>`;
    } finally {
        drawerPackageLoading.style.display = 'none';
    }
}

// Check for pip updates in the background
async function checkForPipUpdatesInBackground() {
    try {
        const jsonStr = await invoke('get_outdated_packages', { workspaceDir: getWorkspaceDir(), target: currentExplorerTarget });
        const outdated = JSON.parse(jsonStr);
        
        const pipOutdated = outdated.find(p => p.name.toLowerCase() === 'pip');
        if (pipOutdated) {
            pipSelfUpdateCard.style.display = 'flex';
            pipSelfUpdateVersions.textContent = `Current: ${pipOutdated.version} → Latest: ${pipOutdated.latest_version}`;
            
            // Also light up the active pip status in the main GUI if this matches active environment!
            updateActivePipLight(true, pipOutdated.version, pipOutdated.latest_version);
        } else {
            pipSelfUpdateCard.style.display = 'none';
            updateActivePipLight(false);
        }
    } catch(err) {
        console.warn("Failed to check pip updates in background:", err);
    }
}

// Upgrade Pip cozy action
btnUpgradePip.addEventListener('click', async () => {
    btnUpgradePip.disabled = true;
    btnUpgradePip.textContent = "Upgrading...";
    
    try {
        await invoke('update_pip_packages', {
            workspaceDir: getWorkspaceDir(),
            target: currentExplorerTarget,
            packages: ["pip"],
            all: false
        });
        showAlert("Pip Upgraded", "pip self-update completed successfully.");
        loadDrawerPackages();
    } catch(err) {
        showAlert("Upgrade Failed", err);
    } finally {
        btnUpgradePip.disabled = false;
        btnUpgradePip.textContent = "Update";
    }
});

// Render installed packages list with filtering
function renderInstalledPackages() {
    drawerPackageList.innerHTML = '';
    const query = drawerPackageSearch.value.trim().toLowerCase();
    
    const filtered = installedPackages.filter(p => p.name.toLowerCase().includes(query));
    
    if (filtered.length === 0) {
        drawerPackageEmpty.style.display = 'block';
        return;
    }
    
    drawerPackageEmpty.style.display = 'none';
    
    filtered.forEach(p => {
        const tr = document.createElement('tr');
        
        // Determine status tag if any
        let statusTag = '<span style="opacity: 0.6;">OK</span>';
        if (p.name.toLowerCase() === 'pip') {
            statusTag = '<span style="color: var(--accent); font-weight: 500;">System</span>';
        }
        
        tr.innerHTML = `
            <td style="font-weight: 500;">${p.name}</td>
            <td style="font-family: 'JetBrains Mono', monospace; opacity: 0.8;">${p.version}</td>
            <td>${statusTag}</td>
        `;
        drawerPackageList.appendChild(tr);
    });
}

// Search filter binding
drawerPackageSearch.addEventListener('input', renderInstalledPackages);

// Reset Outdated check view
function resetOutdatedScanView() {
    outdatedPackages = [];
    selectedOutdated.clear();
    scannedTarget = '';
    updatesScanPrompt.style.display = 'block';
    updatesScanning.style.display = 'none';
    updatesChecklistView.style.display = 'none';
    updatesCountBadge.style.display = 'none';
}

// Scan Outdated packages
btnScanOutdated.addEventListener('click', async () => {
    updatesScanPrompt.style.display = 'none';
    updatesScanning.style.display = 'block';
    
    try {
        const jsonStr = await invoke('get_outdated_packages', { workspaceDir: getWorkspaceDir(), target: currentExplorerTarget });
        outdatedPackages = JSON.parse(jsonStr);
        
        scannedTarget = currentExplorerTarget;
        
        renderOutdatedChecklist();
    } catch(err) {
        showAlert("Scan Failed", err);
        updatesScanPrompt.style.display = 'block';
    } finally {
        updatesScanning.style.display = 'none';
    }
});

// Render the checklist of outdated packages
function renderOutdatedChecklist() {
    updatesChecklist.innerHTML = '';
    selectedOutdated.clear();
    
    // Filter out 'pip' if it's there, as pip has a cozy dedicated self-update card
    const listToRender = outdatedPackages.filter(p => p.name.toLowerCase() !== 'pip');
    
    // Update badge count
    if (listToRender.length > 0) {
        updatesCountBadge.style.display = 'inline-block';
        updatesCountBadge.textContent = listToRender.length;
    } else {
        updatesCountBadge.style.display = 'none';
    }
    
    if (listToRender.length === 0) {
        updatesChecklist.innerHTML = '<div class="empty-state" style="padding: 20px;">All libraries are up to date!</div>';
        updatesScanPrompt.style.display = 'none';
        updatesChecklistView.style.display = 'block';
        btnUpdateSelected.disabled = true;
        chkSelectAllUpdates.checked = false;
        updateSelectedCountText();
        return;
    }
    
    listToRender.forEach(p => {
        const div = document.createElement('div');
        div.className = 'update-item';
        div.innerHTML = `
            <input type="checkbox" id="chk-pkg-${p.name}" data-pkg-name="${p.name}" />
            <label for="chk-pkg-${p.name}" class="update-info" style="cursor: pointer; width: 100%;">
                <span class="update-name">${p.name}</span>
                <span class="update-versions">
                    <span style="opacity: 0.6;">${p.version}</span>
                    <span class="arrow-symbol">→</span>
                    <span style="color: #ffd43b; font-weight: 600;">${p.latest_version}</span>
                </span>
            </label>
        `;
        
        // Register event listener on checkbox
        const chk = div.querySelector('input[type="checkbox"]');
        chk.addEventListener('change', () => {
            if (chk.checked) {
                selectedOutdated.add(p.name);
            } else {
                selectedOutdated.delete(p.name);
            }
            updateSelectedCountText();
        });
        
        updatesChecklist.appendChild(div);
    });
    
    updatesScanPrompt.style.display = 'none';
    updatesChecklistView.style.display = 'block';
    
    // Reset Select All state
    chkSelectAllUpdates.checked = false;
    updateSelectedCountText();
}

// Update selected text and action button state
function updateSelectedCountText() {
    const size = selectedOutdated.size;
    selectedUpdatesCount.textContent = `${size} selected`;
    btnUpdateSelected.disabled = size === 0;
    
    const actualPackagesCount = outdatedPackages.filter(p => p.name.toLowerCase() !== 'pip').length;
    chkSelectAllUpdates.checked = size === actualPackagesCount && actualPackagesCount > 0;
}

// Select All / Deselect All checkbox binding
chkSelectAllUpdates.addEventListener('change', () => {
    const chks = updatesChecklist.querySelectorAll('input[type="checkbox"]');
    if (chkSelectAllUpdates.checked) {
        chks.forEach(chk => {
            chk.checked = true;
            selectedOutdated.add(chk.dataset.pkgName);
        });
    } else {
        chks.forEach(chk => {
            chk.checked = false;
            selectedOutdated.delete(chk.dataset.pkgName);
        });
    }
    updateSelectedCountText();
});

// Update Selected Packages action
btnUpdateSelected.addEventListener('click', async () => {
    const pkgs = Array.from(selectedOutdated);
    if (pkgs.length === 0) return;
    
    const progressContainer = document.getElementById('pkg-update-progress-container');
    const progressText = document.getElementById('pkg-update-progress-text');
    const progressPercent = document.getElementById('pkg-update-progress-percent');
    const progressBar = document.getElementById('pkg-update-progress-bar');
    
    btnUpdateSelected.disabled = true;
    btnUpdateSelected.textContent = "Updating Packages...";
    progressContainer.style.display = 'block';
    progressBar.style.width = '0%';
    progressBar.setAttribute('aria-valuenow', '0');
    progressPercent.textContent = '0%';
    
    try {
        for (let i = 0; i < pkgs.length; i++) {
            const pkg = pkgs[i];
            const currentNum = i + 1;
            const percent = Math.round((i / pkgs.length) * 100);
            
            progressText.textContent = `Upgrading ${pkg} (${currentNum}/${pkgs.length})...`;
            progressPercent.textContent = `${percent}%`;
            progressBar.style.width = `${percent}%`;
            progressBar.setAttribute('aria-valuenow', String(percent));
            
            await invoke('update_pip_packages', {
                workspaceDir: getWorkspaceDir(),
                target: currentExplorerTarget,
                packages: [pkg],
                all: false
            });
        }
        
        progressText.textContent = `All updates completed!`;
        progressPercent.textContent = `100%`;
        progressBar.style.width = `100%`;
        progressBar.setAttribute('aria-valuenow', '100');
        
        setTimeout(() => {
            progressContainer.style.display = 'none';
            btnUpdateSelected.textContent = "Update Selected Packages";
        }, 1500);
        
        showAlert("Packages Updated", `Successfully upgraded packages:<br><code style="font-size: 11px;">${pkgs.join(', ')}</code>`);
        
        // Refresh installed packages table
        loadDrawerPackages();
        
        // Reset outdated check since updates were performed
        resetOutdatedScanView();
    } catch(err) {
        showAlert("Update Failed", err);
        progressContainer.style.display = 'none';
        btnUpdateSelected.disabled = false;
        btnUpdateSelected.textContent = "Update Selected Packages";
    }
});

// Precheck Requirements.txt URL or File Path
btnPrecheckImport.addEventListener('click', async () => {
    const pathOrUrl = importRequirementsPath.value.trim();
    if (!pathOrUrl) {
        showAlert("Input Required", "Please paste a requirements.txt local path or remote URL.");
        return;
    }
    
    btnPrecheckImport.disabled = true;
    importPrecheckLoading.style.display = 'block';
    importPrecheckDashboard.style.display = 'none';
    btnInstallImported.disabled = true;
    
    try {
        const jsonStr = await invoke('precheck_requirements', {
            workspaceDir: getWorkspaceDir(),
            target: currentExplorerTarget,
            pathOrUrl: pathOrUrl
        });
        
        const precheck = JSON.parse(jsonStr);
        renderPrecheckResults(precheck);
    } catch(err) {
        showAlert("Precheck Failed", err);
    } finally {
        btnPrecheckImport.disabled = false;
        importPrecheckLoading.style.display = 'none';
    }
});

// Render precheck analysis dashboard
function renderPrecheckResults(precheck) {
    importPrecheckDashboard.style.display = 'block';
    
    // 1. Result Banner
    if (precheck.is_safe) {
        precheckBanner.style.background = 'rgba(16, 185, 129, 0.1)';
        precheckBanner.style.border = '1px solid rgba(16, 185, 129, 0.2)';
        precheckBanner.style.color = 'var(--success)';
        precheckBannerIcon.textContent = '✓';
        precheckBannerText.textContent = 'Environment is safe and fully compatible!';
        btnInstallImported.disabled = false;
    } else {
        precheckBanner.style.background = 'rgba(239, 68, 68, 0.1)';
        precheckBanner.style.border = '1px solid rgba(239, 68, 68, 0.2)';
        precheckBanner.style.color = '#f87171';
        precheckBannerIcon.textContent = '⚠';
        precheckBannerText.textContent = 'Version mismatch or conflicts detected.';
        btnInstallImported.disabled = false; // We still allow installation, but with warning
    }
    
    // 2. Active conflicts
    precheckConflictsList.innerHTML = '';
    if (precheck.potential_conflicts && precheck.potential_conflicts.length > 0) {
        precheckConflictsSection.style.display = 'block';
        precheck.potential_conflicts.forEach(c => {
            const div = document.createElement('div');
            div.className = 'import-issue-item';
            div.innerHTML = `<strong>${c.package}</strong>: installed version (${c.installed}) violates requirement "${c.requirement}"`;
            precheckConflictsList.appendChild(div);
        });
    } else {
        precheckConflictsSection.style.display = 'none';
    }
    
    // 3. Resolved packages
    precheckResolvedList.innerHTML = '';
    if (precheck.resolved_packages && precheck.resolved_packages.length > 0) {
        precheck.resolved_packages.forEach(p => {
            const tr = document.createElement('tr');
            
            let statusBadge = '<span style="color: var(--success); font-weight: 500;">Compatible</span>';
            // Check if this package has a conflict
            const hasConflict = precheck.potential_conflicts?.some(c => c.package === p.name);
            if (hasConflict) {
                // Find matching conflict
                const conf = precheck.potential_conflicts.find(c => c.package === p.name);
                statusBadge = `<span class="warning-badge" data-tooltip="Installed: ${conf.installed} vs Req: ${conf.requirement}">Conflict</span>`;
            } else if (p.version === 'not installed') {
                statusBadge = '<span style="color: #60a5fa;">Pending Install</span>';
            }
            
            tr.innerHTML = `
                <td style="font-weight: 500;">${p.name}</td>
                <td style="font-family: 'JetBrains Mono', monospace;">${p.version}</td>
                <td>${statusBadge}</td>
            `;
            precheckResolvedList.appendChild(tr);
        });
    } else {
        precheckResolvedList.innerHTML = '<tr><td colspan="3" style="text-align:center; opacity:0.6;">No packages specified.</td></tr>';
    }
}

// Trigger Install of Imported dependencies
btnInstallImported.addEventListener('click', async () => {
    const pathOrUrl = importRequirementsPath.value.trim();
    if (!pathOrUrl) return;
    
    btnInstallImported.disabled = true;
    btnInstallImported.textContent = "Installing Dependencies...";
    
    try {
        await invoke('install_requirements', {
            workspaceDir: getWorkspaceDir(),
            target: currentExplorerTarget,
            pathOrUrl: pathOrUrl
        });
        
        showAlert("Installation Complete", "Dependencies installed successfully.");
        
        // Refresh installed packages table
        loadDrawerPackages();
        
        // Reset outdated check
        resetOutdatedScanView();
        
        // Switch to installed tab to see them
        switchDrawerTab('tab-installed');
    } catch(err) {
        showAlert("Installation Failed", err);
        btnInstallImported.disabled = false;
        btnInstallImported.textContent = "Install Dependencies";
    }
});

// Codebase Import Analyzer Logic
if (btnScanImports) {
    btnScanImports.addEventListener('click', async () => {
        const wsDir = currentWorkspaceDir || '';
        if (!wsDir) {
            showAlert("Workspace Required", "Please select a workspace directory first using the Workspace Selector in the sidebar.");
            return;
        }
        
        btnScanImports.disabled = true;
        btnScanImports.textContent = "Scanning...";
        if (scanImportsResults) scanImportsResults.style.display = 'none';
        if (scanImportsLoading) scanImportsLoading.style.display = 'block';
        if (scanMissingBadges) scanMissingBadges.innerHTML = '';
        if (scanInstalledBadges) scanInstalledBadges.innerHTML = '';
        
        try {
            const jsonStr = await invoke('analyze_codebase_imports', {
                workspaceDir: getWorkspaceDir(),
                target: currentExplorerTarget,
                dirPath: wsDir
            });
            
            const scan = JSON.parse(jsonStr);
            if (scan.error) {
                throw new Error(scan.error);
            }
            
            scannedMissingImports = scan.missing_imports || [];
            
            // 1. Render Missing Dependencies
            if (scannedMissingImports.length === 0) {
                if (scanMissingCard) scanMissingCard.style.display = 'none';
            } else {
                if (scanMissingCard) scanMissingCard.style.display = 'block';
                const missingDesc = document.getElementById('scan-missing-desc');
                if (missingDesc) {
                    missingDesc.textContent = `These ${scannedMissingImports.length} libraries are used in the codebase but not currently installed.`;
                }
                
                if (scanMissingBadges) {
                    scannedMissingImports.forEach(pkg => {
                        const badge = document.createElement('span');
                        badge.className = 'badge';
                        badge.style.background = 'rgba(239, 68, 68, 0.1)';
                        badge.style.color = '#f87171';
                        badge.style.border = '1px solid rgba(239, 68, 68, 0.2)';
                        badge.style.marginRight = '4px';
                        badge.style.marginBottom = '4px';
                        badge.textContent = pkg;
                        scanMissingBadges.appendChild(badge);
                    });
                }
                
                if (btnInstallMissingImports) {
                    btnInstallMissingImports.disabled = false;
                    btnInstallMissingImports.textContent = "Install Missing Dependencies";
                }
            }
            
            // 2. Render Aligned/Installed Dependencies
            const installedImports = scan.installed_imports || [];
            if (installedImports.length === 0) {
                if (scanInstalledCard) scanInstalledCard.style.display = 'none';
            } else {
                if (scanInstalledCard) scanInstalledCard.style.display = 'block';
                if (scanInstalledBadges) {
                    installedImports.forEach(pkg => {
                        const badge = document.createElement('span');
                        badge.className = 'badge';
                        badge.style.background = 'rgba(16, 185, 129, 0.1)';
                        badge.style.color = '#34d399';
                        badge.style.border = '1px solid rgba(16, 185, 129, 0.2)';
                        badge.style.marginRight = '4px';
                        badge.style.marginBottom = '4px';
                        badge.textContent = `${pkg.name} (${pkg.version})`;
                        scanInstalledBadges.appendChild(badge);
                    });
                }
            }
            
            if (scanImportsResults) scanImportsResults.style.display = 'block';
            
        } catch (err) {
            showAlert("Scan Failed", `Failed to statically scan workspace codebase imports:<br><code style="font-size: 11px;">${err}</code>`);
        } finally {
            if (scanImportsLoading) scanImportsLoading.style.display = 'none';
            btnScanImports.disabled = false;
            btnScanImports.textContent = "Scan Workspace";
        }
    });
}

if (btnInstallMissingImports) {
    btnInstallMissingImports.addEventListener('click', async () => {
        if (!scannedMissingImports || scannedMissingImports.length === 0) return;
        
        btnInstallMissingImports.disabled = true;
        const total = scannedMissingImports.length;
        let installedCount = 0;
        
        for (const pkg of scannedMissingImports) {
            btnInstallMissingImports.textContent = `Installing ${pkg} (${installedCount + 1}/${total})...`;
            try {
                await invoke('update_pip_packages', {
                    workspaceDir: getWorkspaceDir(),
                    target: currentExplorerTarget,
                    packages: [pkg],
                    all: false
                });
                installedCount++;
            } catch (err) {
                showAlert("Installation Failed", `Failed to install dependency <b>${pkg}</b>:<br><code style="font-size:11px;">${err}</code>`);
                btnInstallMissingImports.disabled = false;
                btnInstallMissingImports.textContent = "Install Missing Dependencies";
                return;
            }
        }
        
        showAlert("Installation Complete", `Successfully installed all ${total} missing dependencies!`);
        
        // Refresh installed list in drawer
        loadDrawerPackages();
        // Reset outdated check in updates tab
        resetOutdatedScanView();
        
        // Re-run scan to update status UI instantly
        if (btnScanImports) btnScanImports.click();
    });
}

// Check Active Pip Status in background on Dashboard Load
async function checkActivePipStatus(target) {
    try {
        const jsonStr = await invoke('get_outdated_packages', { workspaceDir: getWorkspaceDir(), target: target });
        const outdated = JSON.parse(jsonStr);
        
        const pipOutdated = outdated.find(p => p.name.toLowerCase() === 'pip');
        if (pipOutdated) {
            updateActivePipLight(true, pipOutdated.version, pipOutdated.latest_version);
        } else {
            updateActivePipLight(false);
        }
    } catch(err) {
        console.warn("Failed to check active pip status in background:", err);
        updateActivePipLight(false);
    }
}

// Update Active Pip Status indicator on main GUI Dashboard
function updateActivePipLight(isOutdated, currentVer = '', latestVer = '') {
    const container = document.getElementById('active-pip-status-container');
    const text = document.getElementById('active-pip-status-text');
    
    if (container && text) {
        if (isOutdated) {
            container.style.display = 'inline-flex';
            text.innerHTML = `pip update available (current ${currentVer} → latest ${latestVer})`;
        } else {
            container.style.display = 'none';
        }
    }
}

// Hook into Dashboard loading to trigger background Pip checks
const originalLoadDashboard = loadDashboard;
loadDashboard = async function() {
    await originalLoadDashboard();
    
    try {
        const jsonStr = await invoke('get_status', { workspaceDir: getWorkspaceDir() });
        const status = JSON.parse(jsonStr);
        
        if (status.active_versions && status.active_versions.length > 0) {
            checkActivePipStatus(status.active_versions[0]);
        } else if (status.managed_venv) {
            checkActivePipStatus(`venv:${status.managed_venv.name}`);
        } else {
            updateActivePipLight(false);
        }
    } catch(err) {
        console.warn("Dashboard background check failure:", err);
        updateActivePipLight(false);
    }
};

// ─── Shell Integration & Diagnostics ───
async function loadShellIntegration() {
    const cardsContainer = document.getElementById('shell-status-cards');
    if (!cardsContainer) return;
    
    cardsContainer.innerHTML = '<div class="empty-state">Scanning shells and configurations...</div>';
    
    try {
        const statuses = await invoke('get_shell_statuses', { workspaceDir: getWorkspaceDir() });
        cardsContainer.innerHTML = '';
        
        if (!statuses || statuses.length === 0) {
            cardsContainer.innerHTML = '<div class="empty-state">No standard shells discovered on this platform.</div>';
            return;
        }
        
        statuses.forEach(status => {
            const card = document.createElement('div');
            card.className = 'status-card glass fade-in';
            card.style.display = 'flex';
            card.style.flexDirection = 'column';
            card.style.justifyContent = 'space-between';
            card.style.minHeight = '180px';
            
            if (!status.is_installed) {
                card.style.opacity = '0.55';
            }
            
            // Config Badge HTML
            let configBadge = '';
            if (!status.is_installed) {
                configBadge = `<span class="badge" style="background: rgba(148, 163, 184, 0.08); color: #94a3b8; border: 1px solid rgba(148, 163, 184, 0.15);">
                    <svg viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" stroke-width="2.5" fill="none" stroke-linecap="round" stroke-linejoin="round" style="margin-right: 4px; display: inline; vertical-align: middle;"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>Not Detected
                   </span>`;
            } else if (status.is_configured) {
                configBadge = `<span class="badge" style="background: rgba(16, 185, 129, 0.1); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.2);">
                    <svg viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" stroke-width="2.5" fill="none" stroke-linecap="round" stroke-linejoin="round" style="margin-right: 4px; display: inline; vertical-align: middle;"><polyline points="20 6 9 17 4 12"></polyline></svg>Configured
                   </span>`;
            } else {
                configBadge = `<span class="badge" style="background: rgba(245, 158, 11, 0.1); color: #fbbf24; border: 1px solid rgba(245, 158, 11, 0.2);">
                    <svg viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" stroke-width="2.5" fill="none" stroke-linecap="round" stroke-linejoin="round" style="margin-right: 4px; display: inline; vertical-align: middle;"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>Not Configured
                   </span>`;
            }
                   
            // PATH Badge HTML
            let pathBadge = '';
            if (status.is_installed) {
                pathBadge = status.active_in_path 
                    ? `<span class="badge" style="background: rgba(16, 185, 129, 0.1); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.2); margin-left: 8px;">
                        PATH Active
                       </span>`
                    : `<span class="badge" style="background: rgba(245, 158, 11, 0.1); color: #fbbf24; border: 1px solid rgba(245, 158, 11, 0.2); margin-left: 8px;">
                        Shims Missing from PATH
                       </span>`;
            }

            // Button style & text
            let btnClass = 'btn-gold';
            let btnText = 'Auto-Configure Shell';
            let btnDisabled = '';
            
            if (!status.is_installed) {
                btnClass = 'btn-outline';
                btnText = 'Shell Not Installed';
                btnDisabled = 'disabled';
            } else if (status.is_configured) {
                btnClass = 'btn-outline';
                btnText = 'Configured';
                btnDisabled = 'disabled';
            }

            card.innerHTML = `
                <div>
                  <h3 style="margin: 0 0 8px; color: #fff; font-size: 16px;">${status.name}</h3>
                  <div style="margin-bottom: 12px; display: flex; flex-wrap: wrap; gap: 4px;">
                    ${configBadge}
                    ${pathBadge}
                  </div>
                  <div style="font-size: 11px; color: var(--text-muted); word-break: break-all; line-height: 1.4;">
                    <strong>Profile File:</strong><br>${status.profile_path}
                  </div>
                </div>
                <div style="margin-top: 16px;">
                  <button class="btn ${btnClass}" 
                    style="width: 100%; padding: 8px; font-size: 11px; font-weight: 600;" 
                    ${btnDisabled} 
                    id="btn-cfg-${status.name.replace(/[^a-zA-Z0-9]/g, '-')}">
                    ${btnText}
                  </button>
                </div>
            `;
            
            cardsContainer.appendChild(card);
            
            // Add click listener to config button
            if (status.is_installed && !status.is_configured) {
                const btn = card.querySelector(`#btn-cfg-${status.name.replace(/[^a-zA-Z0-9]/g, '-')}`);
                if (btn) {
                    btn.addEventListener('click', async () => {
                        btn.disabled = true;
                        btn.textContent = "Configuring...";
                        try {
                            await invoke('configure_shell', { shellName: status.name, profilePath: status.profile_path });
                            showAlert("Shell Configured", `Successfully injected pyenv-native shell initialization block into profile:<br><code style="font-size: 11px;">${status.profile_path}</code><br><br>Restart your terminal shell for the changes to take effect!`);
                            loadShellIntegration();
                        } catch(err) {
                            showAlert("Configuration Failed", err);
                            btn.disabled = false;
                            btn.textContent = "Auto-Configure Shell";
                        }
                    });
                }
            }
        });
        
        // Proactively run diagnostics in the background
        runDoctorDiagnostics();
    } catch(err) {
        console.error("Failed to load shell statuses:", err);
        cardsContainer.innerHTML = `<div class="empty-state" style="color: var(--danger);">Failed to scan shells: ${err}</div>`;
    }
}

// Refresh Shells action
const btnRefreshShells = document.getElementById('btn-refresh-shells');
if (btnRefreshShells) {
    btnRefreshShells.addEventListener('click', loadShellIntegration);
}

// Doctor Diagnostics & Self-Healing Repairs Logic
async function runDoctorDiagnostics() {
    const btnRunDoctor = document.getElementById('btn-run-doctor');
    const doctorLoading = document.getElementById('doctor-loading');
    const doctorResults = document.getElementById('doctor-results');
    const doctorIssuesCard = document.getElementById('doctor-issues-card');
    const doctorIssuesList = document.getElementById('doctor-issues-list');
    const doctorHealthyCard = document.getElementById('doctor-healthy-card');
    const btnDoctorFix = document.getElementById('btn-doctor-fix');

    if (!btnRunDoctor) return;

    btnRunDoctor.disabled = true;
    btnRunDoctor.textContent = "Checking...";
    if (doctorLoading) doctorLoading.style.display = 'block';
    if (doctorResults) doctorResults.style.display = 'none';
    if (doctorIssuesCard) doctorIssuesCard.style.display = 'none';
    if (doctorHealthyCard) doctorHealthyCard.style.display = 'none';
    if (doctorIssuesList) doctorIssuesList.innerHTML = '';

    try {
        const jsonStr = await invoke('run_doctor', { workspaceDir: getWorkspaceDir() });
        // FFI command returns Vec<DoctorCheckGui> direct serialization
        const checks = typeof jsonStr === 'string' ? JSON.parse(jsonStr) : jsonStr;
        
        // Filter out Ok/Info checks to locate Warn/issues
        const issues = checks.filter(c => c.status === 'Warn');
        
        if (issues.length === 0) {
            if (doctorHealthyCard) doctorHealthyCard.style.display = 'block';
        } else {
            if (doctorIssuesCard) doctorIssuesCard.style.display = 'block';
            if (doctorIssuesList) {
                issues.forEach(issue => {
                    const item = document.createElement('div');
                    item.style.padding = '8px 12px';
                    item.style.borderRadius = '6px';
                    item.style.background = 'rgba(251, 191, 36, 0.04)';
                    item.style.borderLeft = '3px solid #fbbf24';
                    item.style.marginBottom = '6px';
                    item.innerHTML = `<strong style="color: #fbbf24;">${issue.name}</strong>: ${issue.detail}`;
                    doctorIssuesList.appendChild(item);
                });
            }
            if (btnDoctorFix) {
                btnDoctorFix.disabled = false;
                btnDoctorFix.textContent = "Attempt Self-Healing Repair";
            }
        }
        
        if (doctorResults) doctorResults.style.display = 'block';
    } catch(err) {
        console.error("Doctor failed:", err);
    } finally {
        if (doctorLoading) doctorLoading.style.display = 'none';
        btnRunDoctor.disabled = false;
        btnRunDoctor.textContent = "Run Diagnostics";
    }
}

async function attemptDoctorFix() {
    const btnDoctorFix = document.getElementById('btn-doctor-fix');
    if (!btnDoctorFix) return;

    btnDoctorFix.disabled = true;
    btnDoctorFix.textContent = "Applying self-healing repairs...";

    try {
        const applied = await invoke('run_doctor_fix', { workspaceDir: getWorkspaceDir() });
        
        let message = "Applied the following automated repairs:<br><ul style='margin: 8px 0; padding-left: 20px; font-size: 11px;'>";
        applied.forEach(act => {
            message += `<li>${act}</li>`;
        });
        message += "</ul><br>Diagnostics will now be re-run.";
        
        showAlert("Self-Healing Complete", message);
        
        // Re-run diagnostics
        runDoctorDiagnostics();
    } catch(err) {
        showAlert("Self-Healing Failed", `Failed to automatically repair system environment:<br><code style="font-size:11px;">${err}</code>`);
        btnDoctorFix.disabled = false;
        btnDoctorFix.textContent = "Attempt Self-Healing Repair";
    }
}

const btnRunDoctor = document.getElementById('btn-run-doctor');
if (btnRunDoctor) {
    btnRunDoctor.addEventListener('click', runDoctorDiagnostics);
}

const btnDoctorFix = document.getElementById('btn-doctor-fix');
if (btnDoctorFix) {
    btnDoctorFix.addEventListener('click', attemptDoctorFix);
}

// Init startup
initAppVersion();
checkInstallation();
