// ./crates/pyenv-gui/ui/app.js
//! Frontend logic for pyenv-native GUI.
//! Uses Tauri v2 IPC to communicate with the Rust backend.

function invoke(cmd, args) {
    const fn = window.__TAURI__?.core?.invoke || window.__TAURI__?.invoke;
    if (typeof fn !== 'function') {
        return Promise.reject(new Error('Pyenv Native GUI backend is not connected. Launch with `pyenv gui`.'));
    }
    return fn(cmd, args);
}
const {
    lockAppShell,
    unlockAppShell,
    getFocusableElements,
    trapFocus,
    appendRichMessage,
    escapeHtml,
    setRegionLoading,
    showEmptyState,
    createActionButton,
    createWarningBadge,
    sanitizeDomId,
    fillSelectOptions,
    createBadge,
} = window.DomUtils;

let lastFocusedBeforeOverlay = null;
let drawerReturnFocus = null;
let toastTimer = null;
let lastNavInput = 'pointer';
let openDrawerCount = 0;
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

function setStepIconContent(iconEl, content) {
    if (!iconEl) return;
    iconEl.replaceChildren();
    if (typeof content === 'string') {
        iconEl.textContent = content;
        return;
    }
    iconEl.appendChild(content);
}

function createStepLoader() {
    const loader = document.createElement('div');
    loader.className = 'loader loader-sm';
    loader.style.width = '12px';
    loader.style.height = '12px';
    loader.style.borderWidth = '2px';
    loader.style.borderTopColor = '#ffd43b';
    loader.setAttribute('aria-hidden', 'true');
    return loader;
}

function createStepSvg(paths, viewBox = '0 0 24 24') {
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('viewBox', viewBox);
    svg.setAttribute('width', '10');
    svg.setAttribute('height', '10');
    svg.setAttribute('stroke', 'currentColor');
    svg.setAttribute('stroke-width', '3');
    svg.setAttribute('fill', 'none');
    svg.setAttribute('stroke-linecap', 'round');
    svg.setAttribute('stroke-linejoin', 'round');
    paths.forEach((pathData) => {
        const path = document.createElementNS('http://www.w3.org/2000/svg', pathData.tag);
        Object.entries(pathData.attrs).forEach(([key, value]) => path.setAttribute(key, value));
        svg.appendChild(path);
    });
    return svg;
}

function setFooterAppStatus(kind, detail = '') {
    const statusEl = document.getElementById('footer-app-status');
    if (!statusEl) return;
    statusEl.replaceChildren();
    const span = document.createElement('span');
    if (kind === 'update') {
        span.style.color = 'var(--danger)';
        span.textContent = `Update Available: v${detail}`;
    } else if (kind === 'uptodate') {
        span.style.color = '#10b981';
        span.textContent = '✓ Up to Date';
    } else if (kind === 'offline') {
        span.style.opacity = '0.5';
        span.textContent = '(Offline)';
    }
    statusEl.appendChild(span);
}

function appendFooterPortableBadge() {
    const statusEl = document.getElementById('footer-app-status');
    if (!statusEl || statusEl.querySelector('[data-portable-badge]')) return;
    const badge = createBadge('Portable', 'badge badge-success', 'font-size:10px; padding: 2px 6px; margin-left: 8px;');
    badge.dataset.portableBadge = 'true';
    statusEl.appendChild(document.createTextNode(' '));
    statusEl.appendChild(badge);
}

function showInlineRemovingState(container) {
    if (!container) return;
    container.replaceChildren();
    const loader = document.createElement('div');
    loader.className = 'loader loader-sm';
    loader.style.display = 'inline-block';
    loader.setAttribute('aria-hidden', 'true');
    const label = document.createElement('span');
    label.style.fontSize = '12px';
    label.style.marginLeft = '8px';
    label.textContent = 'Removing…';
    container.append(loader, label);
}

function showToast(message) {
    const toast = document.getElementById('status-toast');
    if (!toast) return;
    toast.textContent = message;
    toast.classList.add('visible');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => toast.classList.remove('visible'), 2600);
}

function getOpenDrawerOverlay() {
    if (drawer?.classList.contains('open')) return drawer;
    if (upgraderDrawer?.classList.contains('open')) return upgraderDrawer;
    return null;
}

function setDrawerState(drawerEl, open, options = {}) {
    if (!drawerEl) return;
    const closeBtn = options.closeButton;
    if (open) {
        if (openDrawerCount === 0) lockAppShell();
        openDrawerCount += 1;
        drawerReturnFocus = options.returnFocus || document.activeElement;
        drawerEl.classList.add('open');
        drawerEl.removeAttribute('inert');
        drawerEl.setAttribute('aria-hidden', 'false');
        const focusTarget = closeBtn || drawerEl.querySelector('.drawer-close');
        focusTarget?.focus();
        announce(options.announce || 'Panel opened');
    } else {
        openDrawerCount = Math.max(0, openDrawerCount - 1);
        drawerEl.classList.remove('open');
        drawerEl.setAttribute('inert', '');
        drawerEl.setAttribute('aria-hidden', 'true');
        if (openDrawerCount === 0) unlockAppShell();
        const restore = options.returnFocus || drawerReturnFocus;
        if (restore && typeof restore.focus === 'function') restore.focus();
        announce(options.announce || 'Panel closed');
    }
}

function readBooleanPreference(storageKey, mediaQuery) {
    const stored = localStorage.getItem(storageKey);
    if (stored === null && mediaQuery) {
        return window.matchMedia(mediaQuery).matches;
    }
    return stored === 'true';
}

function applyAccessibilityPreferences() {
    const prefs = {
        highContrast: readBooleanPreference('pyenv-a11y-high-contrast'),
        reducedMotion: readBooleanPreference('pyenv-a11y-reduced-motion', '(prefers-reduced-motion: reduce)'),
        strongFocus: readBooleanPreference('pyenv-a11y-strong-focus'),
        largeText: readBooleanPreference('pyenv-a11y-large-text'),
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

function resetAccessibilityPreferences() {
    [
        'pyenv-a11y-high-contrast',
        'pyenv-a11y-reduced-motion',
        'pyenv-a11y-strong-focus',
        'pyenv-a11y-large-text',
    ].forEach((key) => localStorage.removeItem(key));
    applyAccessibilityPreferences();
    showToast('Accessibility preferences reset');
    announce('Accessibility preferences reset to system defaults');
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
    const selectBindings = [
        ['config-windows.registry_mode', 'windows.registry_mode'],
        ['config-install.arch', 'install.arch'],
    ];
    selectBindings.forEach(([id, key]) => {
        const select = document.getElementById(id);
        if (!select) return;
        select.addEventListener('change', () => updateConfig(key, select.value));
    });

    const checkboxBindings = [
        ['config-install.bootstrap_pip', 'install.bootstrap_pip'],
        ['config-venv.auto_create_base_venv', 'venv.auto_create_base_venv'],
        ['config-venv.auto_use_base_venv', 'venv.auto_use_base_venv'],
    ];
    checkboxBindings.forEach(([id, key]) => {
        const checkbox = document.getElementById(id);
        if (!checkbox) return;
        checkbox.addEventListener('change', () => updateConfig(key, checkbox.checked ? 'true' : 'false'));
    });
}

function handleDelegatedAction(event) {
    const button = event.target.closest('[data-action]');
    if (!button || button.disabled) return;

    closeCardMenus();
    const { action, target, baseVersion } = button.dataset;
    switch (action) {
        case 'navigate':
            navigateToView(target);
            if (button.dataset.scroll) {
                requestAnimationFrame(() => {
                    document.getElementById(button.dataset.scroll)?.scrollIntoView({ block: 'start' });
                });
            }
            break;
        case 'package-explorer':
            openPackageExplorer(target);
            break;
        case 'set-global':
            setGlobal(target);
            break;
        case 'set-local':
            setLocal(target, { pickFolder: false });
            break;
        case 'set-local-pick':
            setLocal(target, { pickFolder: true });
            break;
        case 'uninstall':
            uninstallVersion(target);
            break;
        case 'install':
            installTarget(target, button);
            break;
        case 'upgrade-venv':
            openVenvUpgradeWizard(target, baseVersion);
            break;
        case 'delete-venv':
            deleteVenv(target);
            break;
        default:
            break;
    }
}

function bindDelegatedActions() {
    document.getElementById('main-content')?.addEventListener('click', handleDelegatedAction);
}

// Workspace context logic
let currentWorkspaceDir = localStorage.getItem('pyenv-workspace-dir') || '';

function getWorkspaceDir() {
    return currentWorkspaceDir || null;
}

function updateWorkspaceUI() {
    const el = document.getElementById('workspace-path');
    const btn = document.getElementById('btn-change-workspace');
    if (el) {
        el.textContent = currentWorkspaceDir || 'No project folder selected';
        el.title = currentWorkspaceDir
            ? currentWorkspaceDir
            : 'Choose a folder so Make Local knows where to write .python-version.';
    }
    if (btn) {
        btn.textContent = currentWorkspaceDir ? 'Change Folder' : 'Choose Folder';
        btn.title = currentWorkspaceDir
            ? 'Switch the project folder used for Make Local.'
            : 'Select the project folder used for Make Local.';
    }
}

// Bind Change Workspace Button
document.addEventListener('DOMContentLoaded', () => {
    applyAccessibilityPreferences();
    bindAccessibilityPreferenceControls();
    bindConfigControls();
    bindDelegatedActions();
    document.getElementById('btn-reset-a11y')?.addEventListener('click', resetAccessibilityPreferences);
    document.getElementById('chk-latest-only')?.addEventListener('change', () => applyAvailableFilters());
    bindCatalogFamilyFilters();
    document.getElementById('btn-open-website')?.addEventListener('click', () => openExternal('https://imyourboyroy.com'));
    document.getElementById('btn-open-github')?.addEventListener('click', () => openExternal('https://github.com/imyourboyroy/pyenv-native'));
    document.getElementById('btn-open-pypi')?.addEventListener('click', () => openExternal('https://pypi.org/user/ImYourBoyRoy/'));
    document.getElementById('footer-btn')?.addEventListener('click', () => checkUpdates());
    document.querySelectorAll('.view').forEach((view) => {
        if (view.id !== 'view-dashboard') {
            view.setAttribute('aria-hidden', 'true');
        }
    });
    updateWorkspaceUI();
    document.addEventListener('click', (event) => {
        if (!event.target.closest('.card-menu')) closeCardMenus();
    });
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
                showAlert('Project Folder', `Now using:<br><code style="font-size: 12px; opacity: 0.8;">${escapeHtml(selectedDir)}</code>`);
            }
        } catch (err) {
            showAlert('Error', 'Failed to change workspace directory:\n' + escapeHtml(err));
        }
    });
});

