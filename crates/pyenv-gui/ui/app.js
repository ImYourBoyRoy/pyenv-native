// ./crates/pyenv-gui/ui/app.js
//! Frontend logic for pyenv-native GUI.
//! Uses Tauri v2 IPC to communicate with the Rust backend.

const invoke = (window.__TAURI__ && window.__TAURI__.core) ? window.__TAURI__.core.invoke : window.__TAURI__.invoke;

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
function showModal(title, message, buttons = [{label: 'OK', style: 'btn-primary'}]) {
    return new Promise(resolve => {
        const root = document.getElementById('modal-root');
        const overlay = document.createElement('div');
        overlay.className = 'modal-overlay';
        overlay.innerHTML = `
            <div class="modal-box">
                <h3>${title}</h3>
                <p>${message}</p>
                <div class="modal-actions" id="modal-btns"></div>
            </div>
        `;
        const btnContainer = overlay.querySelector('#modal-btns');
        buttons.forEach((b, i) => {
            const btn = document.createElement('button');
            btn.className = `btn ${b.style || 'btn-outline'}`;
            btn.textContent = b.label;
            btn.onclick = () => { root.removeChild(overlay); resolve(i === 0); };
            btnContainer.appendChild(btn);
        });
        root.appendChild(overlay);
    });
}

async function showAlert(title, message) {
    return showModal(title, message, [{label: 'OK', style: 'btn-primary'}]);
}

async function showConfirm(title, message) {
    return showModal(title, message, [
        {label: 'Confirm', style: 'btn-danger'},
        {label: 'Cancel', style: 'btn-outline'}
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
const sidebarNavItems = document.querySelectorAll('.nav li');
const views = document.querySelectorAll('.view');

function showView(viewId) {
    views.forEach(v => {
        v.style.display = 'none';
        v.classList.remove('fade-in');
    });
    const target = document.getElementById(viewId);
    target.style.display = 'block';
    void target.offsetWidth;
    target.classList.add('fade-in');
}

sidebarNavItems.forEach(item => {
    item.addEventListener('click', () => {
        if(item.classList.contains('active')) return;
        sidebarNavItems.forEach(i => i.classList.remove('active'));
        item.classList.add('active');
        showView(item.dataset.view);
        
        if (item.dataset.view === 'view-dashboard') loadDashboard();
        if (item.dataset.view === 'view-installed') loadInstalled();
        if (item.dataset.view === 'view-available') setupAvailableView();
        if (item.dataset.view === 'view-venvs') loadVenvs();
        if (item.dataset.view === 'view-settings') loadConfig();
    });
});

// ─── Dashboard ───
async function loadDashboard() {
    try {
        const jsonStr = await invoke('get_status', { workspaceDir: getWorkspaceDir() });
        const status = JSON.parse(jsonStr);
        
        const activeVersionEl = document.getElementById('active-version');
        activeVersionEl.textContent = status.active_versions.length ? status.active_versions.join(', ') : 'None';
        document.getElementById('active-origin').textContent = `Origin: ${status.origin}`;
        
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
    list.innerHTML = '<div class="empty-state"><div class="loader"></div></div>';
    
    try {
        const jsonStr = await invoke('get_installed_versions', { workspaceDir: getWorkspaceDir() });
        const versions = JSON.parse(jsonStr);
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

        if (displayVersions.length === 0) {
            list.innerHTML = '<div class="empty-state">No versions installed yet.</div>';
            return;
        }

        displayVersions.forEach(entry => {
            const card = document.createElement('div');
            card.className = 'version-card';
            
            if (entry.isSystem) {
                const badge = entry.available
                    ? '<span class="system-badge badge-success">Detected</span>'
                    : '<span class="system-badge badge-muted">Not Available</span>';
                
                card.innerHTML = `
                    <div>
                        <div class="version-name">system ${badge}</div>
                        <div class="version-meta">${entry.available ? 'System-wide Python installation' : 'No system Python found (Microsoft Store alias detected)'}</div>
                    </div>
                    <div class="version-actions">
                        ${entry.available ? `
                            <button class="btn btn-outline" onclick="openPackageExplorer('system')">Package Explorer</button>
                            <button class="btn btn-outline" onclick="setGlobal('system')">Make Global</button>
                        ` : ''}
                    </div>
                `;
            } else {
                card.innerHTML = `
                    <div class="version-name">${entry.name}</div>
                    <div class="version-actions">
                        <button class="btn btn-outline" onclick="openPackageExplorer('${entry.name}')">Package Explorer</button>
                        <button class="btn btn-outline" onclick="setGlobal('${entry.name}')">Make Global</button>
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
        const jsonStr = await invoke('get_managed_venvs', { workspaceDir: getWorkspaceDir() });
        const venvs = JSON.parse(jsonStr);
        list.innerHTML = '';
        if (venvs.length === 0) {
            list.innerHTML = '<div class="empty-state">No virtual environments found.</div>';
        } else {
                venvs.forEach(v => {
                    const card = document.createElement('div');
                    card.className = 'version-card';
                    card.innerHTML = `
                        <div>
                            <div class="version-name">${v.name}</div>
                            <div class="version-meta">Base: ${v.base_version} • ${v.path}</div>
                        </div>
                        <div class="version-actions">
                            <button class="btn btn-outline" onclick="openPackageExplorer('venv:${v.name}')">Package Explorer</button>
                            <button class="btn btn-outline" onclick="setGlobal('venv:${v.name}')">Global</button>
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
    try {
        await invoke('set_config', { workspaceDir: getWorkspaceDir(), key, value });
        console.log(`Saved config ${key}=${value}`);
    } catch(err) {
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
    drawer.classList.add('open');
    
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
};

// Close Package Explorer Drawer
function closePackageExplorer() {
    drawer.classList.remove('open');
    currentExplorerTarget = '';
}

drawerCloseBtn.addEventListener('click', closePackageExplorer);

// Tab switching logic inside the drawer
function switchDrawerTab(tabId) {
    drawerTabBtns.forEach(btn => {
        if (btn.dataset.drawerTab === tabId) {
            btn.classList.add('active');
        } else {
            btn.classList.remove('active');
        }
    });
    
    drawerTabContents.forEach(content => {
        if (content.id === tabId) {
            content.classList.add('active');
        } else {
            content.classList.remove('active');
        }
    });
}

drawerTabBtns.forEach(btn => {
    btn.addEventListener('click', () => {
        switchDrawerTab(btn.dataset.drawerTab);
    });
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
            packages: ["pip"]
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
    
    btnUpdateSelected.disabled = true;
    btnUpdateSelected.textContent = "Updating Selected Packages...";
    
    try {
        await invoke('update_pip_packages', {
            workspaceDir: getWorkspaceDir(),
            target: currentExplorerTarget,
            packages: pkgs
        });
        
        showAlert("Packages Updated", `Successfully upgraded packages:<br><code style="font-size: 11px;">${pkgs.join(', ')}</code>`);
        
        // Refresh installed packages table
        loadDrawerPackages();
        
        // Reset outdated check since updates were performed
        resetOutdatedScanView();
    } catch(err) {
        showAlert("Update Failed", err);
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

// Init startup
initAppVersion();
checkInstallation();
