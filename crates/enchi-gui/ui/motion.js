// ./crates/enchi-gui/ui/motion.js
// Motion and feedback helpers for the Enchi GUI. Loaded before app.js.
//
// Public API (all on window):
//   Motion.toast(message, { type, action, duration, id }) -> { update(msg, opts), dismiss() }
//       type: 'success' | 'error' | 'info' | 'progress'. action: { label, onClick } or { label, view }.
//       duration: ms before auto-dismiss (0 = stay until dismissed). Errors linger, progress stays.
//   Motion.withBusy(button, promiseOrFn, { done }) -> the promise's result
//   Motion.setBusy(button, on), Motion.markDone(button)
//   Motion.createProgressBar() -> { el, set(pct | null), state('done' | 'failed' | '') }
//   Activity.start(id, label) -> { progress(pct, msg), done(msg), fail(msg, detail), dismiss() }
//
// CSP: the page allows no inline styles. This file only uses classes and
// element.style.setProperty / element.style.<prop>, which are CSSOM and allowed.

(function initMotion(global) {
    'use strict';

    const doc = global.document;
    const SVG_NS = 'http://www.w3.org/2000/svg';

    // ─── helpers ───
    function tr(id, fallback, args) {
        const i18n = global.I18n;
        if (i18n && typeof i18n.t === 'function') {
            const value = i18n.t(id, args);
            if (value && value !== id) return value;
        }
        return String(fallback).replace(/\{\s*\$([A-Za-z0-9_-]+)\s*\}/g, (_, name) => (
            args && args[name] !== undefined ? String(args[name]) : ''
        ));
    }

    function h(tag, className, text) {
        const el = doc.createElement(tag);
        if (className) el.className = className;
        if (text !== undefined) el.textContent = text;
        return el;
    }

    function svgIcon(shapes, className) {
        const svg = doc.createElementNS(SVG_NS, 'svg');
        svg.setAttribute('viewBox', '0 0 16 16');
        svg.setAttribute('width', '16');
        svg.setAttribute('height', '16');
        svg.setAttribute('fill', 'none');
        svg.setAttribute('stroke', 'currentColor');
        svg.setAttribute('stroke-width', '1.8');
        svg.setAttribute('stroke-linecap', 'round');
        svg.setAttribute('stroke-linejoin', 'round');
        svg.setAttribute('aria-hidden', 'true');
        svg.setAttribute('focusable', 'false');
        if (className) svg.setAttribute('class', className);
        shapes.forEach(([tag, attrs]) => {
            const node = doc.createElementNS(SVG_NS, tag);
            Object.entries(attrs).forEach(([k, v]) => node.setAttribute(k, v));
            svg.appendChild(node);
        });
        return svg;
    }

    const ICONS = {
        // pathLength=20 pairs with stroke-dasharray in motion.css so the check can draw itself.
        check: () => svgIcon([['path', { d: 'M3 8.6l3.2 3.2L13 4.8', pathLength: '20' }]], 'check-draw'),
        cross: () => svgIcon([['path', { d: 'M4.2 4.2l7.6 7.6M11.8 4.2l-7.6 7.6' }]]),
        info: () => svgIcon([
            ['circle', { cx: '8', cy: '8', r: '6.3' }],
            ['path', { d: 'M8 7.4v3.6M8 5v.1' }],
        ]),
        error: () => svgIcon([
            ['circle', { cx: '8', cy: '8', r: '6.3' }],
            ['path', { d: 'M8 4.8v3.8M8 11v.1' }],
        ]),
        chevron: () => svgIcon([['path', { d: 'M4 6.2l4 4 4-4' }]]),
    };

    function announce(message) {
        if (typeof global.announce === 'function') global.announce(message);
    }

    // ─── progress bar ───
    function createProgressBar() {
        const el = h('div', 'pbar is-indeterminate');
        el.setAttribute('role', 'progressbar');
        el.setAttribute('aria-valuemin', '0');
        el.setAttribute('aria-valuemax', '100');
        const fill = h('div', 'pbar-fill');
        el.appendChild(fill);
        return {
            el,
            set(pct) {
                const n = Number(pct);
                if (pct === null || pct === undefined || !Number.isFinite(n)) {
                    el.classList.add('is-indeterminate');
                    el.removeAttribute('aria-valuenow');
                    return;
                }
                const clamped = Math.max(0, Math.min(100, n));
                el.classList.remove('is-indeterminate');
                el.style.setProperty('--p', String(Math.round(clamped * 10) / 10));
                el.setAttribute('aria-valuenow', String(Math.round(clamped)));
            },
            state(value) {
                if (value) el.dataset.state = value; else delete el.dataset.state;
            },
        };
    }

    // ─── toasts ───
    const TOAST_MS = { success: 3200, info: 3800, error: 8000, progress: 0 };
    const MAX_VISIBLE = 3;
    const MAX_WAITING = 5;
    const liveToasts = [];
    const waitingToasts = [];

    function toastIcon(type) {
        if (type === 'success') return ICONS.check();
        if (type === 'error') return ICONS.error();
        if (type === 'progress') return h('span', 'toast-spin');
        return ICONS.info();
    }

    function toastHost() {
        return doc.getElementById('status-toast');
    }

    function armToast(entry, ms, withBar) {
        clearTimeout(entry.timer);
        entry.timer = null;
        entry.barEl?.remove();
        entry.barEl = null;
        if (!ms || ms <= 0 || entry.leaving) {
            entry.remaining = 0;
            return;
        }
        entry.remaining = ms;
        entry.started = Date.now();
        entry.timer = setTimeout(() => dismissToast(entry), ms);
        if (withBar && entry.el) {
            const bar = h('span', 'toast-timer');
            bar.setAttribute('aria-hidden', 'true');
            bar.style.setProperty('--toast-ms', `${ms}ms`);
            entry.el.appendChild(bar);
            entry.barEl = bar;
        }
    }

    function holdToast(entry, delta) {
        entry.holds = Math.max(0, entry.holds + delta);
        if (entry.holds === 1 && delta > 0 && entry.timer) {
            clearTimeout(entry.timer);
            entry.timer = null;
            entry.remaining = Math.max(400, entry.remaining - (Date.now() - entry.started));
        } else if (entry.holds === 0 && delta < 0 && entry.remaining > 0 && !entry.timer && !entry.leaving) {
            entry.started = Date.now();
            entry.timer = setTimeout(() => dismissToast(entry), entry.remaining);
        }
    }

    function renderToastIcon(entry) {
        entry.iconEl.replaceChildren(toastIcon(entry.type));
    }

    function mountToast(entry) {
        const host = toastHost();
        if (!host) return;
        const el = h('div', 'toast');
        el.dataset.type = entry.type;
        entry.el = el;

        entry.iconEl = h('span', 'toast-icon');
        entry.iconEl.setAttribute('aria-hidden', 'true');
        renderToastIcon(entry);
        entry.msgEl = h('div', 'toast-msg', entry.message);
        el.append(entry.iconEl, entry.msgEl);

        entry.actionEl = null;
        syncToastAction(entry);

        const close = h('button', 'toast-close');
        close.type = 'button';
        close.setAttribute('aria-label', tr('gui-toast-dismiss', 'Dismiss'));
        close.appendChild(ICONS.cross());
        close.addEventListener('click', () => dismissToast(entry));
        el.appendChild(close);

        el.addEventListener('mouseenter', () => holdToast(entry, 1));
        el.addEventListener('mouseleave', () => holdToast(entry, -1));
        el.addEventListener('focusin', () => holdToast(entry, 1));
        el.addEventListener('focusout', () => holdToast(entry, -1));
        el.addEventListener('keydown', (event) => {
            if (event.key === 'Escape') dismissToast(entry);
        });

        host.appendChild(el);
        liveToasts.push(entry);
        armToast(entry, entry.ms, true);
    }

    // Grid columns: icon, message, [action], close. The action is optional and swappable.
    function syncToastAction(entry) {
        const el = entry.el;
        if (!el) return;
        const wanted = entry.action;
        if (entry.actionEl) {
            entry.actionEl.remove();
            entry.actionEl = null;
        }
        if (!wanted || !wanted.label) return;
        const btn = h('button', 'toast-action', wanted.label);
        btn.type = 'button';
        btn.addEventListener('click', () => {
            try {
                if (typeof wanted.onClick === 'function') wanted.onClick();
                else if (wanted.view) doc.querySelector(`.nav-btn[data-view="${wanted.view}"]`)?.click();
            } finally {
                dismissToast(entry);
            }
        });
        entry.actionEl = btn;
        const close = el.querySelector('.toast-close');
        if (close) el.insertBefore(btn, close); else el.appendChild(btn);
    }

    function pumpToasts() {
        while (liveToasts.length < MAX_VISIBLE && waitingToasts.length) {
            mountToast(waitingToasts.shift());
        }
        if (waitingToasts.length) {
            // Something is queued behind a full stack: let the oldest self-closing toast go sooner.
            const oldest = liveToasts.find((e) => e.timer && e.holds === 0 && !e.leaving);
            if (oldest) {
                const left = oldest.remaining - (Date.now() - oldest.started);
                if (left > 1400) armToast(oldest, 1400, false);
            }
        }
    }

    function dismissToast(entry) {
        if (entry.dismissed) return;
        if (!entry.el) {
            const i = waitingToasts.indexOf(entry);
            if (i >= 0) waitingToasts.splice(i, 1);
            entry.dismissed = true;
            return;
        }
        entry.dismissed = true;
        entry.leaving = true;
        clearTimeout(entry.timer);
        const el = entry.el;
        // Hand focus back rather than dropping it to <body>.
        const hadFocus = el.contains(doc.activeElement);
        el.classList.add('is-leaving');
        let removed = false;
        const finish = () => {
            if (removed) return;
            removed = true;
            el.remove();
            const i = liveToasts.indexOf(entry);
            if (i >= 0) liveToasts.splice(i, 1);
            if (hadFocus && entry.returnFocus?.isConnected) entry.returnFocus.focus();
            pumpToasts();
        };
        el.addEventListener('animationend', finish, { once: true });
        setTimeout(finish, 260);
    }

    function toast(message, options) {
        const opts = typeof options === 'string' ? { type: options } : (options || {});
        const type = Object.prototype.hasOwnProperty.call(TOAST_MS, opts.type) ? opts.type : 'info';
        const text = String(message ?? '').trim();
        const noop = { dismiss() {}, update() {} };
        if (!text || !toastHost()) return noop;

        // Same message already on screen: keep one toast and restart its clock.
        const twin = liveToasts.find((e) => !e.leaving && e.type === type && e.message === text)
            || waitingToasts.find((e) => e.type === type && e.message === text);
        if (twin) {
            if (twin.el && twin.ms > 0) armToast(twin, twin.ms, true);
            return twin.handle;
        }

        const entry = {
            type,
            message: text,
            action: opts.action || null,
            ms: opts.duration !== undefined ? Number(opts.duration) : TOAST_MS[type],
            timer: null, started: 0, remaining: 0, holds: 0,
            el: null, msgEl: null, iconEl: null, actionEl: null, barEl: null,
            leaving: false, dismissed: false,
            returnFocus: doc.activeElement,
        };
        entry.handle = {
            dismiss: () => dismissToast(entry),
            update(next, extra) {
                if (entry.dismissed) return;
                const more = extra || {};
                if (next !== undefined && next !== null) {
                    entry.message = String(next);
                    if (entry.msgEl) entry.msgEl.textContent = entry.message;
                }
                if (more.type && Object.prototype.hasOwnProperty.call(TOAST_MS, more.type)) {
                    entry.type = more.type;
                    entry.ms = more.duration !== undefined ? Number(more.duration) : TOAST_MS[more.type];
                    if (entry.el) {
                        entry.el.dataset.type = entry.type;
                        renderToastIcon(entry);
                        armToast(entry, entry.ms, true);
                    }
                }
                if (more.action !== undefined) {
                    entry.action = more.action;
                    syncToastAction(entry);
                }
            },
        };

        if (liveToasts.length < MAX_VISIBLE) {
            mountToast(entry);
        } else {
            waitingToasts.push(entry);
            while (waitingToasts.length > MAX_WAITING) {
                const drop = waitingToasts.findIndex((e) => e.type !== 'error');
                waitingToasts.splice(drop >= 0 ? drop : 0, 1);
            }
            pumpToasts();
        }
        return entry.handle;
    }

    // ─── busy buttons ───
    // A busy button stays focusable (no `disabled`); a capture-phase guard eats its clicks.
    doc.addEventListener('click', (event) => {
        const target = event.target;
        const btn = target && target.closest ? target.closest('.btn.is-busy') : null;
        if (btn) {
            event.preventDefault();
            event.stopImmediatePropagation();
        }
    }, true);

    function clearDone(btn) {
        clearTimeout(btn._doneTimer);
        btn.classList.remove('is-done');
        btn.querySelector(':scope > .btn-check')?.remove();
    }

    function setBusy(btn, on) {
        if (!btn) return;
        if (on) {
            if (btn.classList.contains('is-busy')) return;
            clearDone(btn);
            const start = global.getComputedStyle(btn).paddingInlineStart;
            btn.style.setProperty('--busy-px', start);
            btn.style.minWidth = `${btn.offsetWidth}px`;
            btn.classList.add('is-busy');
            btn.setAttribute('aria-busy', 'true');
            btn.setAttribute('aria-disabled', 'true');
        } else {
            btn.classList.remove('is-busy');
            btn.removeAttribute('aria-busy');
            btn.removeAttribute('aria-disabled');
            btn.style.minWidth = '';
            if (!btn.classList.contains('is-done')) btn.style.removeProperty('--busy-px');
        }
    }

    function markDone(btn) {
        if (!btn) return;
        // Read the recorded start padding before any class change (a transition would skew a fresh read).
        const px = btn.style.getPropertyValue('--busy-px') || global.getComputedStyle(btn).paddingInlineStart;
        clearDone(btn);
        const check = ICONS.check();
        check.setAttribute('class', 'btn-check');
        btn.appendChild(check);
        btn.classList.add('is-done');   // keeps the padding while is-busy goes away
        setBusy(btn, false);
        btn.style.setProperty('--busy-px', px);
        btn._doneTimer = setTimeout(() => {
            clearDone(btn);
            btn.style.removeProperty('--busy-px');
        }, 1600);
    }

    async function withBusy(btn, work, options) {
        setBusy(btn, true);
        try {
            const result = await (typeof work === 'function' ? work() : work);
            if (options && options.done) markDone(btn); else setBusy(btn, false);
            return result;
        } catch (err) {
            setBusy(btn, false);
            throw err;
        }
    }

    // ─── activity tray ───
    const ACT_DONE_LINGER_MS = 8000;
    const activities = new Map();
    let tray = null;
    let trayParts = null;
    let tickTimer = null;

    function formatElapsed(ms) {
        const total = Math.max(0, Math.floor(ms / 1000));
        const s = total % 60;
        const m = Math.floor(total / 60) % 60;
        const hrs = Math.floor(total / 3600);
        const pad = (n) => String(n).padStart(2, '0');
        return hrs ? `${hrs}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
    }

    function ensureTray() {
        if (tray) return tray;
        const dock = doc.getElementById('status-dock') || doc.body;
        tray = h('section', 'activity');
        tray.setAttribute('aria-label', tr('gui-activity-aria', 'Background activity'));
        tray.hidden = true;

        const head = h('button', 'activity-head');
        head.type = 'button';
        head.setAttribute('aria-expanded', 'true');
        const title = h('span', 'activity-title', tr('gui-activity-title', 'Activity'));
        const count = h('span', 'activity-count');
        head.append(ICONS.chevron(), title, count);
        const list = h('ul', 'activity-list');
        head.addEventListener('click', () => {
            const collapsed = tray.classList.toggle('is-collapsed');
            head.setAttribute('aria-expanded', collapsed ? 'false' : 'true');
        });
        tray.append(head, list);
        dock.appendChild(tray);
        trayParts = { head, title, count, list };
        return tray;
    }

    function refreshTray() {
        if (!tray) return;
        const running = [...activities.values()].filter((a) => a.state === 'running').length;
        trayParts.title.textContent = tr('gui-activity-title', 'Activity');
        trayParts.count.textContent = running
            ? tr('gui-activity-running-count', '{ $count } running', { count: running })
            : tr('gui-activity-all-done', 'All done');
        tray.setAttribute('aria-label', tr('gui-activity-aria', 'Background activity'));
        activities.forEach((a) => {
            a.dismissBtn.setAttribute('aria-label', tr('gui-toast-dismiss', 'Dismiss'));
            a.moreBtn.textContent = a.moreBtn.getAttribute('aria-expanded') === 'true'
                ? tr('gui-activity-hide-details', 'Hide details')
                : tr('gui-activity-details', 'Details');
        });
    }

    function tick() {
        let anyRunning = false;
        const now = Date.now();
        activities.forEach((a) => {
            if (a.state === 'running') {
                anyRunning = true;
                a.timeEl.textContent = formatElapsed(now - a.started);
            }
        });
        if (!anyRunning && tickTimer) {
            clearInterval(tickTimer);
            tickTimer = null;
        }
    }

    function ensureTick() {
        if (!tickTimer) tickTimer = setInterval(tick, 1000);
    }

    function setActIcon(a) {
        a.iconEl.replaceChildren(
            a.state === 'done' ? ICONS.check()
                : a.state === 'failed' ? ICONS.cross()
                    : h('span', 'act-spin'),
        );
    }

    function hideTrayIfEmpty() {
        if (!tray || activities.size) return;
        tray.classList.add('is-leaving');
        let finished = false;
        const finish = () => {
            if (finished || activities.size) return;
            finished = true;
            tray.hidden = true;
            tray.classList.remove('is-leaving');
        };
        tray.addEventListener('animationend', finish, { once: true });
        setTimeout(finish, 260);
    }

    function removeActivity(a) {
        if (activities.get(a.id) !== a) return;
        activities.delete(a.id);
        clearTimeout(a.autoTimer);
        const hadFocus = a.el.contains(doc.activeElement);
        a.el.classList.add('is-leaving');
        let removed = false;
        const finish = () => {
            if (removed) return;
            removed = true;
            a.el.remove();
        };
        a.el.addEventListener('animationend', finish, { once: true });
        setTimeout(finish, 260);
        if (hadFocus) {
            if (activities.size) trayParts.head.focus();
            else a.returnFocus?.isConnected && a.returnFocus.focus();
        }
        refreshTray();
        hideTrayIfEmpty();
    }

    function scheduleAutoDismiss(a) {
        clearTimeout(a.autoTimer);
        a.autoTimer = setTimeout(() => {
            if (a.el.matches(':hover, :focus-within') || a.detailEl.hidden === false) {
                scheduleAutoDismiss(a);
                return;
            }
            removeActivity(a);
        }, ACT_DONE_LINGER_MS);
    }

    function buildActivity(id, label) {
        const a = {
            id, label, state: 'running', started: Date.now(), autoTimer: null,
            returnFocus: doc.activeElement,
        };
        const el = h('li', 'act');
        el.dataset.state = 'running';
        a.el = el;

        const row = h('div', 'act-row');
        a.iconEl = h('span', 'act-icon');
        a.iconEl.setAttribute('aria-hidden', 'true');
        const main = h('div', 'act-main');
        a.labelEl = h('div', 'act-label', label);
        a.labelEl.id = `act-label-${Math.random().toString(36).slice(2, 8)}`;
        a.msgEl = h('div', 'act-msg');
        main.append(a.labelEl, a.msgEl);
        a.timeEl = h('span', 'act-time', '0:00');
        a.dismissBtn = h('button', 'act-x');
        a.dismissBtn.type = 'button';
        a.dismissBtn.setAttribute('aria-label', tr('gui-toast-dismiss', 'Dismiss'));
        a.dismissBtn.appendChild(ICONS.cross());
        a.dismissBtn.addEventListener('click', () => removeActivity(a));
        row.append(a.iconEl, main, a.timeEl, a.dismissBtn);

        a.bar = createProgressBar();
        a.bar.el.setAttribute('aria-labelledby', a.labelEl.id);

        a.moreBtn = h('button', 'act-more', tr('gui-activity-details', 'Details'));
        a.moreBtn.type = 'button';
        a.moreBtn.hidden = true;
        a.moreBtn.setAttribute('aria-expanded', 'false');
        a.detailEl = h('pre', 'act-detail');
        a.detailEl.hidden = true;
        a.detailEl.tabIndex = 0;
        a.detailEl.setAttribute('role', 'group');
        a.detailEl.id = `act-detail-${Math.random().toString(36).slice(2, 8)}`;
        a.detailEl.setAttribute('aria-label', label);
        a.moreBtn.setAttribute('aria-controls', a.detailEl.id);
        a.moreBtn.addEventListener('click', () => {
            const open = a.detailEl.hidden;
            a.detailEl.hidden = !open;
            a.moreBtn.setAttribute('aria-expanded', open ? 'true' : 'false');
            a.moreBtn.textContent = open
                ? tr('gui-activity-hide-details', 'Hide details')
                : tr('gui-activity-details', 'Details');
        });

        el.append(row, a.bar.el, a.moreBtn, a.detailEl);
        setActIcon(a);
        return a;
    }

    function finishActivity(a, state, message, detail) {
        if (activities.get(a.id) !== a || a.state !== 'running') return;
        a.state = state;
        a.ended = Date.now();
        a.el.dataset.state = state;
        a.timeEl.textContent = formatElapsed(a.ended - a.started);
        a.msgEl.textContent = message;
        a.msgEl.title = message;
        a.bar.state(state);
        a.bar.set(100);
        setActIcon(a);
        if (detail) {
            a.detailEl.textContent = detail;
            a.moreBtn.hidden = false;
        }
        if (state === 'done') scheduleAutoDismiss(a);
        else tray.classList.remove('is-collapsed');
        trayParts.head.setAttribute('aria-expanded', tray.classList.contains('is-collapsed') ? 'false' : 'true');
        refreshTray();
        announce(`${a.label}: ${message}`);
    }

    function startActivity(id, label) {
        ensureTray();
        const key = String(id);
        const previous = activities.get(key);
        if (previous) {
            activities.delete(key);
            clearTimeout(previous.autoTimer);
            previous.el.remove();
        }
        const a = buildActivity(key, String(label ?? key));
        activities.set(key, a);
        trayParts.list.appendChild(a.el);
        tray.hidden = false;
        tray.classList.remove('is-leaving', 'is-collapsed');
        trayParts.head.setAttribute('aria-expanded', 'true');
        ensureTick();
        refreshTray();

        return {
            id: key,
            progress(pct, message) {
                if (activities.get(key) !== a || a.state !== 'running') return;
                a.bar.set(pct);
                if (message !== undefined && message !== null) {
                    a.msgEl.textContent = String(message);
                    a.msgEl.title = String(message);
                }
            },
            label(text) {
                if (activities.get(key) !== a) return;
                a.label = String(text);
                a.labelEl.textContent = a.label;
                a.detailEl.setAttribute('aria-label', a.label);
            },
            done(message, detail) {
                finishActivity(a, 'done', message ?? tr('gui-activity-status-done', 'Done'), detail);
            },
            fail(message, detail) {
                finishActivity(a, 'failed', message ?? tr('gui-activity-status-failed', 'Failed'), detail);
            },
            dismiss() { removeActivity(a); },
        };
    }

    const Activity = {
        start: startActivity,
        has: (id) => activities.has(String(id)),
    };

    // ─── list rows: stagger in when a list is (re)populated ───
    const ROW_SELECTOR = '.version-card, .cache-row';
    const ROW_CAP = 24;       // rows beyond the first screenful don't animate at all
    const INDEX_CAP = 12;     // ...and the delay stops growing here (12 * 20ms)
    const lastStagger = new WeakMap();

    function isTyping() {
        const el = doc.activeElement;
        return !!el && /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName);
    }

    function initRowStagger() {
        const root = doc.querySelector('.content') || doc.body;
        const observer = new MutationObserver((records) => {
            if (doc.hidden || isTyping()) return;
            const groups = new Map();
            const add = (row) => {
                const parent = row.parentElement;
                if (!parent) return;
                if (!groups.has(parent)) groups.set(parent, []);
                groups.get(parent).push(row);
            };
            records.forEach((record) => {
                record.addedNodes.forEach((node) => {
                    if (node.nodeType !== 1) return;
                    if (node.matches(ROW_SELECTOR)) add(node);
                    else node.querySelectorAll?.(ROW_SELECTOR).forEach(add);
                });
            });
            const now = global.performance.now();
            groups.forEach((rows, parent) => {
                // A list that re-renders again straight away (filtering) should not strobe.
                if (now - (lastStagger.get(parent) || -Infinity) < 900) return;
                lastStagger.set(parent, now);
                rows.slice(0, ROW_CAP).forEach((row, i) => {
                    row.style.setProperty('--i', String(Math.min(i, INDEX_CAP)));
                    row.classList.add('row-in');
                    row.addEventListener('animationend', () => {
                        row.classList.remove('row-in');
                        row.style.removeProperty('--i');
                    }, { once: true });
                });
            });
        });
        observer.observe(root, { childList: true, subtree: true });
    }

    // ─── nav marker: one bar that slides between items ───
    function initNavMarker() {
        const sidebar = doc.querySelector('.sidebar');
        if (!sidebar || sidebar.querySelector('.nav-marker')) return;
        const marker = h('span', 'nav-marker');
        marker.setAttribute('aria-hidden', 'true');
        sidebar.appendChild(marker);
        let placed = false;
        let frame = 0;

        function place() {
            frame = 0;
            const btn = sidebar.querySelector('.nav-btn.active, .nav-btn[aria-current="page"]');
            if (!btn || !btn.getClientRects().length) return;
            const sb = sidebar.getBoundingClientRect();
            const r = btn.getBoundingClientRect();
            const rtl = global.getComputedStyle(sidebar).direction === 'rtl';
            const x = rtl ? sb.right - r.right : r.left - sb.left;
            marker.style.setProperty('--marker-x', `${Math.round(x)}px`);
            marker.style.setProperty('--marker-y', `${Math.round(r.top - sb.top + sidebar.scrollTop + 7)}px`);
            marker.style.setProperty('--marker-h', `${Math.max(8, Math.round(r.height - 14))}px`);
            if (!placed) {
                placed = true;
                marker.classList.add('is-placed');
                sidebar.classList.add('has-marker');
                // Enable the slide only after the first position has been painted.
                global.requestAnimationFrame(() => global.requestAnimationFrame(() => {
                    marker.classList.add('is-animated');
                }));
            }
        }
        const schedule = () => {
            if (!frame) frame = global.requestAnimationFrame(place);
        };
        new MutationObserver(schedule).observe(sidebar, {
            subtree: true,
            attributes: true,
            attributeFilter: ['class', 'aria-current'],
        });
        global.addEventListener('resize', schedule);
        global.addEventListener('pyenv-i18n-changed', schedule);
        if (doc.fonts && doc.fonts.ready) doc.fonts.ready.then(schedule);
        schedule();
    }

    function init() {
        initRowStagger();
        initNavMarker();
        global.addEventListener('pyenv-i18n-changed', refreshTray);
    }

    global.Motion = { toast, withBusy, setBusy, markDone, createProgressBar, tr };
    global.Activity = Activity;
    global.withBusy = withBusy;

    if (doc.readyState === 'loading') doc.addEventListener('DOMContentLoaded', init);
    else init();
})(window);