// ─── Custom Modal System ───
function showModal(title, message, buttons = [{label: 'OK', style: 'btn-primary', primary: true}], options = {}) {
    return new Promise(resolve => {
        const root = document.getElementById('modal-root');
        const overlay = document.createElement('div');
        overlay.className = 'modal-overlay';

        const dialog = document.createElement('div');
        dialog.className = 'modal-box';
        dialog.setAttribute('role', options.alert ? 'alertdialog' : 'dialog');
        dialog.setAttribute('aria-modal', 'true');

        const heading = document.createElement('h3');
        heading.textContent = title;
        dialog.setAttribute('aria-labelledby', heading.id = `modal-title-${Date.now()}`);

        const body = document.createElement('div');
        body.className = 'modal-message';
        appendRichMessage(body, message);
        dialog.setAttribute('aria-describedby', body.id = `modal-body-${Date.now()}`);

        const btnContainer = document.createElement('div');
        btnContainer.className = 'modal-actions';

        const createdButtons = [];
        buttons.forEach((b) => {
            const btn = document.createElement('button');
            btn.type = 'button';
            btn.className = `btn ${b.style || 'btn-outline'}`;
            btn.textContent = b.label;
            createdButtons.push({ btn, config: b });
            btnContainer.appendChild(btn);
        });

        dialog.append(heading, body, btnContainer);
        overlay.appendChild(dialog);

        let closed = false;
        const closeModal = (resultIndex) => {
            if (closed) return;
            closed = true;
            unlockAppShell();
            root.removeChild(overlay);
            document.removeEventListener('keydown', onKeyDown);
            if (lastFocusedBeforeOverlay) lastFocusedBeforeOverlay.focus();
            resolve(resultIndex === 0);
        };

        createdButtons.forEach(({ btn }, index) => {
            btn.addEventListener('click', () => closeModal(index));
        });

        const onKeyDown = (event) => {
            if (event.key === 'Escape') {
                const cancelIndex = Math.max(buttons.length - 1, 0);
                closeModal(cancelIndex);
                return;
            }
            trapFocus(overlay, event);
        };

        overlay.addEventListener('click', (event) => {
            if (event.target === overlay) {
                closeModal(Math.max(buttons.length - 1, 0));
            }
        });

        lastFocusedBeforeOverlay = document.activeElement;
        lockAppShell();
        root.appendChild(overlay);
        document.addEventListener('keydown', onKeyDown);
        const primaryIndex = createdButtons.findIndex(({ config }) => config.primary);
        const initial = createdButtons[primaryIndex >= 0 ? primaryIndex : createdButtons.length - 1]?.btn
            || createdButtons[0]?.btn;
        initial?.focus();
    });
}

async function showAlert(title, message) {
    return showModal(title, message, [{label: 'OK', style: 'btn-primary', primary: true}]);
}

async function showConfirm(title, message) {
    return showModal(title, message, [
        {label: 'Confirm', style: 'btn-danger'},
        {label: 'Cancel', style: 'btn-outline', primary: true}
    ], { alert: true });
}

