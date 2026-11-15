// ./crates/enchi-gui/ui/tools.js
// Small desktop surfaces for CLI/MCP capabilities: copy-to-clipboard, the "where does this command
// come from?" lookup (`enchi which`), and per-venv details with an inline rename (`enchi venv
// rename`). Loaded after app.js and maintenance.js; uses their globals (invoke, t, showAlert,
// showToast, announce, tracked, loadVenvs, loadDashboard, getWorkspaceDir).

(function initTools(global) {
    'use strict';

    const doc = document;
    const NAME_PATTERN = /^[A-Za-z0-9._-]+$/;
    let seq = 0;

    function h(tag, className, text) {
        const el = doc.createElement(tag);
        if (className) el.className = className;
        if (text !== undefined) el.textContent = text;
        return el;
    }

    function button(label, className, onClick) {
        const el = h('button', className, label);
        el.type = 'button';
        if (onClick) el.addEventListener('click', onClick);
        return el;
    }

    function workspace() {
        return global.getWorkspaceDir ? global.getWorkspaceDir() : null;
    }

    // ── clipboard ─────────────────────────────────────────────
    async function copyToClipboard(text) {
        try {
            if (navigator.clipboard?.writeText) {
                await navigator.clipboard.writeText(text);
                return true;
            }
        } catch (_) { /* fall through to the selection fallback */ }
        const field = h('textarea', 'sr-only');
        field.value = text;
        field.setAttribute('readonly', '');
        field.setAttribute('aria-hidden', 'true');
        field.tabIndex = -1;
        const previous = doc.activeElement;
        doc.body.appendChild(field);
        field.select();
        let ok = false;
        try {
            ok = doc.execCommand('copy');
        } catch (_) {
            ok = false;
        }
        field.remove();
        if (previous && typeof previous.focus === 'function') previous.focus();
        return ok;
    }

    const resetTimers = new WeakMap();

    /** Copy `text`, flash "Copied" on `trigger`, and announce the outcome. */
    async function copy(text, trigger) {
        const ok = await copyToClipboard(String(text ?? ''));
        if (!ok) {
            showToast(t('gui-copy-failed', null, 'Could not copy to the clipboard.'));
            announce(t('gui-copy-failed', null, 'Could not copy to the clipboard.'));
            return false;
        }
        if (trigger) {
            if (!trigger.dataset.label) trigger.dataset.label = trigger.textContent;
            trigger.textContent = t('gui-copied', null, 'Copied');
            trigger.classList.add('is-done');
            clearTimeout(resetTimers.get(trigger));
            resetTimers.set(trigger, setTimeout(() => {
                trigger.textContent = trigger.dataset.label;
                trigger.classList.remove('is-done');
            }, 1500));
        }
        announce(t('gui-copied', null, 'Copied'));
        return true;
    }

    /** A "Copy" link-button for `value`; `ariaLabel` says what is being copied. */
    function createCopyButton(value, ariaLabel) {
        const btn = button(t('gui-copy', null, 'Copy'), 'link-btn', () => copy(value, btn));
        btn.title = t('gui-copy-title', null, 'Copy to the clipboard.');
        if (ariaLabel) btn.setAttribute('aria-label', ariaLabel);
        return btn;
    }

    /** `<path text> [Copy]` row used for install locations and interpreter paths. */
    function createPathRow(path, ariaLabel, className = 'version-location') {
        const row = h('div', className);
        const text = h('span', 'path-text', path);
        text.title = path;
        row.append(text, createCopyButton(path, ariaLabel));
        return row;
    }

    // ── which ─────────────────────────────────────────────────
    function renderWhichResult(host, result) {
        host.replaceChildren();
        host.hidden = false;
        host.dataset.kind = result.found ? 'found' : 'missing';
        if (result.found) {
            host.appendChild(h('div', 'result-title',
                t('gui-which-found', { command: result.command }, `${result.command} resolves to:`)));
            host.appendChild(createPathRow(
                result.path,
                t('gui-which-copy-aria', { command: result.command }, `Copy the path of ${result.command}`),
                'lookup-path-row',
            ));
        } else {
            host.appendChild(h('div', 'result-title',
                t('gui-which-missing', { command: result.command }, `${result.command} was not found.`)));
            if (result.message) host.appendChild(h('pre', 'result-message', result.message));
        }
        if (result.also_in?.length) {
            host.appendChild(h('div', 'result-meta',
                t('gui-which-also-in', { versions: result.also_in.join(', ') }, `Also installed in: ${result.also_in.join(', ')}`)));
        }
        announce(result.found
            ? t('gui-which-found', { command: result.command }, `${result.command} resolves to:`) + ` ${result.path}`
            : t('gui-which-missing', { command: result.command }, `${result.command} was not found.`));
    }

    function renderWhichError(host, message) {
        host.replaceChildren();
        host.hidden = false;
        host.dataset.kind = 'error';
        host.appendChild(h('div', 'result-title', t('gui-which-error', null, 'Could not look that up.')));
        host.appendChild(h('pre', 'result-message', String(message)));
    }

    function initWhich() {
        const form = doc.getElementById('which-form');
        const input = doc.getElementById('which-input');
        const submit = doc.getElementById('btn-which');
        const host = doc.getElementById('which-result');
        if (!form || !input || !submit || !host) return;
        let latest = 0;
        form.addEventListener('submit', async (event) => {
            event.preventDefault();
            const command = input.value.trim();
            if (!command) {
                renderWhichError(host, t('gui-which-empty', null, 'Enter a command name such as python or pip.'));
                input.focus();
                return;
            }
            const mine = ++latest;
            submit.disabled = true;
            submit.setAttribute('aria-busy', 'true');
            try {
                const result = await invoke('which_command', { workspaceDir: workspace(), command });
                if (mine === latest) renderWhichResult(host, result);
            } catch (err) {
                if (mine === latest) renderWhichError(host, err);
            } finally {
                submit.disabled = false;
                submit.removeAttribute('aria-busy');
            }
        });
        input.addEventListener('input', () => {
            if (!host.hidden && !input.value.trim()) host.hidden = true;
        });
    }

    // ── venv details + rename ─────────────────────────────────
    /** A core hint/warning as safe HTML: sentence case, and `backticked` terms as <code>. */
    function formatNote(note) {
        const text = String(note).trim();
        const sentence = text.charAt(0).toUpperCase() + text.slice(1);
        return escapeHtml(sentence).replace(/`([^`]+)`/g, '<code>$1</code>');
    }

    function detailRow(label, path, ariaLabel) {
        const row = h('div', 'detail-row');
        row.appendChild(h('div', 'detail-label', label));
        row.appendChild(createPathRow(path, ariaLabel, 'detail-value'));
        return row;
    }

    function validateName(raw, currentName) {
        const name = raw.trim();
        if (!name) return { ok: false, reason: 'empty' };
        if (name === '.' || name === '..' || !NAME_PATTERN.test(name)) return { ok: false, reason: 'invalid' };
        if (name === currentName) return { ok: false, reason: 'same' };
        return { ok: true, name };
    }

    /**
     * Details + rename panel for one managed venv card. Hidden until `openVenvPanel`.
     * `venv` is an entry of `get_managed_venvs`; `flags` = { isGlobal }.
     */
    function createVenvPanel(venv, flags = {}) {
        seq += 1;
        const spec = String(venv.spec || `${venv.base_version}/envs/${venv.name}`).replace(/\\/g, '/');
        const panel = h('div', 'version-panel');
        panel.hidden = true;
        panel.tabIndex = -1;
        panel.setAttribute('role', 'group');
        panel.setAttribute('aria-label', t('gui-venv-details-aria', { name: venv.name }, `Details for ${venv.name}`));

        const rows = h('div', 'detail-list');
        rows.appendChild(detailRow(t('gui-detail-location', null, 'Location'), venv.path,
            t('gui-copy-detail-aria', { what: t('gui-detail-location', null, 'Location') }, 'Copy Location')));
        if (venv.python_path) {
            rows.appendChild(detailRow(t('gui-detail-python', null, 'Python'), venv.python_path,
                t('gui-copy-detail-aria', { what: t('gui-detail-python', null, 'Python') }, 'Copy Python')));
        }
        if (venv.pip_path) {
            rows.appendChild(detailRow(t('gui-detail-pip', null, 'Pip'), venv.pip_path,
                t('gui-copy-detail-aria', { what: t('gui-detail-pip', null, 'Pip') }, 'Copy Pip')));
        }
        panel.appendChild(rows);

        // Rename form
        const form = h('form', 'rename-form');
        form.hidden = true;
        form.noValidate = true;
        form.autocomplete = 'off';
        const inputId = `venv-rename-${seq}`;
        const previewId = `venv-rename-preview-${seq}`;
        const label = h('label', 'rename-label', t('gui-venv-rename-label', null, 'New name'));
        label.htmlFor = inputId;
        const field = h('div', 'rename-field');
        const input = h('input', 'search-input');
        input.type = 'text';
        input.id = inputId;
        input.value = venv.name;
        input.spellcheck = false;
        input.autocapitalize = 'off';
        input.setAttribute('aria-describedby', previewId);
        const submit = button(t('gui-venv-rename-apply', null, 'Rename'), 'btn btn-primary btn-compact');
        submit.type = 'submit';
        submit.disabled = true;
        const cancel = button(t('gui-cancel', null, 'Cancel'), 'btn btn-outline btn-compact');
        field.append(input, submit, cancel);
        const preview = h('div', 'rename-preview');
        preview.id = previewId;
        preview.setAttribute('role', 'status');
        preview.setAttribute('aria-live', 'polite');
        const help = t('gui-venv-rename-help', null, 'Letters, numbers, . _ and - only.');
        preview.textContent = help;
        form.append(label, field, preview);
        panel.appendChild(form);

        let timer = null;
        let token = 0;
        const setPreview = (text, kind, title) => {
            preview.textContent = text;
            preview.dataset.kind = kind || '';
            preview.title = title || '';
        };
        const evaluate = () => {
            clearTimeout(timer);
            token += 1;
            const mine = token;
            submit.disabled = true;
            const verdict = validateName(input.value, venv.name);
            if (!verdict.ok) {
                if (verdict.reason === 'invalid') {
                    setPreview(t('gui-venv-rename-invalid', null, 'Use letters, numbers, . _ or - only.'), 'error');
                } else if (verdict.reason === 'same') {
                    setPreview(t('gui-venv-rename-same', null, 'Enter a different name.'), '');
                } else {
                    setPreview(help, '');
                }
                return;
            }
            setPreview(t('gui-venv-rename-checking', null, 'Checking…'), '');
            timer = setTimeout(async () => {
                try {
                    const plan = await invoke('plan_venv_rename', {
                        workspaceDir: workspace(), spec, newName: verdict.name,
                    });
                    if (mine !== token) return;
                    setPreview(
                        t('gui-venv-rename-preview', { old: venv.name, new: verdict.name }, `Will rename ${venv.name} to ${verdict.name}.`),
                        'ok',
                        plan,
                    );
                    submit.disabled = false;
                } catch (err) {
                    if (mine !== token) return;
                    setPreview(String(err).split('\n')[0], 'error', String(err));
                }
            }, 250);
        };
        input.addEventListener('input', evaluate);

        const close = () => {
            clearTimeout(timer);
            token += 1;
            form.hidden = true;
            input.value = venv.name;
            setPreview(help, '');
            submit.disabled = true;
            const card = panel.closest('.version-card');
            if (panel.dataset.mode === 'rename') {
                panel.hidden = true;
                card?.classList.remove('has-panel');
            }
            card?.querySelector('.card-menu-toggle')?.focus();
        };
        cancel.addEventListener('click', close);
        form.addEventListener('keydown', (event) => {
            if (event.key === 'Escape') {
                event.stopPropagation();
                close();
            }
        });

        form.addEventListener('submit', async (event) => {
            event.preventDefault();
            const verdict = validateName(input.value, venv.name);
            if (!verdict.ok || submit.disabled) return;
            const newName = verdict.name;
            submit.disabled = true;
            input.disabled = true;
            cancel.disabled = true;
            const idleLabel = submit.textContent;
            submit.textContent = t('gui-venv-rename-working', null, 'Renaming…');
            submit.setAttribute('aria-busy', 'true');
            try {
                const outcome = await tracked(
                    'rename-venv',
                    t('gui-activity-rename-venv', { old: venv.name, new: newName }, `Renaming ${venv.name} to ${newName}`),
                    () => invoke('rename_venv', { workspaceDir: workspace(), spec, newName }),
                );
                const lines = [`<p>${escapeHtml(t('gui-venv-rename-done', { old: venv.name, new: newName }, `${venv.name} is now ${newName}.`))}</p>`];
                const notes = [];
                if (flags.isGlobal) {
                    notes.push(t('gui-venv-rename-global-note', { new: newName }, `This environment is your global default. Choose Make Global on ${newName} to point the default at the new name.`));
                }
                (outcome.hints || []).forEach((hint) => notes.push(hint));
                (outcome.warnings || []).forEach((warning) => notes.push(warning));
                if (notes.length) {
                    lines.push(`<ul class="alert-list">${notes.map((note) => `<li>${formatNote(note)}</li>`).join('')}</ul>`);
                }
                showAlert(t('gui-venv-rename-done-title', null, 'Environment renamed'), lines.join(''));
                announce(t('gui-venv-rename-done', { old: venv.name, new: newName }, `${venv.name} is now ${newName}.`));
                global.loadVenvs?.();
                global.loadDashboard?.();
            } catch (err) {
                setPreview(String(err).split('\n')[0], 'error', String(err));
                showAlert(t('gui-venv-rename-failed', null, 'Rename Failed'), escapeHtml(String(err)));
                input.disabled = false;
                cancel.disabled = false;
                submit.textContent = idleLabel;
                submit.removeAttribute('aria-busy');
                submit.disabled = false;
                input.focus();
            }
        });

        panel.dataset.renameInput = inputId;
        return panel;
    }

    /** Show a card's panel; `mode` is 'details' or 'rename'. Details toggles; rename always opens. */
    function openVenvPanel(card, mode) {
        const panel = card?.querySelector('.version-panel');
        if (!panel) return;
        const form = panel.querySelector('.rename-form');
        if (mode === 'rename') {
            if (panel.hidden) panel.dataset.mode = 'rename';
            panel.hidden = false;
            form.hidden = false;
            card.classList.add('has-panel');
            const input = panel.querySelector('.rename-form input');
            input?.focus();
            input?.select();
            return;
        }
        const open = panel.hidden || !form.hidden;
        panel.hidden = !open;
        card.classList.toggle('has-panel', open);
        if (open) {
            panel.dataset.mode = 'details';
            form.hidden = true;
            panel.focus();
        } else {
            card.querySelector('.card-menu-toggle')?.focus();
        }
    }

    global.EnchiTools = {
        copy,
        copyFrom(trigger) {
            return copy(trigger.dataset.target || '', trigger);
        },
        createCopyButton,
        createPathRow,
        createVenvPanel,
        openVenvPanel,
        validateName,
    };

    initWhich();
})(window);
