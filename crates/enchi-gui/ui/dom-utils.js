// ./crates/enchi-gui/ui/dom-utils.js
// Shared DOM, accessibility, and safe-rendering helpers for the Enchi GUI.

(function initDomUtils(global) {
    let appShellLockCount = 0;

    function escapeHtml(value) {
        return String(value ?? '')
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;')
            .replace(/'/g, '&#39;');
    }

    function lockAppShell() {
        appShellLockCount += 1;
        document.querySelector('.app-container')?.setAttribute('inert', '');
        document.querySelector('.app-footer')?.setAttribute('inert', '');
    }

    function unlockAppShell() {
        appShellLockCount = Math.max(0, appShellLockCount - 1);
        if (appShellLockCount === 0) {
            document.querySelector('.app-container')?.removeAttribute('inert');
            document.querySelector('.app-footer')?.removeAttribute('inert');
        }
    }

    function getFocusableElements(container) {
        return Array.from(container.querySelectorAll(
            'a[href], button:not([disabled]), input:not([disabled]):not([type="hidden"]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
        )).filter((el) => !el.closest('[inert]') && el.getClientRects().length > 0);
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

    const RICH_TAGS = new Set(['BR', 'CODE', 'B', 'STRONG', 'UL', 'OL', 'LI', 'SPAN', 'P', 'EM']);

    function appendRichMessage(container, message) {
        container.replaceChildren();
        const text = String(message ?? '');
        if (!text.includes('<')) {
            container.textContent = text;
            return;
        }

        const template = document.createElement('template');
        const parsed = new DOMParser().parseFromString(text, 'text/html');
        parsed.body.childNodes.forEach((node) => {
            template.content.appendChild(node.cloneNode(true));
        });

        function copyNode(node, parent) {
            node.childNodes.forEach((child) => {
                if (child.nodeType === Node.TEXT_NODE) {
                    parent.appendChild(document.createTextNode(child.textContent));
                    return;
                }
                if (child.nodeType !== Node.ELEMENT_NODE) return;
                if (!RICH_TAGS.has(child.tagName)) {
                    copyNode(child, parent);
                    return;
                }
                const el = document.createElement(child.tagName.toLowerCase());
                if (child.className) el.className = child.className;
                parent.appendChild(el);
                copyNode(child, el);
            });
        }

        copyNode(template.content, container);
    }

    // Skeleton rows stand in for a list while it loads (shape of a .version-card).
    function createSkeleton(message, rows = 4) {
        const wrap = document.createElement('div');
        wrap.className = 'skeleton';
        wrap.setAttribute('role', 'status');
        const label = document.createElement('span');
        label.className = 'sr-only loading-label';
        label.textContent = message || 'Loading…';
        wrap.appendChild(label);
        for (let i = 0; i < rows; i += 1) {
            const row = document.createElement('div');
            row.className = 'skeleton-row';
            row.setAttribute('aria-hidden', 'true');
            const lines = document.createElement('div');
            lines.className = 'sk-lines';
            const title = document.createElement('span');
            title.className = 'sk sk-title';
            const meta = document.createElement('span');
            meta.className = 'sk sk-meta';
            lines.append(title, meta);
            const btn = document.createElement('span');
            btn.className = 'sk sk-btn';
            row.append(lines, btn);
            wrap.appendChild(row);
        }
        return wrap;
    }

    function setRegionLoading(container, isLoading, message) {
        if (!container) return;
        container.setAttribute('aria-busy', isLoading ? 'true' : 'false');
        if (isLoading) {
            if (container.querySelector('.version-card, .status-card, .cache-row, .shell-card, .skeleton')) {
                return;
            }
            container.replaceChildren(createSkeleton(message));
            return;
        }
    }

    // The ">>> |" motif used by empty states. Decorative, so hidden from assistive tech.
    function createPrompt() {
        const prompt = document.createElement('div');
        prompt.className = 'empty-prompt';
        prompt.setAttribute('aria-hidden', 'true');
        const marker = document.createElement('span');
        marker.textContent = '>>>';
        const caret = document.createElement('span');
        caret.className = 'empty-caret';
        prompt.append(marker, caret);
        return prompt;
    }

    function showEmptyState(container, message, options = {}) {
        if (!container) return;
        container.setAttribute('aria-busy', 'false');
        container.replaceChildren();
        const state = document.createElement('div');
        state.className = 'empty-state';
        if (options.danger) state.classList.add('is-danger');
        const text = document.createElement('p');
        text.className = 'empty-msg';
        text.textContent = message;
        state.append(createPrompt(), text);
        // Optional guidance (`hint`), the underlying error (`detail`), and one follow-up button
        // (`action` = { label, action, target, className }) such as Retry.
        if (options.hint) {
            const hint = document.createElement('p');
            hint.className = 'empty-sub';
            hint.textContent = options.hint;
            state.appendChild(hint);
        }
        if (options.detail) {
            const detail = document.createElement('pre');
            detail.className = 'empty-detail';
            detail.textContent = String(options.detail);
            state.appendChild(detail);
        }
        if (options.action) {
            const { label, action, target, className, ...extra } = options.action;
            const wrap = document.createElement('div');
            wrap.className = 'empty-actions';
            wrap.appendChild(createActionButton(label, action, target, className || 'btn btn-outline', extra));
            state.appendChild(wrap);
        }
        container.appendChild(state);
    }

    function createActionButton(label, action, target, className, extraDataset = {}) {
        const button = document.createElement('button');
        button.type = 'button';
        button.className = className;
        button.textContent = label;
        button.dataset.action = action;
        if (target !== undefined && target !== null) button.dataset.target = target;
        Object.entries(extraDataset).forEach(([key, value]) => {
            if (key === 'title') {
                if (value) button.title = String(value);
                return;
            }
            button.dataset[key] = value;
        });
        return button;
    }

    function createWarningBadge(text, tooltipId, tooltipText) {
        const badge = document.createElement('span');
        badge.className = 'warning-badge';
        badge.textContent = text;
        badge.tabIndex = 0;
        badge.setAttribute('role', 'button');
        badge.setAttribute('aria-describedby', tooltipId);
        const tooltip = document.createElement('span');
        tooltip.className = 'sr-only';
        tooltip.id = tooltipId;
        tooltip.textContent = tooltipText;
        badge.appendChild(tooltip);
        return badge;
    }

    function sanitizeDomId(value) {
        return String(value ?? '').replace(/[^a-zA-Z0-9_-]/g, '-');
    }

    function fillSelectOptions(select, items, options = {}) {
        if (!select) return;
        select.replaceChildren();
        if (options.placeholder !== undefined) {
            const placeholder = document.createElement('option');
            placeholder.value = '';
            placeholder.textContent = options.placeholder;
            select.appendChild(placeholder);
        }
        items.forEach((item) => {
            const option = document.createElement('option');
            if (typeof item === 'string') {
                option.value = item;
                option.textContent = item;
            } else {
                option.value = item.value;
                option.textContent = item.label ?? item.value;
            }
            select.appendChild(option);
        });
    }

    function createBadge(text, className) {
        const badge = document.createElement('span');
        badge.className = className || 'badge';
        badge.textContent = text;
        return badge;
    }

    global.DomUtils = {
        escapeHtml,
        lockAppShell,
        unlockAppShell,
        getFocusableElements,
        trapFocus,
        appendRichMessage,
        setRegionLoading,
        createSkeleton,
        createPrompt,
        showEmptyState,
        createActionButton,
        createWarningBadge,
        sanitizeDomId,
        fillSelectOptions,
        createBadge,
    };
})(window);