function openExternal(url) {
    invoke('open_url', { url }).catch((err) => {
        const detail = err ? String(err) : 'unknown error';
        showToast(`Could not open link: ${detail}`);
        showAlert('Open Link Failed', `Could not open <code>${escapeHtml(url)}</code>:\n${escapeHtml(detail)}`);
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
    closeCardMenus();
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
    if (heading && lastNavInput === 'keyboard') {
        heading.setAttribute('tabindex', '-1');
        heading.focus({ preventScroll: true });
    }
    lastNavInput = 'pointer';
    document.title = `${VIEW_TITLES[viewId] || 'Pyenv Native'} · Pyenv Native`;
    announce(`Showing ${VIEW_TITLES[viewId] || 'view'}`);
}

function navigateToView(viewId) {
    const navItem = document.querySelector(`.nav-btn[data-view="${viewId}"]`);
    if (!navItem) return;
    navItem.click();
}

function versionKeyAliases(value) {
    const normalized = String(value ?? '').trim().replace(/\\/g, '/');
    const aliases = new Set([normalized]);
    if (!normalized) return aliases;
    const withoutPrefix = normalized.replace(/^venv:/i, '');
    aliases.add(withoutPrefix);
    const marker = '/envs/';
    const idx = withoutPrefix.indexOf(marker);
    if (idx >= 0) {
        const name = withoutPrefix.slice(idx + marker.length);
        if (name && !name.includes('/')) {
            aliases.add(`venv:${name}`);
            aliases.add(name);
        }
    } else if (withoutPrefix && withoutPrefix !== normalized) {
        aliases.add(`venv:${withoutPrefix}`);
    }
    return aliases;
}

function isGlobalVersion(versionName, globalVersions) {
    const aliases = versionKeyAliases(versionName);
    if (!globalVersions?.length) {
        return aliases.has('system');
    }
    return globalVersions.some((entry) => {
        const other = versionKeyAliases(entry);
        for (const alias of aliases) {
            if (other.has(alias)) return true;
        }
        return false;
    });
}

function venvSpec(v) {
    return String(v.spec || `${v.base_version}/envs/${v.name}`).replace(/\\/g, '/');
}

function venvDisplayName(specOrName) {
    const value = String(specOrName ?? '').trim().replace(/\\/g, '/');
    const marker = '/envs/';
    const idx = value.indexOf(marker);
    if (idx >= 0) return value.slice(idx + marker.length);
    return value.replace(/^venv:/i, '');
}

function pushGlobalMenuItem(items, versionName, globalVersions) {
    if (isGlobalVersion(versionName, globalVersions)) return;
    items.push({
        label: 'Make Global',
        action: 'set-global',
        target: versionName,
        title: 'Use this as the default for new shells.',
    });
}

function localMenuItems(target) {
    return [
        {
            label: 'Make Local',
            action: 'set-local',
            target,
            title: 'Write .python-version in the current project folder.',
        },
        {
            label: 'Pin to another folder…',
            action: 'set-local-pick',
            target,
            title: 'Write .python-version in a folder you pick.',
        },
    ];
}

function appendMenuGroup(items, group) {
    if (!group.length) return;
    if (items.length) items.push({ separator: true });
    items.push(...group);
}

let cardMenuSeq = 0;

function closeCardMenus(exceptWrap = null) {
    document.querySelectorAll('.card-menu').forEach((wrap) => {
        if (wrap === exceptWrap) return;
        wrap.classList.remove('is-open');
        wrap.closest('.version-card')?.classList.remove('is-menu-open');
        const toggle = wrap.querySelector('.card-menu-toggle');
        const menu = wrap.querySelector('.card-menu-list');
        if (toggle) toggle.setAttribute('aria-expanded', 'false');
        if (menu) {
            menu.hidden = true;
            menu.classList.remove('opens-up');
        }
    });
}

function positionCardMenu(menu) {
    menu.classList.remove('opens-up');
    const host = document.getElementById('main-content');
    if (!host) return;
    const menuRect = menu.getBoundingClientRect();
    const hostRect = host.getBoundingClientRect();
    if (menuRect.bottom > hostRect.bottom - 8) {
        menu.classList.add('opens-up');
    }
}

function createOverflowMenu(items) {
    if (!items.length) return null;
    const menuId = `card-menu-${++cardMenuSeq}`;
    const wrap = document.createElement('div');
    wrap.className = 'card-menu';
    const toggle = document.createElement('button');
    toggle.type = 'button';
    toggle.className = 'btn btn-outline card-menu-toggle';
    toggle.setAttribute('aria-haspopup', 'menu');
    toggle.setAttribute('aria-expanded', 'false');
    toggle.setAttribute('aria-controls', menuId);
    toggle.setAttribute('aria-label', 'More actions');
    toggle.title = 'More actions for this item.';
    toggle.textContent = 'More';
    const menu = document.createElement('div');
    menu.id = menuId;
    menu.className = 'card-menu-list';
    menu.setAttribute('role', 'menu');
    menu.hidden = true;
    items.forEach((item) => {
        if (item.separator) {
            const sep = document.createElement('div');
            sep.className = 'card-menu-sep';
            sep.setAttribute('role', 'separator');
            menu.appendChild(sep);
            return;
        }
        const className = item.danger ? 'card-menu-item is-danger' : 'card-menu-item';
        const btn = createActionButton(item.label, item.action, item.target, className, item.extra || {});
        btn.setAttribute('role', 'menuitem');
        if (item.title) btn.title = item.title;
        menu.appendChild(btn);
    });
    toggle.addEventListener('click', (event) => {
        event.stopPropagation();
        const willOpen = menu.hidden;
        closeCardMenus(willOpen ? wrap : null);
        wrap.classList.toggle('is-open', willOpen);
        wrap.closest('.version-card')?.classList.toggle('is-menu-open', willOpen);
        toggle.setAttribute('aria-expanded', willOpen ? 'true' : 'false');
        menu.hidden = !willOpen;
        if (willOpen) {
            positionCardMenu(menu);
            menu.querySelector('[role="menuitem"]')?.focus();
        }
    });
    wrap.addEventListener('keydown', (event) => {
        if (event.key === 'Escape') {
            event.stopPropagation();
            closeCardMenus();
            toggle.focus();
            return;
        }
        if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
        const itemEls = Array.from(menu.querySelectorAll('[role="menuitem"]'));
        if (!itemEls.length) return;
        event.preventDefault();
        const idx = itemEls.indexOf(document.activeElement);
        let next = 0;
        if (event.key === 'ArrowDown') next = idx < 0 ? 0 : (idx + 1) % itemEls.length;
        else if (event.key === 'ArrowUp') next = idx <= 0 ? itemEls.length - 1 : idx - 1;
        else if (event.key === 'End') next = itemEls.length - 1;
        itemEls[next].focus();
    });
    wrap.append(toggle, menu);
    return wrap;
}

function appendCardOverflow(actions, items) {
    const menu = createOverflowMenu(items);
    if (menu) actions.appendChild(menu);
}

function appendBadge(parent, text, className, title) {
    const badge = document.createElement('span');
    badge.className = className;
    badge.textContent = text;
    if (title) badge.title = title;
    parent.appendChild(document.createTextNode(' '));
    parent.appendChild(badge);
}

sidebarNavItems.forEach((item) => {
    item.addEventListener('pointerdown', () => { lastNavInput = 'pointer'; });
    item.addEventListener('keydown', (event) => {
        if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
            lastNavInput = 'keyboard';
        }
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
});

// Escape-to-close is registered after drawer elements are defined below.

// ─── Dashboard ───
function setDashboardHealth(kind, title, body) {
    const banner = document.getElementById('dashboard-health');
    const titleEl = document.getElementById('dashboard-health-title');
    const bodyEl = document.getElementById('dashboard-health-body');
    if (!banner || !titleEl || !bodyEl) return;
    titleEl.textContent = title;
    bodyEl.textContent = body;
    banner.hidden = kind === 'ok';
    banner.dataset.kind = kind;
}

function doctorIssueStatus(status) {
    const normalized = String(status || '').toUpperCase();
    return normalized === 'WARN' || normalized === 'WARNING' || normalized === 'FAIL' || normalized === 'ERROR' || normalized === 'BLOCKED';
}

function normalizeDoctorCheck(raw) {
    if (!raw || typeof raw !== 'object') return null;
    const name = String(raw.name || raw.id || raw.title || '').trim();
    const detail = String(raw.detail || raw.message || raw.reason || '').trim();
    const status = String(raw.status || '').toUpperCase();
    if (!name && !detail) return null;
    return {
        name: name || 'Health check',
        detail,
        status,
    };
}

async function refreshDashboardHealth() {
    const banner = document.getElementById('dashboard-health');
    if (!banner) return;
    try {
        const jsonStr = await invoke('run_doctor', { workspaceDir: getWorkspaceDir() });
        const checks = typeof jsonStr === 'string' ? JSON.parse(jsonStr) : jsonStr;
        if (!Array.isArray(checks)) {
            setDashboardHealth('error', 'Diagnostics unavailable', 'Doctor did not return a usable report.');
            return;
        }
        const issues = checks
            .map(normalizeDoctorCheck)
            .filter((check) => check && doctorIssueStatus(check.status))
            .map((check) => ({
                ...check,
                detail: check.detail || 'No additional detail was reported for this check.',
            }));
        if (issues.length === 0) {
            setDashboardHealth('ok', 'System health', 'No doctor warnings.');
            return;
        }
        const preview = issues.slice(0, 2).map((issue) => `${issue.name}: ${issue.detail}`).join(' · ');
        const extra = issues.length > 2 ? ` · +${issues.length - 2} more` : '';
        setDashboardHealth('warn', `${issues.length} health issue${issues.length === 1 ? '' : 's'}`, preview + extra);
    } catch (err) {
        setDashboardHealth('error', 'Diagnostics unavailable', String(err));
    }
}

async function loadDashboard() {
    try {
        const jsonStr = await invoke('get_status', { workspaceDir: getWorkspaceDir() });
        const status = JSON.parse(jsonStr);
        
        const activeVersionEl = document.getElementById('active-version');
        activeVersionEl.textContent = status.active_versions.length ? status.active_versions.join(', ') : 'None';
        activeVersionEl.title = activeVersionEl.textContent;
        const globalLabel = status.global_versions?.length
            ? status.global_versions.join(', ')
            : 'system';
        const originEl = document.getElementById('active-origin');
        originEl.textContent =
            `Origin: ${status.origin} • Global: ${globalLabel}`;
        originEl.title = originEl.textContent;
        
        if (status.managed_venv) {
            const venvEl = document.getElementById('active-venv');
            venvEl.textContent = status.managed_venv.name;
            venvEl.title = status.managed_venv.spec || status.managed_venv.name;
            document.getElementById('active-venv-base').textContent = `Base: ${status.managed_venv.base_version}`;
        } else {
            document.getElementById('active-venv').textContent = 'None';
            document.getElementById('active-venv').title = 'No managed venv is currently selected.';
            document.getElementById('active-venv-base').textContent = 'Integrated Project Env';
        }

        document.getElementById('sys-root').textContent = status.root;
        document.getElementById('sys-root').title = String(status.root || '');
        refreshDashboardHealth();

    } catch (err) {
        console.error("Failed to load status:", err);
        document.getElementById('active-version').textContent = "Error";
        document.getElementById('active-origin').textContent = String(err);
        setDashboardHealth('error', 'Could not load environment status.', String(err));
    }
}

// ─── Installed Versions ───
async function loadInstalled() {
    const list = document.getElementById('installed-list');
    const summary = document.getElementById('installed-global-summary');
    setRegionLoading(list, true, 'Loading installed runtimes…');

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
            summary.replaceChildren();
            const row = document.createElement('div');
            row.style.cssText = 'display:flex;justify-content:space-between;align-items:center;gap:16px;flex-wrap:wrap;';

            const info = document.createElement('div');
            const eyebrow = document.createElement('div');
            eyebrow.style.cssText = 'font-size:12px;color:var(--text-muted);margin-bottom:4px;';
            eyebrow.textContent = 'Current setup';
            const activeLine = document.createElement('div');
            activeLine.style.cssText = 'font-size:14px;font-weight:600;';
            activeLine.textContent = `Active: ${activeLabel} • Global: ${globalLabel}`;
            const originLine = document.createElement('div');
            originLine.style.cssText = 'font-size:12px;color:var(--text-muted);margin-top:6px;';
            originLine.textContent = `Origin: ${status.origin}`;
            info.append(eyebrow, activeLine, originLine);

            const installBtn = createActionButton('Install New Runtime', 'navigate', 'view-available', 'btn btn-primary', {
                title: 'Open the install catalog.',
            });
            installBtn.style.fontSize = '12px';
            row.append(info, installBtn);
            summary.appendChild(row);
        }

        list.replaceChildren();
        list.setAttribute('aria-busy', 'false');

        const displayVersions = [];
        const hasRealSystem = await detectSystemPython();
        displayVersions.push({ name: 'system', isSystem: true, available: hasRealSystem });
        versions.forEach(v => {
            if (v !== 'system') {
                displayVersions.push({ name: v, isSystem: false, available: true });
            }
        });

        const managedVersions = displayVersions.filter(entry => !entry.isSystem);
        if (managedVersions.length === 0) {
            const empty = document.createElement('div');
            empty.className = 'empty-state';
            empty.style.padding = '32px 16px';
            const title = document.createElement('div');
            title.style.cssText = 'font-size:15px;font-weight:600;margin-bottom:8px;';
            title.textContent = 'No managed runtimes installed yet';
            const hint = document.createElement('div');
            hint.style.cssText = 'font-size:13px;color:var(--text-muted);margin-bottom:16px;';
            hint.textContent = 'Install a Python 2 or 3 runtime to get started.';
            empty.append(title, hint, createActionButton('Install Runtimes', 'navigate', 'view-available', 'btn btn-primary', {
                title: 'Open the install catalog.',
            }));
            list.appendChild(empty);
            announce('No managed runtimes installed');
            return;
        }

        displayVersions.forEach(entry => {
            const card = document.createElement('div');
            card.className = 'version-card';
            card.dataset.version = entry.name;
            const isGlobal = isGlobalVersion(entry.name, globalVersions);

            const info = document.createElement('div');
            info.className = 'version-info';
            const nameEl = document.createElement('div');
            nameEl.className = 'version-name';
            nameEl.append(document.createTextNode(entry.name));

            const meta = document.createElement('div');
            meta.className = 'version-meta';

            if (entry.isSystem) {
                appendBadge(nameEl, entry.available ? 'Detected' : 'Not Available', entry.available ? 'system-badge badge-success' : 'system-badge badge-muted', entry.available ? 'A system Python was found on PATH.' : 'No usable system Python (Store alias only).');
                if (isGlobal) appendBadge(nameEl, 'Global', 'badge badge-success', 'Default for new shells.');
                meta.textContent = entry.available
                    ? 'System-wide Python installation'
                    : 'No system Python found (Microsoft Store alias detected)';
            } else {
                if (isGlobal) appendBadge(nameEl, 'Global', 'badge badge-success', 'Default for new shells.');
                meta.textContent = isGlobal ? 'Configured as the global default runtime' : 'Installed managed runtime';
            }
            info.append(nameEl, meta);

            const actions = document.createElement('div');
            actions.className = 'version-actions';
            if (!entry.isSystem || entry.available) {
                actions.appendChild(createActionButton('Package Explorer', 'package-explorer', entry.name, 'btn btn-outline', {
                    title: 'Browse and update packages in this runtime.',
                }));
            }
            const menuItems = [];
            if (!entry.isSystem) {
                const globalItems = [];
                pushGlobalMenuItem(globalItems, entry.name, globalVersions);
                appendMenuGroup(menuItems, globalItems);
                appendMenuGroup(menuItems, localMenuItems(entry.name));
                appendMenuGroup(menuItems, [{
                    label: 'Uninstall',
                    action: 'uninstall',
                    target: entry.name,
                    danger: true,
                    title: 'Remove this runtime from pyenv.',
                }]);
                appendCardOverflow(actions, menuItems);
            } else if (entry.available) {
                pushGlobalMenuItem(menuItems, 'system', globalVersions);
                appendCardOverflow(actions, menuItems);
            }

            card.append(info, actions);
            list.appendChild(card);
        });
        announce(`Loaded ${managedVersions.length} managed runtime${managedVersions.length === 1 ? '' : 's'}`);
    } catch (err) {
        console.error("Failed to load installed:", err);
        showEmptyState(list, 'Failed to load installed runtimes.', { danger: true });
        announce('Failed to load installed runtimes');
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

function runtimeSpec(entry) {
    return typeof entry === 'string' ? entry : (entry.name || entry.spec || String(entry));
}

function runtimeFamily(spec) {
    const name = String(spec || '').toLowerCase();
    if (name.startsWith('pypy')) return 'pypy';
    const match = name.match(/^(\d+)/);
    if (match) return match[1] === '2' ? 'cpython2' : 'cpython3';
    return 'other';
}

function runtimeFamilyLabel(family) {
    switch (family) {
        case 'cpython3': return 'Python 3';
        case 'cpython2': return 'Python 2';
        case 'pypy': return 'PyPy';
        default: return 'Other';
    }
}

function versionNumericParts(name) {
    return String(name).split(/[^\d]+/).filter(Boolean).map((part) => Number.parseInt(part, 10));
}

function compareVersionsDesc(left, right) {
    const leftParts = versionNumericParts(left);
    const rightParts = versionNumericParts(right);
    const length = Math.max(leftParts.length, rightParts.length);
    for (let i = 0; i < length; i += 1) {
        const delta = (rightParts[i] || 0) - (leftParts[i] || 0);
        if (delta) return delta;
    }
    return String(right).localeCompare(String(left));
}

function latestPatchPerLine(versions) {
    const best = new Map();
    versions.forEach((entry) => {
        const spec = runtimeSpec(entry);
        if (/^\d/.test(spec) && spec.endsWith('t')) return;
        let key = spec.toLowerCase();
        const cpython = spec.match(/^(\d+)\.(\d+)/);
        const pypy = spec.match(/^(pypy\d+(?:\.\d+)?)/i);
        if (cpython) key = `cpython:${cpython[1]}.${cpython[2]}`;
        else if (pypy) key = pypy[1].toLowerCase();
        const previous = best.get(key);
        if (!previous || compareVersionsDesc(spec, previous) < 0) {
            best.set(key, spec);
        }
    });
    return [...best.values()];
}

function bindCatalogFamilyFilters() {
    const chips = document.querySelectorAll('.filter-chip[data-family]');
    chips.forEach((chip) => {
        chip.addEventListener('click', () => {
            chips.forEach((other) => other.setAttribute('aria-checked', other === chip ? 'true' : 'false'));
            applyAvailableFilters();
        });
    });
}

function applyAvailableFilters() {
    const list = document.getElementById('available-list');
    const countEl = document.getElementById('available-count');
    const query = (document.getElementById('available-search')?.value || '').trim().toLowerCase();
    const family = document.querySelector('.filter-chip[aria-checked="true"]')?.dataset.family || 'all';
    const latestOnly = Boolean(document.getElementById('chk-latest-only')?.checked);
    let displayList = [...fullAvailableCache];
    if (latestOnly) displayList = latestPatchPerLine(displayList);
    if (family !== 'all') {
        displayList = displayList.filter((entry) => runtimeFamily(runtimeSpec(entry)) === family);
    }
    if (query) {
        displayList = displayList.filter((entry) => runtimeSpec(entry).toLowerCase().includes(query));
    }
    displayList.sort((left, right) => {
        const leftFamily = runtimeFamily(runtimeSpec(left));
        const rightFamily = runtimeFamily(runtimeSpec(right));
        const order = { cpython3: 0, cpython2: 1, pypy: 2, other: 3 };
        const familyDelta = (order[leftFamily] ?? 9) - (order[rightFamily] ?? 9);
        if (familyDelta) return familyDelta;
        return compareVersionsDesc(runtimeSpec(left), runtimeSpec(right));
    });
    window.availableVersionsCache = displayList;
    if (countEl) {
        const python2 = displayList.filter((entry) => runtimeFamily(runtimeSpec(entry)) === 'cpython2').length;
        const total = fullAvailableCache.length;
        countEl.textContent = `Showing ${displayList.length} of ${total} runtimes${python2 ? ` · ${python2} Python 2` : ''}. Uncheck latest patch to see every release.`;
    }
    if (list) renderAvailable(displayList);
}

let availableLoaded = false;
async function setupAvailableView() {
    if (availableLoaded) return;
    const list = document.getElementById('available-list');
    setRegionLoading(list, true, 'Loading installable runtimes…');
    try {
        const installedStr = await invoke('get_installed_versions', { workspaceDir: getWorkspaceDir() });
        installedVersionsSet = JSON.parse(installedStr);

        if (fullAvailableCache.length === 0) {
            const jsonStr = await invoke('get_available_versions', { workspaceDir: getWorkspaceDir(), family: null, pattern: null });
            const data = JSON.parse(jsonStr);
            const groups = (data.results || data);
            groups.forEach((group) => {
                if (group.versions) group.versions.forEach((version) => fullAvailableCache.push(version));
                else fullAvailableCache.push(group.name || group.spec || group);
            });
        }

        applyAvailableFilters();
        availableLoaded = true;
    } catch (err) {
        console.error(err);
        showEmptyState(list, 'Failed to load catalog. Launch this window with `pyenv gui` to talk to the backend.', { danger: true });
        announce('Failed to load installable runtimes');
    }
}

function renderAvailable(items) {
    const list = document.getElementById('available-list');
    list.replaceChildren();
    list.setAttribute('aria-busy', 'false');
    if (!items || items.length === 0) {
        showEmptyState(list, 'No runtimes match these filters.');
        announce('No installable runtimes found');
        return;
    }

    let lastFamily = null;
    items.forEach((entry) => {
        const spec = runtimeSpec(entry);
        const family = runtimeFamily(spec);
        if (family !== lastFamily) {
            const heading = document.createElement('h2');
            heading.className = 'catalog-group-title';
            heading.textContent = runtimeFamilyLabel(family);
            list.appendChild(heading);
            lastFamily = family;
        }

        const card = document.createElement('div');
        card.className = 'version-card';

        const info = document.createElement('div');
        info.className = 'version-info';
        const nameEl = document.createElement('div');
        nameEl.className = 'version-name';
        nameEl.textContent = spec;
        if (installedVersionsSet.includes(spec)) {
            appendBadge(nameEl, 'Installed', 'badge badge-success', 'Already installed on this machine.');
        }
        info.appendChild(nameEl);

        const actions = document.createElement('div');
        actions.className = 'version-actions';
        if (installedVersionsSet.includes(spec)) {
            const button = document.createElement('button');
            button.type = 'button';
            button.className = 'btn btn-outline';
            button.disabled = true;
            button.textContent = 'Installed';
            button.title = `${spec} is already installed.`;
            actions.appendChild(button);
        } else {
            actions.appendChild(createActionButton('Install', 'install', spec, 'btn btn-primary', {
                title: `Download and install ${spec}.`,
            }));
        }

        card.append(info, actions);
        list.appendChild(card);
    });
    announce(`Showing ${items.length} installable runtime${items.length === 1 ? '' : 's'}`);
}

// ─── Virtual Environments ───
async function loadVenvs() {
    const list = document.getElementById('venvs-list');
    setRegionLoading(list, true, 'Loading virtual environments…');
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
        list.replaceChildren();
        list.setAttribute('aria-busy', 'false');
        if (venvs.length === 0) {
            showEmptyState(list, 'No virtual environments found.');
            announce('No virtual environments found');
        } else {
            venvs.forEach(v => {
                const spec = venvSpec(v);
                const isGlobal = isGlobalVersion(spec, globalVersions);
                const isActive = isGlobalVersion(spec, status.active_versions)
                    || (status.managed_venv && isGlobalVersion(spec, [status.managed_venv.spec]));
                const card = document.createElement('div');
                card.className = isActive ? 'version-card is-active' : 'version-card';
                card.dataset.version = spec;

                const info = document.createElement('div');
                info.className = 'version-info';
                const nameEl = document.createElement('div');
                nameEl.className = 'version-name';
                nameEl.textContent = v.name;
                if (isActive) appendBadge(nameEl, 'Active', 'badge badge-success', 'Currently selected environment.');
                if (isGlobal) appendBadge(nameEl, 'Global', 'badge badge-success', 'Default for new shells.');
                const meta = document.createElement('div');
                meta.className = 'version-meta';
                meta.textContent = `Base ${v.base_version} · ${v.path}`;
                meta.title = v.path;
                info.append(nameEl, meta);

                const actions = document.createElement('div');
                actions.className = 'version-actions';
                actions.appendChild(createActionButton('Package Explorer', 'package-explorer', spec, 'btn btn-outline', {
                    title: 'Browse and update packages in this venv.',
                }));
                const menuItems = [];
                const globalItems = [];
                pushGlobalMenuItem(globalItems, spec, globalVersions);
                appendMenuGroup(menuItems, globalItems);
                appendMenuGroup(menuItems, localMenuItems(spec));
                appendMenuGroup(menuItems, [{
                    label: 'Migrate Runtime',
                    action: 'upgrade-venv',
                    target: spec,
                    extra: { baseVersion: v.base_version },
                    title: `Recreate this venv on another installed Python (now ${v.base_version}).`,
                }]);
                appendMenuGroup(menuItems, [{
                    label: 'Delete',
                    action: 'delete-venv',
                    target: spec,
                    danger: true,
                    title: 'Permanently delete this virtual environment.',
                }]);
                appendCardOverflow(actions, menuItems);
                card.append(info, actions);
                list.appendChild(card);
            });
            announce(`Loaded ${venvs.length} virtual environment${venvs.length === 1 ? '' : 's'}`);
        }
        const sysInfo = await invoke('get_installed_versions', { workspaceDir: getWorkspaceDir() });
        const bases = JSON.parse(sysInfo);
        const sel = document.getElementById('venv-base-version');
        sel.replaceChildren();
        const placeholder = document.createElement('option');
        placeholder.value = '';
        placeholder.textContent = 'Select base runtime…';
        sel.appendChild(placeholder);
        bases.forEach(b => {
            const option = document.createElement('option');
            option.value = b;
            option.textContent = b;
            sel.appendChild(option);
        });
    } catch (err) {
        console.error("Failed to load venvs:", err);
        showEmptyState(list, 'Failed to load virtual environments.', { danger: true });
        announce('Failed to load virtual environments');
    }
}

document.getElementById('available-search').addEventListener('input', () => {
    applyAvailableFilters();
});

// ─── Actions ───
async function installTarget(v, btnEl) {
    btnEl.disabled = true;
    const originalText = btnEl.textContent;
    btnEl.textContent = 'Installing…';
    btnEl.setAttribute('aria-busy', 'true');
    
    try {
        await invoke('install_version', { workspaceDir: getWorkspaceDir(), version: v });
        btnEl.textContent = 'Installed ✓';
        btnEl.classList.remove('btn-primary');
        btnEl.classList.add('btn-outline');
        installedVersionsSet.push(v);
        loadDashboard();
        loadInstalled();
        announce(`Installed ${v}`);
    } catch (err) {
        console.error("Install failed:", err);
        btnEl.textContent = originalText;
        btnEl.disabled = false;
        showAlert('Install Failed', escapeHtml(String(err)));
    } finally {
        btnEl.removeAttribute('aria-busy');
    }
}

async function setGlobal(v) {
    try {
        await invoke('set_global', { workspaceDir: getWorkspaceDir(), version: v });
        loadDashboard();
        loadInstalled();
        loadVenvs();
        showAlert('Global Version Set', `Python <b>${escapeHtml(v)}</b> is now the global default.`);
    } catch(err) {
        showAlert('Error', 'Failed to set global version:\n' + escapeHtml(err));
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
        showAlert('Venv Creation Failed', escapeHtml(err));
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
        showAlert('Delete Failed', escapeHtml(err));
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
            showAlert('Up to Date', escapeHtml(result));
        } else if (result.includes('Update available') || result.includes('newer')) {
            const yes = await showConfirm('Update Available', escapeHtml(result) + '<br><br>Would you like to update now?');
            if (yes) {
                if(btn) btn.innerText = "Updating…";
                try {
                    await invoke('perform_update', { workspaceDir: getWorkspaceDir() });
                    // Close immediately so the background updater can finish and relaunch.
                    await invoke('close_app');
                } catch(updateErr) {
                    showAlert('Update Failed', escapeHtml(updateErr));
                }
            }
        } else {
            // Generic result
            showAlert('Update Check', escapeHtml(result));
        }
    } catch(err) {
        showAlert('Update Check Failed', escapeHtml(err));
    } finally {
        if(btn) { btn.innerText = "Check for Updates"; btn.disabled = false; }
    }
}

async function setLocal(v, { pickFolder = false } = {}) {
    try {
        let selectedDir = null;
        if (!pickFolder && currentWorkspaceDir) {
            const yes = await showConfirm(
                'Make Local',
                `Pin <b>${escapeHtml(v)}</b> to the current project folder?<br><code style="font-size: 12px; opacity: 0.8;">${escapeHtml(currentWorkspaceDir)}</code>`,
            );
            if (!yes) return;
            selectedDir = currentWorkspaceDir;
        } else {
            selectedDir = await invoke('select_directory');
        }
        if (!selectedDir) {
            if (!pickFolder && !currentWorkspaceDir) {
                showAlert(
                    'No project folder',
                    'Choose a project folder in the bar above, or use <b>Pin to another folder</b> from More.',
                );
            }
            return;
        }
        await invoke('set_local', { version: v, path: selectedDir });
        loadDashboard();
        loadInstalled();
        showAlert('Local Version Set', `Python <b>${escapeHtml(v)}</b> pinned to:<br><code style="font-size: 12px; opacity: 0.8;">${escapeHtml(selectedDir)}</code>`);
    } catch (err) {
        showAlert('Error', 'Failed to set local version:\n' + escapeHtml(err));
    }
}

// ─── Config ───
function formatBytes(bytes) {
    if (!bytes || bytes <= 0) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    let value = bytes;
    let unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
        value /= 1024;
        unit += 1;
    }
    return `${value.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

async function loadCacheStats() {
    const host = document.getElementById('cache-stats');
    if (!host) return;
    try {
        const stats = await invoke('get_cache_stats', { workspaceDir: getWorkspaceDir() });
        host.replaceChildren();
        const total = document.createElement('div');
        total.className = 'cache-total';
        total.textContent = `Total tracked: ${formatBytes(stats.total_bytes)}`;
        host.appendChild(total);
        (stats.entries || []).forEach((entry) => {
            const row = document.createElement('div');
            row.className = 'cache-row';
            const label = document.createElement('div');
            label.className = 'cache-row-label';
            label.textContent = entry.name;
            const meta = document.createElement('div');
            meta.className = 'cache-row-meta';
            meta.textContent = `${formatBytes(entry.bytes)} · ${entry.exists ? entry.path : 'not present'}`;
            row.append(label, meta);
            host.appendChild(row);
        });
    } catch (err) {
        host.replaceChildren();
        const empty = document.createElement('div');
        empty.className = 'empty-state';
        empty.style.padding = '12px';
        empty.textContent = `Could not read cache stats: ${err}`;
        host.appendChild(empty);
    }
}

async function purgeCacheTarget(target, label) {
    const yes = await showConfirm(
        'Purge Cache',
        `Safely purge <b>${label}</b>?<br><br>Installed Python versions and virtual environments are <b>not</b> deleted.`,
    );
    if (!yes) return;
    try {
        const result = await invoke('purge_cache', { workspaceDir: getWorkspaceDir(), target });
        showAlert('Cache Purged', escapeHtml(result));
        loadCacheStats();
    } catch (err) {
        showAlert('Purge Failed', escapeHtml(err));
    }
}

async function loadConfig() {
    try {
        const jsonStr = await invoke('get_config', { workspaceDir: getWorkspaceDir() });
        const config = JSON.parse(jsonStr);
        const platform = window.__installPlatform
            || (await invoke('check_install_status').then((s) => s.platform).catch(() => 'linux'));
        window.__installPlatform = platform;

        const windowsBlock = document.getElementById('settings-windows');
        if (windowsBlock) {
            windowsBlock.hidden = platform !== 'windows';
        }

        const registry = document.getElementById('config-windows.registry_mode');
        if (registry) {
            registry.value = config.windows?.registry_mode || 'disabled';
        }
        document.getElementById('config-install.arch').value = config.install?.arch || 'auto';
        document.getElementById('config-install.bootstrap_pip').checked = config.install?.bootstrap_pip ?? true;
        document.getElementById('config-venv.auto_create_base_venv').checked = config.venv?.auto_create_base_venv ?? false;
        document.getElementById('config-venv.auto_use_base_venv').checked = config.venv?.auto_use_base_venv ?? false;
        loadCacheStats();
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
        showAlert('Config Error', 'Failed to save config:\n' + escapeHtml(err));
    }
}

// ─── Uninstall ───
async function uninstallVersion(v) {
    const yes = await showConfirm('Uninstall Python', `Are you sure you want to completely remove Python <b>${escapeHtml(v)}</b>? This will delete all files for this version.`);
    if (!yes) return;
    
    // Show inline progress
    const targetCard = Array.from(document.querySelectorAll('#installed-list .version-card'))
        .find((card) => card.dataset.version === v);
    if (targetCard) {
        const actionsDiv = targetCard.querySelector('.version-actions');
        showInlineRemovingState(actionsDiv);
    }
    
    try {
        await invoke('uninstall_version', { workspaceDir: getWorkspaceDir(), version: v });
        installedVersionsSet = installedVersionsSet.filter(x => x !== v);
        showAlert('Uninstalled', `Python <b>${escapeHtml(v)}</b> has been removed.`);
        loadInstalled();
        loadDashboard();
    } catch(err) {
        showAlert('Uninstall Failed', escapeHtml(err));
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
                if (compareVersions(latest, version) > 0) {
                    setFooterAppStatus('update', latest);
                } else {
                    setFooterAppStatus('uptodate');
                }
            }
        } catch {
            setFooterAppStatus('offline');
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
        const title = document.getElementById('setup-banner-title');
        const body = document.getElementById('setup-banner-body');
        const actionBtn = document.getElementById('btn-run-setup-banner');
        window.__installPlatform = status.platform || 'linux';

        const showBanner = !status.is_installed || status.needs_shell_setup;
        if (banner) banner.style.display = showBanner ? 'block' : 'none';
        if (showBanner) {
            if (!status.is_installed) {
                if (title) title.textContent = 'Install core features';
                if (body) {
                    body.textContent = 'Pyenv-native binaries were not found under the install root. Run setup to install the CLI, shims, and MCP companion.';
                }
                if (actionBtn) actionBtn.textContent = 'Install Now';
            } else {
                if (title) title.textContent = 'Activate shell integration';
                if (body) {
                    body.textContent = 'Core binaries are installed, but no shell profile has pyenv init yet. Finish setup so new terminals get shims on PATH.';
                }
                if (actionBtn) actionBtn.textContent = 'Finish Setup';
            }
        }

        if (currentWorkspaceDir && status.root) {
            const normalize = (value) => String(value).replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase();
            if (normalize(currentWorkspaceDir) === normalize(status.root)) {
                currentWorkspaceDir = '';
                localStorage.removeItem('pyenv-workspace-dir');
                updateWorkspaceUI();
            }
        }

        document.querySelector('.sidebar').style.display = 'flex';
        loadDashboard();

        if (status.is_portable && status.is_installed) {
            appendFooterPortableBadge();
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
        const result = await invoke('install_local_pyenv');
        if (statusText) statusText.textContent = "Done! Environment refreshed.";
        showAlert(
            'Setup Complete',
            `${escapeHtml(result)}<br><br>Open a <b>new terminal</b> so shims appear on PATH. This GUI process keeps its original PATH until restarted.`,
        );
        setTimeout(() => {
            location.reload();
        }, 1200);
    } catch (err) {
        if (progress) progress.style.display = 'none';
        if (actions) actions.style.display = 'flex';
        showAlert('Setup Failed', escapeHtml(err));
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
        setStepIconContent(icon, num);
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
        setStepIconContent(icon, createStepLoader());
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
        setStepIconContent(icon, createStepSvg([
            { tag: 'polyline', attrs: { points: '20 6 9 17 4 12' } },
        ]));
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
        setStepIconContent(icon, createStepSvg([
            { tag: 'line', attrs: { x1: '18', y1: '6', x2: '6', y2: '18' } },
            { tag: 'line', attrs: { x1: '6', y1: '6', x2: '18', y2: '18' } },
        ]));
        icon.style.background = 'rgba(239, 68, 68, 0.15)';
        icon.style.color = '#ef4444';
    }
    if (lbl) lbl.textContent = label;
    if (dsc) dsc.textContent = desc;
}

function applyUpgradeProgressLog(log, success) {
    const lines = String(log || '').split(/\r?\n/);
    lines.forEach((line) => {
        if (line.trim()) appendUpgraderLog(line);
    });

    const joined = lines.join('\n');
    const hasBackup = joined.includes('backup:');
    const hasCreate = joined.includes('venv:') || joined.includes('plan:');
    const hasRestore = joined.includes('restore:');
    const hasVerify = joined.includes('verify:');
    const hasCleanup = joined.includes('cleanup:') || joined.includes('Renamed managed venv');

    if (hasBackup) markStepSuccess('backup', 'Inventory complete', 'Captured source packages');
    if (hasCreate) markStepSuccess('recreate', 'Environment recreated', 'Created migration venv');
    if (hasRestore) markStepSuccess('restore', 'Packages restored', 'Reinstalled custom packages');
    if (hasVerify) {
        if (joined.includes('[WARNING] dependency conflicts')) {
            markStepWarning('verify', 'Conflicts discovered', 'pip check reported warnings');
        } else {
            markStepSuccess('verify', 'Verification complete', 'Dependency check finished');
        }
    }
    if (hasCleanup) markStepSuccess('cleanup', 'Cleanup complete', 'Old environment removed');

    if (!success) {
        if (!hasBackup) markStepWarning('backup', 'Inventory failed', 'Could not list packages in the source venv');
        else if (!hasCreate) markStepWarning('recreate', 'Create failed', 'Could not create the migration venv');
        else if (!hasRestore) markStepWarning('restore', 'Restore failed', 'Package reinstall stopped; source venv left unchanged');
        else if (!hasCleanup) markStepWarning('cleanup', 'Cleanup failed', 'New venv exists; source may still be present');
        appendUpgraderLog('\n[FATAL ERROR] Migration failed.');
    } else {
        appendUpgraderLog('\n[FINISH] Venv migration completed successfully!');
    }
}

// Function to open the wizard drawer
window.openVenvUpgradeWizard = async function(spec, baseVersion) {
    upgraderSourceSpec = spec;
    upgraderSourceName = venvDisplayName(spec);
    upgraderSourceBase = baseVersion;
    
    upgraderTargetTitle.textContent = `Migrate ${upgraderSourceName}`;
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
        fillSelectOptions(
            upgraderTargetVersionSel,
            bases.filter((b) => b !== baseVersion),
            { placeholder: 'Select Target...' },
        );
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
        appendUpgraderLog(`Initializing migration for venv:${upgraderSourceName} -> ${targetVer}...`);
        markStepInProgress('backup', 'Inventory old environment', 'Capturing installed packages...');
        markStepInProgress('recreate', 'Create fresh environment', 'Waiting for core migrate...');
        markStepInProgress('restore', 'Restore packages', 'Waiting for core migrate...');
        markStepInProgress('verify', 'Dependency diagnostics', 'Waiting for core migrate...');
        markStepInProgress('cleanup', 'Cleanup old environment', 'Waiting for core migrate...');

        try {
            const log = await invoke('upgrade_venv', {
                workspaceDir: getWorkspaceDir(),
                spec: upgraderSourceSpec,
                newRuntime: targetVer,
            });
            applyUpgradeProgressLog(String(log || ''), true);
            showAlert("Migration Success", `Managed virtual environment '${escapeHtml(upgraderSourceName)}' has been fully migrated to Python ${escapeHtml(targetVer)}.`);
            loadVenvs();
        } catch (error) {
            console.error("Migration failed:", error);
            applyUpgradeProgressLog(String(error || ''), false);
            showAlert("Migration Failed", escapeHtml(error));
        }
    });
}

// Open Package Explorer Drawer
window.openPackageExplorer = function(target) {
    currentExplorerTarget = target;
    
    // Set Target Title/Subtitle
    const envName = venvDisplayName(target);
    const isVenv = target.includes('/envs/') || target.startsWith('venv:');
    drawerTargetTitle.textContent = isVenv ? `Venv: ${envName}` : `Runtime: ${target}`;
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
    btnInstallImported.textContent = "Install Dependencies";
    
    // Clear import scanner results
    if (scanImportsResults) scanImportsResults.style.display = 'none';
    if (scanImportsLoading) scanImportsLoading.style.display = 'none';
    if (btnScanImports) {
        btnScanImports.disabled = false;
        btnScanImports.textContent = "Scan";
    }
    scannedMissingImports = [];
    if (scanMissingBadges) scanMissingBadges.replaceChildren();
    if (scanInstalledBadges) scanInstalledBadges.replaceChildren();
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
    const openDrawer = getOpenDrawerOverlay();
    if (openDrawer && event.key === 'Tab') {
        trapFocus(openDrawer, event);
    }
    if (event.key !== 'Escape') return;
    if (document.querySelector('.card-menu.is-open')) {
        closeCardMenus();
        return;
    }
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
    drawerPackageEmpty.style.display = 'none';
    pipSelfUpdateCard.style.display = 'none';
    drawerPackageList.replaceChildren();
    drawerPackageLoading.style.display = 'block';
    drawerPackageList.setAttribute('aria-busy', 'true');
    
    try {
        const jsonStr = await invoke('get_pip_packages', { workspaceDir: getWorkspaceDir(), target: currentExplorerTarget });
        installedPackages = JSON.parse(jsonStr);
        
        renderInstalledPackages();
        checkForPipUpdatesInBackground();
    } catch(err) {
        console.error("Failed to load packages:", err);
        drawerPackageList.replaceChildren();
        const tr = document.createElement('tr');
        const td = document.createElement('td');
        td.colSpan = 3;
        td.style.color = 'var(--danger)';
        td.style.textAlign = 'center';
        td.textContent = `Error loading packages: ${err}`;
        tr.appendChild(td);
        drawerPackageList.appendChild(tr);
    } finally {
        drawerPackageLoading.style.display = 'none';
        drawerPackageList.setAttribute('aria-busy', 'false');
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
        showAlert("Upgrade Failed", escapeHtml(err));
    } finally {
        btnUpgradePip.disabled = false;
        btnUpgradePip.textContent = "Update";
    }
});

// Render installed packages list with filtering
function renderInstalledPackages() {
    drawerPackageList.replaceChildren();
    const query = drawerPackageSearch.value.trim().toLowerCase();
    
    const filtered = installedPackages.filter(p => p.name.toLowerCase().includes(query));
    
    if (filtered.length === 0) {
        drawerPackageEmpty.style.display = 'block';
        return;
    }
    
    drawerPackageEmpty.style.display = 'none';
    
    filtered.forEach(p => {
        const tr = document.createElement('tr');
        const nameTd = document.createElement('td');
        nameTd.style.fontWeight = '500';
        nameTd.textContent = p.name;

        const versionTd = document.createElement('td');
        versionTd.style.fontFamily = "var(--font-mono)";
        versionTd.style.opacity = '0.8';
        versionTd.textContent = p.version;

        const statusTd = document.createElement('td');
        const statusSpan = document.createElement('span');
        if (p.name.toLowerCase() === 'pip') {
            statusSpan.style.color = 'var(--accent)';
            statusSpan.style.fontWeight = '500';
            statusSpan.textContent = 'System';
        } else {
            statusSpan.style.opacity = '0.6';
            statusSpan.textContent = 'OK';
        }
        statusTd.appendChild(statusSpan);
        tr.append(nameTd, versionTd, statusTd);
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
        showAlert("Scan Failed", escapeHtml(err));
        updatesScanPrompt.style.display = 'block';
    } finally {
        updatesScanning.style.display = 'none';
    }
});

// Render the checklist of outdated packages
function renderOutdatedChecklist() {
    updatesChecklist.replaceChildren();
    selectedOutdated.clear();
    
    const listToRender = outdatedPackages.filter(p => p.name.toLowerCase() !== 'pip');
    
    if (listToRender.length > 0) {
        updatesCountBadge.style.display = 'inline-block';
        updatesCountBadge.textContent = listToRender.length;
    } else {
        updatesCountBadge.style.display = 'none';
    }
    
    if (listToRender.length === 0) {
        const empty = document.createElement('div');
        empty.className = 'empty-state';
        empty.style.padding = '20px';
        empty.textContent = 'All libraries are up to date!';
        updatesChecklist.appendChild(empty);
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
        const pkgId = sanitizeDomId(p.name);
        const chk = document.createElement('input');
        chk.type = 'checkbox';
        chk.id = `chk-pkg-${pkgId}`;
        chk.dataset.pkgName = p.name;

        const label = document.createElement('label');
        label.htmlFor = chk.id;
        label.className = 'update-info';
        label.style.cursor = 'pointer';
        label.style.width = '100%';

        const nameSpan = document.createElement('span');
        nameSpan.className = 'update-name';
        nameSpan.textContent = p.name;

        const versionsSpan = document.createElement('span');
        versionsSpan.className = 'update-versions';

        const currentSpan = document.createElement('span');
        currentSpan.style.opacity = '0.6';
        currentSpan.textContent = p.version;

        const arrowSpan = document.createElement('span');
        arrowSpan.className = 'arrow-symbol';
        arrowSpan.textContent = '→';

        const latestSpan = document.createElement('span');
        latestSpan.style.color = '#ffd43b';
        latestSpan.style.fontWeight = '600';
        latestSpan.textContent = p.latest_version;

        versionsSpan.append(currentSpan, arrowSpan, latestSpan);
        label.append(nameSpan, versionsSpan);
        div.append(chk, label);

        chk.addEventListener('change', () => {
            if (chk.checked) selectedOutdated.add(p.name);
            else selectedOutdated.delete(p.name);
            updateSelectedCountText();
        });

        updatesChecklist.appendChild(div);
    });
    
    updatesScanPrompt.style.display = 'none';
    updatesChecklistView.style.display = 'block';
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
            selectedOutdated.delete(pkg);
        }
        
        progressText.textContent = `All updates completed!`;
        progressPercent.textContent = `100%`;
        progressBar.style.width = `100%`;
        progressBar.setAttribute('aria-valuenow', '100');
        
        setTimeout(() => {
            progressContainer.style.display = 'none';
            btnUpdateSelected.textContent = "Update Selected Packages";
        }, 1500);
        
        showAlert("Packages Updated", `Successfully upgraded packages:<br><code style="font-size: 11px;">${escapeHtml(pkgs.join(', '))}</code>`);
        
        // Refresh installed packages table
        loadDrawerPackages();
        
        // Reset outdated check since updates were performed
        resetOutdatedScanView();
    } catch(err) {
        showAlert("Update Failed", escapeHtml(err));
        progressContainer.style.display = 'none';
        btnUpdateSelected.disabled = false;
        btnUpdateSelected.textContent = "Update Selected Packages";
        loadDrawerPackages();
        updateSelectedCountText();
    }
});

// Precheck Requirements.txt URL or File Path
btnPrecheckImport.addEventListener('click', async () => {
    const pathOrUrl = importRequirementsPath.value.trim();
    if (!pathOrUrl) {
        showAlert("Input Required", "Please paste a requirements.txt local path or HTTPS URL.");
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
        showAlert("Precheck Failed", escapeHtml(err));
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
    precheckConflictsList.replaceChildren();
    if (precheck.potential_conflicts && precheck.potential_conflicts.length > 0) {
        precheckConflictsSection.style.display = 'block';
        precheck.potential_conflicts.forEach(c => {
            const div = document.createElement('div');
            div.className = 'import-issue-item';
            const strong = document.createElement('strong');
            strong.textContent = c.package;
            div.append(
                strong,
                document.createTextNode(`: installed version (${c.installed}) violates requirement "${c.requirement}"`),
            );
            precheckConflictsList.appendChild(div);
        });
    } else {
        precheckConflictsSection.style.display = 'none';
    }
    
    precheckResolvedList.replaceChildren();
    if (precheck.resolved_packages && precheck.resolved_packages.length > 0) {
        precheck.resolved_packages.forEach((p, index) => {
            const tr = document.createElement('tr');
            const nameTd = document.createElement('td');
            nameTd.style.fontWeight = '500';
            nameTd.textContent = p.name;

            const versionTd = document.createElement('td');
            versionTd.style.fontFamily = "var(--font-mono)";
            versionTd.textContent = p.version;

            const statusTd = document.createElement('td');
            const hasConflict = precheck.potential_conflicts?.some(c => c.package === p.name);
            if (hasConflict) {
                const conf = precheck.potential_conflicts.find(c => c.package === p.name);
                statusTd.appendChild(createWarningBadge(
                    'Conflict',
                    `precheck-conflict-tip-${sanitizeDomId(p.name)}-${index}`,
                    `Installed: ${conf.installed} vs Req: ${conf.requirement}`,
                ));
            } else if (p.version === 'not installed') {
                const pending = document.createElement('span');
                pending.style.color = '#60a5fa';
                pending.textContent = 'Pending Install';
                statusTd.appendChild(pending);
            } else {
                const compatible = document.createElement('span');
                compatible.style.color = 'var(--success)';
                compatible.style.fontWeight = '500';
                compatible.textContent = 'Compatible';
                statusTd.appendChild(compatible);
            }

            tr.append(nameTd, versionTd, statusTd);
            precheckResolvedList.appendChild(tr);
        });
    } else {
        const tr = document.createElement('tr');
        const td = document.createElement('td');
        td.colSpan = 3;
        td.style.textAlign = 'center';
        td.style.opacity = '0.6';
        td.textContent = 'No packages specified.';
        tr.appendChild(td);
        precheckResolvedList.appendChild(tr);
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
        showAlert("Installation Failed", escapeHtml(err));
    } finally {
        btnInstallImported.disabled = false;
        btnInstallImported.textContent = "Install Dependencies";
    }
});

// Codebase Import Analyzer Logic
let scanTargetDir = '';
const btnSelectScanFolder = document.getElementById('btn-select-scan-folder');
const scanFolderLabel = document.getElementById('scan-folder-label');

function updateScanFolderLabel() {
    if (!scanFolderLabel) return;
    if (scanTargetDir) {
        scanFolderLabel.textContent = `Scan folder: ${scanTargetDir}`;
    } else {
        scanFolderLabel.textContent = 'Choose a project folder, then scan. Only third-party (pip-installable) imports are reported — stdlib and private modules are skipped.';
    }
}

if (btnSelectScanFolder) {
    btnSelectScanFolder.addEventListener('click', async () => {
        try {
            const selected = await invoke('select_directory');
            if (selected) {
                scanTargetDir = selected;
                updateScanFolderLabel();
            }
        } catch (err) {
            showAlert('Folder Selection Failed', escapeHtml(err));
        }
    });
}

if (btnScanImports) {
    btnScanImports.addEventListener('click', async () => {
        const scanDir = scanTargetDir || currentWorkspaceDir || '';
        if (!scanDir) {
            showAlert("Folder Required", "Select a project folder with <b>Select Folder</b>, or set a workspace directory first.");
            return;
        }
        
        btnScanImports.disabled = true;
        btnScanImports.textContent = "Scanning...";
        if (scanImportsResults) scanImportsResults.style.display = 'none';
        if (scanImportsLoading) scanImportsLoading.style.display = 'block';
        if (scanMissingBadges) scanMissingBadges.replaceChildren();
        if (scanInstalledBadges) scanInstalledBadges.replaceChildren();
        
        try {
            const jsonStr = await invoke('analyze_codebase_imports', {
                workspaceDir: getWorkspaceDir(),
                target: currentExplorerTarget,
                dirPath: scanDir
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
            showAlert("Scan Failed", `Failed to statically scan workspace codebase imports:<br><code style="font-size: 11px;">${escapeHtml(err)}</code>`);
        } finally {
            if (scanImportsLoading) scanImportsLoading.style.display = 'none';
            btnScanImports.disabled = false;
            btnScanImports.textContent = "Scan";
        }
    });
}

document.getElementById('btn-refresh-cache')?.addEventListener('click', () => loadCacheStats());
document.getElementById('btn-purge-packages-cache')?.addEventListener('click', () => purgeCacheTarget('packages', 'download / package cache'));
document.getElementById('btn-purge-build-cache')?.addEventListener('click', () => purgeCacheTarget('python-build', 'python-build cache'));
document.getElementById('btn-purge-all-cache')?.addEventListener('click', () => purgeCacheTarget('all', 'all pyenv caches'));

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
                showAlert("Installation Failed", `Failed to install dependency <b>${escapeHtml(pkg)}</b>:<br><code style="font-size:11px;">${escapeHtml(err)}</code>`);
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
            text.textContent = `pip update available (current ${currentVer} → latest ${latestVer})`;
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
            checkActivePipStatus(status.managed_venv.spec || `venv:${status.managed_venv.name}`);
        } else {
            updateActivePipLight(false);
        }
    } catch(err) {
        console.warn("Dashboard background check failure:", err);
        updateActivePipLight(false);
    }
};

// ─── Shell Integration & Diagnostics ───
function createShellStatusBadge(label, style) {
    const badge = createBadge(label, 'badge', style);
    return badge;
}

async function loadPlatformIntelligence() {
    const summaryEl = document.getElementById('platform-intel-summary');
    const verdictEl = document.getElementById('platform-intel-verdict');
    const factsEl = document.getElementById('platform-intel-facts');
    const blockersEl = document.getElementById('platform-intel-blockers');
    if (!summaryEl || !verdictEl || !factsEl) return;

    try {
        const intel = await invoke('get_platform_intelligence', {
            workspaceDir: getWorkspaceDir(),
        });
        if (!intel || typeof intel !== 'object' || Array.isArray(intel)) {
            throw new Error('Platform intelligence payload was empty.');
        }
        summaryEl.textContent = intel.summary || 'Host environment details loaded.';
        const verdict = String(intel.verdict || '').toLowerCase();
        verdictEl.className = 'platform-intel-verdict';
        if (verdict === 'ready') {
            verdictEl.classList.add('ready');
            verdictEl.textContent = 'Ready';
        } else if (verdict === 'blocked') {
            verdictEl.classList.add('blocked');
            verdictEl.textContent = 'Blocked';
        } else if (verdict === 'needs-attention' || verdict === 'attention') {
            verdictEl.classList.add('attention');
            verdictEl.textContent = 'Needs attention';
        } else {
            verdictEl.textContent = 'Unavailable';
        }

        factsEl.replaceChildren();
        (intel.facts || []).slice(0, 8).forEach((fact) => {
            const item = document.createElement('div');
            item.className = 'platform-intel-fact';
            const label = document.createElement('span');
            label.className = 'label';
            label.textContent = fact.label || fact.key || 'Fact';
            const value = document.createElement('span');
            value.className = 'value';
            value.textContent = fact.value || '';
            item.append(label, value);
            factsEl.appendChild(item);
        });

        if (blockersEl) {
            const blockers = intel.blocking_issues || [];
            const warnings = intel.warnings || [];
            if (blockers.length || warnings.length) {
                blockersEl.style.display = 'block';
                blockersEl.replaceChildren();
                if (blockers.length) {
                    const title = document.createElement('div');
                    title.style.cssText = 'font-size: 11px; font-weight: 600; color: #f87171; margin-bottom: 6px;';
                    title.textContent = 'Blocking before install';
                    blockersEl.appendChild(title);
                    blockers.forEach((issue) => {
                        const row = document.createElement('div');
                        row.style.cssText = 'font-size: 11px; color: var(--text-muted); line-height: 1.4; margin-bottom: 4px;';
                        row.textContent = `• ${issue}`;
                        blockersEl.appendChild(row);
                    });
                }
                if (warnings.length) {
                    const warnTitle = document.createElement('div');
                    warnTitle.style.cssText = `font-size: 11px; font-weight: 600; color: #fbbf24; margin-bottom: 6px;${blockers.length ? ' margin-top: 10px;' : ''}`;
                    warnTitle.textContent = 'Install prerequisites needing attention';
                    blockersEl.appendChild(warnTitle);
                    warnings.forEach((issue) => {
                        const row = document.createElement('div');
                        row.style.cssText = 'font-size: 11px; color: var(--text-muted); line-height: 1.4; margin-bottom: 4px;';
                        row.textContent = `• ${issue}`;
                        blockersEl.appendChild(row);
                    });
                }
            } else {
                blockersEl.style.display = 'none';
                blockersEl.replaceChildren();
            }
        }
    } catch (err) {
        console.error('Failed to load platform intelligence:', err);
        summaryEl.textContent = 'Unable to load platform intelligence for this host.';
        verdictEl.className = 'platform-intel-verdict';
        verdictEl.textContent = 'Unavailable';
    }
}

