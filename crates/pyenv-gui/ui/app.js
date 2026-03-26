// ./crates/pyenv-gui/ui/app.js
//! Frontend logic for pyenv-native GUI.
//! Uses Tauri v2 IPC to communicate with the Rust backend.

const invoke = (window.__TAURI__ && window.__TAURI__.core) ? window.__TAURI__.core.invoke : window.__TAURI__.invoke;

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
        const jsonStr = await invoke('get_status');
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
        const jsonStr = await invoke('get_installed_versions');
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
                        ${entry.available ? `<button class="btn btn-outline" onclick="setGlobal('system')">Make Global</button>` : ''}
                    </div>
                `;
            } else {
                card.innerHTML = `
                    <div class="version-name">${entry.name}</div>
                    <div class="version-actions">
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
        const statusJson = await invoke('get_status');
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
        const installedStr = await invoke('get_installed_versions');
        installedVersionsSet = JSON.parse(installedStr);

        if (fullAvailableCache.length === 0) {
            const jsonStr = await invoke('get_available_versions', { family: null, pattern: null });
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
        const jsonStr = await invoke('get_managed_venvs');
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
                        <button class="btn btn-outline" onclick="setGlobal('venv:${v.name}')">Global</button>
                        <button class="btn btn-outline" onclick="setLocal('venv:${v.name}')">Local</button>
                        <button class="btn btn-danger" onclick="deleteVenv('${v.name}')">Delete</button>
                    </div>
                `;
                list.appendChild(card);
            });
        }
        const sysInfo = await invoke('get_installed_versions');
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
        await invoke('install_version', { version: v });
        btnEl.innerHTML = "Installed ✓";
        btnEl.classList.remove('btn-primary');
        btnEl.classList.add('btn-outline');
        installedVersionsSet.push(v);
        loadDashboard();
    } catch (err) {
        console.error("Install failed:", err);
        btnEl.innerText = originalText;
        btnEl.disabled = false;
        showAlert('Install Failed', err);
    }
}

async function setGlobal(v) {
    try {
        await invoke('set_global', { version: v });
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
        await invoke('create_venv', { baseVersion: base, name: name });
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
        await invoke('delete_venv', { spec: name });
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
        const result = await invoke('check_for_updates');
        // Parse whether an update is available from the result text
        if (result.includes('up to date') || result.includes('Up to date') || result.includes('already up to date')) {
            showAlert('Up to Date', result);
        } else if (result.includes('Update available') || result.includes('newer')) {
            const yes = await showConfirm('Update Available', result + '<br><br>Would you like to update now?');
            if (yes) {
                if(btn) btn.innerText = "Updating…";
                try {
                    const updateResult = await invoke('perform_update');
                    showAlert('Update Started', updateResult || 'The update process has been initiated. The application may need to restart.');
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
        const jsonStr = await invoke('get_config');
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
        await invoke('set_config', { key, value });
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
        await invoke('uninstall_version', { version: v });
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

// Init startup
initAppVersion();
checkInstallation();
