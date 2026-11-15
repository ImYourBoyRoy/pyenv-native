// ./crates/enchi-gui/ui/brand.js
// The one place the product's name lives in the UI. Change `name` (and `command` once the CLI is
// renamed) and every `{ $brand }` / `{ $command }` in the locale catalogs follows. The logo is
// brand/mark.svg; the accent colour is --accent in styles/tokens.css.
//
// Loaded first so the static fallback text in index.html is corrected before i18n applies.

(function initBrand(global) {
    'use strict';

    const Brand = Object.freeze({
        name: 'Enchi',
        command: 'enchi',
        // GitHub repository that publishes releases (keep in sync with brand::RELEASE_REPO in core).
        repo: 'imyourboyroy/pyenv-native',
        get args() {
            return { brand: this.name, command: this.command };
        },
    });

    function expand(text) {
        return String(text).replace(/\{\s*\$(brand|command)\s*\}/g, (_, key) => Brand.args[key]);
    }

    function applyStatic() {
        document.title = expand(document.title);
        document.querySelectorAll('[data-i18n], [data-i18n-title], [data-i18n-aria]').forEach((el) => {
            if (el.children.length === 0 && /\{\s*\$(brand|command)/.test(el.textContent)) {
                el.textContent = expand(el.textContent);
            }
            ['title', 'aria-label'].forEach((attr) => {
                const value = el.getAttribute(attr);
                if (value && /\{\s*\$(brand|command)/.test(value)) el.setAttribute(attr, expand(value));
            });
        });
    }

    global.Brand = Brand;
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', applyStatic, { once: true });
    } else {
        applyStatic();
    }
})(window);