async function loadShellIntegration() {
    await loadPlatformIntelligence();
    const cardsContainer = document.getElementById('shell-status-cards');
    if (!cardsContainer) return;

    setRegionLoading(cardsContainer, true, 'Scanning shells and configurations…');
    cardsContainer.setAttribute('aria-busy', 'true');

    try {
        const statuses = await invoke('get_shell_statuses', { workspaceDir: getWorkspaceDir() });
        cardsContainer.replaceChildren();

        if (!statuses || statuses.length === 0) {
            showEmptyState(cardsContainer, 'No standard shells discovered on this platform.');
            return;
        }

        statuses.forEach((status) => {
            const card = document.createElement('div');
            card.className = 'status-card glass fade-in';
            card.style.display = 'flex';
            card.style.flexDirection = 'column';
            card.style.justifyContent = 'space-between';
            card.style.minHeight = '180px';
            if (!status.is_installed) card.style.opacity = '0.55';

            const top = document.createElement('div');
            const title = document.createElement('h3');
            title.style.margin = '0 0 8px';
            title.style.color = '#fff';
            title.style.fontSize = '16px';
            title.textContent = status.name;

            const badges = document.createElement('div');
            badges.style.marginBottom = '12px';
            badges.style.display = 'flex';
            badges.style.flexWrap = 'wrap';
            badges.style.gap = '4px';

            if (!status.is_installed) {
                badges.appendChild(createShellStatusBadge(
                    'Not Detected',
                    'background: rgba(148, 163, 184, 0.08); color: #94a3b8; border: 1px solid rgba(148, 163, 184, 0.15);',
                ));
            } else if (status.is_configured) {
                badges.appendChild(createShellStatusBadge(
                    'Configured',
                    'background: rgba(16, 185, 129, 0.1); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.2);',
                ));
            } else {
                badges.appendChild(createShellStatusBadge(
                    'Not Configured',
                    'background: rgba(245, 158, 11, 0.1); color: #fbbf24; border: 1px solid rgba(245, 158, 11, 0.2);',
                ));
            }

            if (status.is_installed) {
                const pathLabel = status.path_label
                    || (status.active_in_path
                        ? 'PATH Active'
                        : (status.is_configured ? 'Restart terminal to activate' : 'Shims not on PATH'));
                const pathOk = status.active_in_path;
                const pathSoft = !pathOk && status.is_configured;
                badges.appendChild(createShellStatusBadge(
                    pathLabel,
                    pathOk
                        ? 'background: rgba(16, 185, 129, 0.1); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.2); margin-left: 8px;'
                        : pathSoft
                            ? 'background: rgba(59, 130, 246, 0.1); color: #93c5fd; border: 1px solid rgba(59, 130, 246, 0.2); margin-left: 8px;'
                            : 'background: rgba(245, 158, 11, 0.1); color: #fbbf24; border: 1px solid rgba(245, 158, 11, 0.2); margin-left: 8px;',
                ));
            }

            const profile = document.createElement('div');
            profile.style.fontSize = '11px';
            profile.style.color = 'var(--text-muted)';
            profile.style.wordBreak = 'break-all';
            profile.style.lineHeight = '1.4';
            const profileLabel = document.createElement('strong');
            profileLabel.textContent = 'Profile File:';
            profile.append(profileLabel, document.createElement('br'), document.createTextNode(status.profile_path));
            top.append(title, badges, profile);

            const actions = document.createElement('div');
            actions.style.marginTop = '16px';
            const btn = document.createElement('button');
            btn.type = 'button';
            btn.className = 'btn';
            btn.style.width = '100%';
            btn.style.padding = '8px';
            btn.style.fontSize = '11px';
            btn.style.fontWeight = '600';
            btn.id = `btn-cfg-${sanitizeDomId(status.name)}`;

            if (!status.is_installed) {
                const wantsPwsh = status.name.toLowerCase().includes('powershell 7');
                if (wantsPwsh) {
                    btn.classList.add('btn-gold');
                    btn.textContent = 'Install PowerShell 7';
                    btn.disabled = false;
                    btn.addEventListener('click', async () => {
                        btn.disabled = true;
                        btn.textContent = 'Installing...';
                        try {
                            const result = await invoke('install_powershell_7');
                            showAlert('PowerShell 7', escapeHtml(result));
                            loadShellIntegration();
                        } catch (err) {
                            showAlert('PowerShell 7 Install Failed', escapeHtml(err));
                            btn.disabled = false;
                            btn.textContent = 'Install PowerShell 7';
                        }
                    });
                } else {
                    btn.classList.add('btn-outline');
                    btn.textContent = 'Shell Not Installed';
                    btn.disabled = true;
                }
            } else if (status.is_configured) {
                btn.classList.add('btn-outline');
                btn.textContent = 'Configured';
                btn.disabled = true;
            } else {
                btn.classList.add('btn-gold');
                btn.textContent = 'Auto-Configure Shell';
                btn.addEventListener('click', async () => {
                    btn.disabled = true;
                    btn.textContent = 'Configuring...';
                    try {
                        await invoke('configure_shell', { shellName: status.name, profilePath: status.profile_path });
                        showAlert(
                            'Shell Configured',
                            `Successfully injected pyenv-native shell initialization block into profile:<br><code style="font-size: 11px;">${escapeHtml(status.profile_path)}</code><br><br>Restart your terminal shell for the changes to take effect!`,
                        );
                        loadShellIntegration();
                    } catch (err) {
                        showAlert('Configuration Failed', escapeHtml(err));
                        btn.disabled = false;
                        btn.textContent = 'Auto-Configure Shell';
                    }
                });
            }

            actions.appendChild(btn);
            card.append(top, actions);
            cardsContainer.appendChild(card);
        });

        runDoctorDiagnostics();
    } catch (err) {
        console.error('Failed to load shell statuses:', err);
        showEmptyState(cardsContainer, `Failed to scan shells: ${err}`, { danger: true });
    } finally {
        cardsContainer.setAttribute('aria-busy', 'false');
    }
}

