// ./crates/pyenv-gui/ui/maintenance.js
// Fleet maintenance: upgrade installed runtimes to the newest patch (venvs follow) and update
// outdated packages across every runtime and venv. Loaded after app.js; uses its globals
// (invoke, t, showToast, announce, loadInstalled, loadVenvs) plus window.Activity / Motion.
//
// Both actions are plan-first: the panel shows exactly what will change, then one button applies it.
// Progress arrives as `operation-progress` events from the Rust side and feeds the Activity tray.

(function initMaintenance() {
    'use strict';

    const doc = document;
    let counter = 0;

    function h(tag, className, text) {
        const el = doc.createElement(tag);
        if (className) el.className = className;
        if (text !== undefined) el.textContent = text;
        return el;
    }

    function nextId(prefix) {
        counter += 1;
        return `${prefix}-${Date.now().toString(36)}-${counter}`;
    }

    function parseReport(raw) {
        return typeof raw === 'string' ? JSON.parse(raw) : raw;
    }

    /** Run a backend command while mirroring its progress events into the Activity tray. */
    async function runTracked(command, args, label) {
        const operationId = nextId(command);
        const activity = window.Activity?.start(operationId, label);
        let unlisten = null;
        try {
            const listen = window.__TAURI__?.event?.listen;
            if (typeof listen === 'function') {
                unlisten = await listen('operation-progress', (event) => {
                    const payload = event.payload || {};
                    if (payload.id !== operationId || !activity) return;
                    const step = String(payload.step || '');
                    const counted = /^\[(\d+)\/(\d+)\]/.exec(step);
                    if (counted) {
                        activity.progress(((Number(counted[1]) - 1) / Number(counted[2])) * 100, step);
                    } else {
                        activity.progress(null, step);
                    }
                });
            }
            const raw = await invoke(command, { ...args, operationId });
            return { report: parseReport(raw), activity };
        } catch (error) {
            activity?.fail(String(error).split('\n')[0], String(error));
            throw error;
        } finally {
            if (typeof unlisten === 'function') unlisten();
        }
    }

    function statusBadge(status) {
        const map = {
            up_to_date: ['gui-status-up-to-date', 'Up to date', 'muted'],
            available: ['gui-status-available', 'Upgrade available', 'info'],
            upgraded: ['gui-status-upgraded', 'Upgraded', 'success'],
            partial: ['gui-status-partial', 'Partly done', 'warn'],
            failed: ['gui-status-failed', 'Failed', 'warn'],
            skipped: ['gui-status-skipped', 'Skipped', 'muted'],
            planned: ['gui-status-outdated', 'Outdated', 'info'],
            updated: ['gui-status-updated', 'Updated', 'success'],
            rolled_back: ['gui-status-rolled-back', 'Rolled back', 'warn'],
        };
        const [key, fallback, tone] = map[status] || ['gui-status-unknown', status, 'muted'];
        return h('span', `badge badge-${tone}`, t(key, null, fallback));
    }

    function row(name, status, detail, extra) {
        const el = h('div', 'maint-row');
        const head = h('div', 'maint-row-head');
        head.append(h('span', 'maint-row-name', name), statusBadge(status));
        el.appendChild(head);
        if (detail) el.appendChild(h('div', 'maint-row-detail', detail));
        if (extra) el.appendChild(extra);
        return el;
    }

    function packageList(packages) {
        if (!packages || packages.length === 0) return null;
        const list = h('ul', 'maint-packages');
        packages.forEach((pkg) => {
            list.appendChild(h('li', '', `${pkg.name}  ${pkg.version} → ${pkg.latest_version}`));
        });
        return list;
    }

    function panelShell(panel, title) {
        panel.replaceChildren();
        panel.hidden = false;
        panel.classList.remove('is-hidden');
        panel.appendChild(h('div', 'maint-title', title));
        return panel;
    }

    function actionBar(note, buttonLabel, onClick) {
        const bar = h('div', 'maint-actions');
        bar.appendChild(h('p', 'maint-note', note));
        const button = h('button', 'btn btn-primary', buttonLabel);
        button.type = 'button';
        button.addEventListener('click', onClick);
        bar.appendChild(button);
        return { bar, button };
    }

    // ── runtimes ──────────────────────────────────────────────
    function renderUpgradePlan(panel, items, onApply) {
        panelShell(panel, t('gui-upgrade-panel-title', null, 'Runtime upgrades'));
        const available = items.filter((item) => item.status === 'available');
        if (items.length === 0 || available.length === 0) {
            panel.appendChild(h('p', 'maint-note', t('gui-upgrade-none', null, 'Every runtime is already on the newest patch release.')));
        }
        const rows = h('div', 'maint-rows');
        items.forEach((item) => {
            const target = item.latest && item.latest !== item.installed ? ` → ${item.latest}` : '';
            rows.appendChild(row(`${item.installed}${target}`, item.status, item.status === 'available' ? '' : item.message));
        });
        panel.appendChild(rows);
        if (available.length > 0) {
            const { bar, button } = actionBar(
                t('gui-upgrade-note', null, 'Installs the newest patch, moves the managed venvs onto it, and updates your global and local pins. Old runtimes are kept.'),
                t('gui-upgrade-all', { count: available.length }, `Upgrade ${available.length}`),
                () => onApply(button),
            );
            panel.appendChild(bar);
        }
    }

    function renderUpgradeResult(panel, report) {
        panelShell(panel, t('gui-upgrade-panel-title', null, 'Runtime upgrades'));
        const rows = h('div', 'maint-rows');
        report.items.forEach((item) => {
            const detail = [item.message];
            item.venvs.forEach((venv) => detail.push(`${venv.ok ? '✓' : '✗'} ${venv.spec}: ${venv.message}`));
            item.repinned.forEach((file) => detail.push(`${t('gui-upgrade-repinned', null, 're-pinned')} ${file}`));
            rows.appendChild(row(`${item.installed}${item.latest ? ` → ${item.latest}` : ''}`, item.status, detail.join('\n')));
        });
        panel.appendChild(rows);
    }

    async function checkUpgrades(button) {
        const panel = doc.getElementById('upgrade-panel');
        if (!panel) return;
        panelShell(panel, t('gui-upgrade-panel-title', null, 'Runtime upgrades'));
        panel.appendChild(h('p', 'maint-note', t('gui-upgrade-checking', null, 'Checking for newer patch releases…')));
        const work = async () => {
            const items = parseReport(await invoke('plan_runtime_upgrades', { workspaceDir: window.getWorkspaceDir?.() || null }));
            renderUpgradePlan(panel, items, applyUpgrades);
            window.announce?.(t('gui-upgrade-announce', { count: items.filter((i) => i.status === 'available').length }, 'Upgrade check finished'));
        };
        try {
            await (window.withBusy ? window.withBusy(button, work) : work());
        } catch (error) {
            panelShell(panel, t('gui-upgrade-panel-title', null, 'Runtime upgrades'));
            panel.appendChild(h('p', 'maint-note maint-error', String(error)));
        }
    }

    async function applyUpgrades(button) {
        const panel = doc.getElementById('upgrade-panel');
        const label = t('gui-upgrade-running', null, 'Upgrading runtimes');
        const work = async () => {
            const { report, activity } = await runTracked(
                'run_runtime_upgrades',
                { workspaceDir: window.getWorkspaceDir?.() || null, targets: [], removeOld: false },
                label,
            );
            const ok = !report.items.some((item) => item.status === 'failed' || item.status === 'partial');
            if (ok) activity?.done(t('gui-upgrade-done', null, 'Runtimes upgraded'));
            else activity?.fail(t('gui-upgrade-runtimes-failed', null, 'Some upgrades did not finish'), JSON.stringify(report.items, null, 2));
            renderUpgradeResult(panel, report);
            window.loadInstalled?.();
            window.loadVenvs?.();
        };
        try {
            await (window.withBusy ? window.withBusy(button, work) : work());
        } catch (error) {
            panelShell(panel, t('gui-upgrade-panel-title', null, 'Runtime upgrades'));
            panel.appendChild(h('p', 'maint-note maint-error', String(error).split('\n')[0]));
        }
    }

    // ── packages ──────────────────────────────────────────────
    function renderBulkPlan(panel, report, onApply) {
        panelShell(panel, t('gui-bulk-panel-title', null, 'Package updates'));
        const outdated = report.targets.filter((target) => target.status === 'planned');
        const total = outdated.reduce((sum, target) => sum + target.outdated.length, 0);
        if (outdated.length === 0) {
            panel.appendChild(h('p', 'maint-note', t('gui-bulk-none', null, 'Every package in every environment is current.')));
            return;
        }
        panel.appendChild(h('p', 'maint-note', t('gui-bulk-count', { envs: outdated.length, packages: total }, `${outdated.length} environment(s) have ${total} outdated package(s).`)));
        const rows = h('div', 'maint-rows');
        outdated.forEach((target) => rows.appendChild(row(target.spec, target.status, '', packageList(target.outdated))));
        panel.appendChild(rows);
        const { bar, button } = actionBar(
            t('gui-bulk-note', null, 'Each environment is checked with pip check afterwards and rolled back if the update breaks something.'),
            t('gui-bulk-run', null, 'Update everything'),
            () => onApply(button),
        );
        panel.appendChild(bar);
    }

    function renderBulkResult(panel, report) {
        panelShell(panel, t('gui-bulk-panel-title', null, 'Package updates'));
        const rows = h('div', 'maint-rows');
        report.targets
            .filter((target) => target.status !== 'up_to_date')
            .forEach((target) => rows.appendChild(row(target.spec, target.status, target.message, packageList(target.outdated))));
        panel.appendChild(rows);
        const s = report.summary;
        panel.appendChild(h('p', 'maint-note', t('gui-bulk-summary', {
            packages: s.packages_updated, envs: s.updated, rolled: s.rolled_back, failed: s.failed,
        }, `${s.packages_updated} package(s) updated in ${s.updated} environment(s); ${s.rolled_back} rolled back, ${s.failed} failed.`)));
    }

    async function checkBulk(button) {
        const panel = doc.getElementById('bulk-panel');
        if (!panel) return;
        panelShell(panel, t('gui-bulk-panel-title', null, 'Package updates'));
        panel.appendChild(h('p', 'maint-note', t('gui-bulk-checking', null, 'Checking every environment…')));
        const work = async () => {
            const report = parseReport(await invoke('plan_bulk_package_updates', { workspaceDir: window.getWorkspaceDir?.() || null }));
            renderBulkPlan(panel, report, applyBulk);
        };
        try {
            await (window.withBusy ? window.withBusy(button, work) : work());
        } catch (error) {
            panelShell(panel, t('gui-bulk-panel-title', null, 'Package updates'));
            panel.appendChild(h('p', 'maint-note maint-error', String(error)));
        }
    }

    async function applyBulk(button) {
        const panel = doc.getElementById('bulk-panel');
        const label = t('gui-bulk-running', null, 'Updating packages');
        const work = async () => {
            const { report, activity } = await runTracked(
                'run_bulk_package_updates',
                { workspaceDir: window.getWorkspaceDir?.() || null, only: [] },
                label,
            );
            if (report.summary.failed === 0 && report.summary.rolled_back === 0) {
                activity?.done(t('gui-bulk-done', null, 'Packages updated'));
            } else {
                activity?.fail(t('gui-bulk-attention', null, 'Some environments need attention'), JSON.stringify(report.targets, null, 2));
            }
            renderBulkResult(panel, report);
        };
        try {
            await (window.withBusy ? window.withBusy(button, work) : work());
        } catch (error) {
            panelShell(panel, t('gui-bulk-panel-title', null, 'Package updates'));
            panel.appendChild(h('p', 'maint-note maint-error', String(error).split('\n')[0]));
        }
    }

    function bind(id, handler) {
        const button = doc.getElementById(id);
        if (button) button.addEventListener('click', () => handler(button));
    }

    bind('btn-check-upgrades', checkUpgrades);
    bind('btn-bulk-update', checkBulk);
})();
