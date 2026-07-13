// ./crates/pyenv-gui/ui/dom-utils.js
// Shared DOM, accessibility, and safe-rendering helpers for the pyenv-native GUI.

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
        template.innerHTML = text;

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
                if (child.tagName === 'CODE' && child.getAttribute('style')) {
                    el.setAttribute('style', child.getAttribute('style'));
                }
                parent.appendChild(el);
                copyNode(child, el);
            });
        }

        copyNode(template.content, container);
    }

    function setRegionLoading(container, isLoading, message) {
        if (!container) return;
        container.setAttribute('aria-busy', isLoading ? 'true' : 'false');
        if (isLoading) {
            container.replaceChildren();
            const state = document.createElement('div');
            state.className = 'empty-state';
            state.setAttribute('role', 'status');
            const loader = document.createElement('div');
            loader.className = 'loader';
            loader.setAttribute('aria-hidden', 'true');
            const label = document.createElement('div');
            label.className = 'loading-label';
            label.textContent = message || 'Loading…';
            state.append(loader, label);
            container.appendChild(state);
            return;
        }
    }

    function showEmptyState(container, message, options = {}) {
        if (!container) return;
        container.setAttribute('aria-busy', 'false');
        container.replaceChildren();
        const state = document.createElement('div');
        state.className = 'empty-state';
        if (options.danger) state.style.color = 'var(--danger)';
        state.textContent = message;
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

    function createBadge(text, className, style) {
        const badge = document.createElement('span');
        badge.className = className || 'badge';
        if (style) badge.setAttribute('style', style);
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
        showEmptyState,
        createActionButton,
        createWarningBadge,
        sanitizeDomId,
        fillSelectOptions,
        createBadge,
    };
})(window);