// Refresh Shells action
const btnRefreshShells = document.getElementById('btn-refresh-shells');
if (btnRefreshShells) {
    btnRefreshShells.addEventListener('click', loadShellIntegration);
}
const btnRefreshPlatformIntel = document.getElementById('btn-refresh-platform-intel');
if (btnRefreshPlatformIntel) {
    btnRefreshPlatformIntel.addEventListener('click', loadPlatformIntelligence);
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
    if (doctorIssuesList) doctorIssuesList.replaceChildren();

    try {
        const jsonStr = await invoke('run_doctor', { workspaceDir: getWorkspaceDir() });
        // FFI command returns Vec<DoctorCheckGui> direct serialization
        const checks = typeof jsonStr === 'string' ? JSON.parse(jsonStr) : jsonStr;
        if (!Array.isArray(checks)) {
            throw new Error('Doctor did not return a check list.');
        }

        const parsed = checks.map(normalizeDoctorCheck).filter(Boolean);
        const issues = parsed
            .filter((check) => doctorIssueStatus(check.status))
            .map((check) => ({
                ...check,
                detail: check.detail || 'No additional detail was reported for this check.',
            }));

        if (checks.length > 0 && parsed.length === 0) {
            if (doctorIssuesCard) doctorIssuesCard.style.display = 'block';
            if (doctorIssuesList) {
                const item = document.createElement('div');
                item.className = 'doctor-issue';
                const strong = document.createElement('strong');
                strong.textContent = 'Incomplete diagnostics';
                item.append(strong, document.createTextNode(': the health report did not include names and reasons, so no warning is shown.'));
                doctorIssuesList.appendChild(item);
            }
            if (btnDoctorFix) {
                btnDoctorFix.disabled = true;
                btnDoctorFix.textContent = 'Repair unavailable';
            }
        } else if (issues.length === 0) {
            if (doctorHealthyCard) doctorHealthyCard.style.display = 'block';
        } else {
            if (doctorIssuesCard) doctorIssuesCard.style.display = 'block';
            if (doctorIssuesList) {
                issues.forEach((issue) => {
                    const item = document.createElement('div');
                    item.className = 'doctor-issue';
                    const strong = document.createElement('strong');
                    strong.textContent = issue.name;
                    item.append(strong, document.createTextNode(`: ${issue.detail}`));
                    doctorIssuesList.appendChild(item);
                });
            }
            if (btnDoctorFix) {
                btnDoctorFix.disabled = false;
                btnDoctorFix.textContent = 'Attempt Self-Healing Repair';
            }
        }
        
        if (doctorResults) doctorResults.style.display = 'block';
    } catch(err) {
        console.error("Doctor failed:", err);
        showAlert("Diagnostics Failed", `Doctor could not complete:<br><code style="font-size:11px;">${escapeHtml(err)}</code>`);
        if (doctorHealthyCard) doctorHealthyCard.style.display = 'none';
        if (doctorIssuesCard) doctorIssuesCard.style.display = 'none';
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
            message += `<li>${escapeHtml(act)}</li>`;
        });
        message += "</ul><br>Diagnostics will now be re-run.";
        
        showAlert("Self-Healing Complete", message);
        
        // Re-run diagnostics
        runDoctorDiagnostics();
    } catch(err) {
        showAlert("Self-Healing Failed", `Failed to automatically repair system environment:<br><code style="font-size:11px;">${escapeHtml(err)}</code>`);
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
