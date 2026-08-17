// ./crates/pyenv-gui/ui/i18n.js
// Fluent-backed GUI localization. Catalogs are resolved in Rust; this module
// applies data-i18n attributes and interpolates { $name } placeholders.

(function initGuiI18n(global) {
    const state = {
        lang: 'en-US',
        dir: 'ltr',
        nativeName: 'English',
        source: 'auto',
        messages: {},
        locales: [],
        ready: false,
    };

    function interpolate(template, args) {
        if (!args) return template;
        return String(template).replace(/\{\s*\$([A-Za-z0-9_-]+)\s*\}/g, (_, name) => (
            args[name] !== undefined && args[name] !== null ? String(args[name]) : `{ $${name} }`
        ));
    }

    function t(id, args) {
        const value = state.messages[id];
        if (value === undefined || value === null || value === '') {
            return interpolate(id, args);
        }
        return interpolate(value, args);
    }

    function applyDocumentLang() {
        document.documentElement.setAttribute('lang', state.lang);
        document.documentElement.setAttribute('dir', state.dir);
        document.documentElement.classList.toggle('is-rtl', state.dir === 'rtl');
    }

    function shortCode(tag) {
        if (!tag || tag === 'auto') return 'SYS';
        const parts = String(tag).split('-');
        return (parts[0] || 'EN').toUpperCase();
    }

    function updateSwitcher() {
        const code = document.getElementById('lang-switcher-code');
        const name = document.getElementById('lang-switcher-name');
        if (code) code.textContent = shortCode(state.lang);
        if (name) name.textContent = state.nativeName || state.lang;
        const select = document.getElementById('config-ui.language');
        if (select && select.options.length) {
            const configured = select.dataset.configured || 'auto';
            if ([...select.options].some((opt) => opt.value === configured)) {
                select.value = configured;
            }
        }
    }

    function localeMatches(query, locale) {
        const hay = `${locale.tag} ${locale.native_name || locale.nativeName || ''} ${locale.english_name || locale.englishName || ''}`.toLowerCase();
        return hay.includes(query);
    }

    function renderLangList(filter) {
        const list = document.getElementById('lang-list');
        if (!list) return;
        list.replaceChildren();
        const query = (filter || '').trim().toLowerCase();
        const items = [{ tag: 'auto', native_name: t('gui-language-auto'), english_name: t('gui-match-system-hint'), rtl: false }]
            .concat(state.locales || []);
        items
            .filter((locale) => !query || localeMatches(query, locale))
            .forEach((locale) => {
                const li = document.createElement('li');
                const btn = document.createElement('button');
                btn.type = 'button';
                btn.className = 'lang-option';
                btn.setAttribute('role', 'option');
                const selected = locale.tag === 'auto'
                    ? (state.source === 'auto')
                    : locale.tag === state.lang;
                btn.setAttribute('aria-selected', selected ? 'true' : 'false');
                const native = document.createElement('span');
                native.className = 'lang-option-native';
                native.textContent = locale.native_name || locale.nativeName || locale.tag;
                const english = document.createElement('span');
                english.className = 'lang-option-english';
                english.textContent = locale.tag === 'auto'
                    ? t('gui-match-system-hint')
                    : (locale.english_name || locale.englishName || locale.tag);
                btn.append(native, english);
                btn.addEventListener('click', () => {
                    setLanguage(locale.tag);
                    closePopover();
                });
                li.appendChild(btn);
                list.appendChild(li);
            });
    }

    function fillSettingsSelect() {
        const select = document.getElementById('config-ui.language');
        if (!select) return;
        const current = select.value || select.dataset.configured || 'auto';
        select.replaceChildren();
        const auto = document.createElement('option');
        auto.value = 'auto';
        auto.textContent = t('gui-language-auto');
        select.appendChild(auto);
        (state.locales || []).forEach((locale) => {
            const option = document.createElement('option');
            option.value = locale.tag;
            option.textContent = `${locale.native_name || locale.nativeName} — ${locale.english_name || locale.englishName}`;
            select.appendChild(option);
        });
        if ([...select.options].some((opt) => opt.value === current)) {
            select.value = current;
        }
    }

    function closePopover() {
        const pop = document.getElementById('lang-popover');
        const btn = document.getElementById('lang-switcher');
        if (pop) pop.hidden = true;
        if (btn) btn.setAttribute('aria-expanded', 'false');
    }

    function bindChrome() {
        const btn = document.getElementById('lang-switcher');
        const pop = document.getElementById('lang-popover');
        const search = document.getElementById('lang-search');
        const select = document.getElementById('config-ui.language');
        if (btn && pop && !btn.dataset.bound) {
            btn.dataset.bound = '1';
            btn.addEventListener('click', (event) => {
                event.stopPropagation();
                const open = pop.hidden;
                pop.hidden = !open;
                btn.setAttribute('aria-expanded', open ? 'true' : 'false');
                if (open) {
                    renderLangList(search?.value);
                    search?.focus();
                }
            });
            document.addEventListener('click', (event) => {
                if (!event.target.closest('.sidebar-foot')) closePopover();
            });
            document.addEventListener('keydown', (event) => {
                if (event.key === 'Escape') closePopover();
            });
        }
        if (search && !search.dataset.bound) {
            search.dataset.bound = '1';
            search.addEventListener('input', () => renderLangList(search.value));
        }
        if (select && !select.dataset.bound) {
            select.dataset.bound = '1';
            select.addEventListener('change', () => {
                select.dataset.configured = select.value;
                setLanguage(select.value);
            });
        }
        fillSettingsSelect();
        updateSwitcher();
        renderLangList(search?.value);
    }

    function applyStatic(root) {
        if (Object.keys(state.messages).length) {
            const scope = root || document;
            scope.querySelectorAll('[data-i18n]').forEach((el) => {
                const id = el.getAttribute('data-i18n');
                if (id) el.textContent = t(id);
            });
            scope.querySelectorAll('[data-i18n-placeholder]').forEach((el) => {
                const id = el.getAttribute('data-i18n-placeholder');
                if (id) el.setAttribute('placeholder', t(id));
            });
            scope.querySelectorAll('[data-i18n-aria]').forEach((el) => {
                const id = el.getAttribute('data-i18n-aria');
                if (id) el.setAttribute('aria-label', t(id));
            });
            scope.querySelectorAll('[data-i18n-title]').forEach((el) => {
                const id = el.getAttribute('data-i18n-title');
                if (id) el.setAttribute('title', t(id));
            });
            document.title = t('gui-app-title');
        }
        applyDocumentLang();
        updateSwitcher();
        fillSettingsSelect();
    }

    async function invokeI18n(cmd, args) {
        const fn = global.__TAURI__?.core?.invoke || global.__TAURI__?.invoke;
        if (typeof fn !== 'function') {
            return null;
        }
        return fn(cmd, args);
    }

    function adoptBundle(bundle, source) {
        if (!bundle) return;
        state.lang = bundle.lang || 'en-US';
        state.dir = bundle.dir || 'ltr';
        state.nativeName = bundle.native_name || bundle.nativeName || 'English';
        state.messages = bundle.messages || {};
        state.locales = bundle.locales || [];
        state.source = source || state.source;
        state.ready = true;
        applyStatic();
        global.dispatchEvent(new CustomEvent('pyenv-i18n-changed', { detail: { lang: state.lang } }));
    }

    async function load() {
        try {
            const bundle = await invokeI18n('i18n_bundle');
            adoptBundle(bundle, bundle?.source);
        } catch (_err) {
            /* Preview without Tauri keeps the English HTML. */
        }
        document.documentElement.classList.remove('i18n-pending');
        bindChrome();
        return state;
    }

    async function setLanguage(tag) {
        const bundle = await invokeI18n('set_ui_language', { tag });
        adoptBundle(bundle, tag === 'auto' ? 'auto' : 'explicit');
        return state;
    }

    async function format(id, args) {
        const result = await invokeI18n('i18n_format', { id, args: args || {} });
        if (typeof result === 'string' && result.length) return result;
        return t(id, args);
    }

    global.I18n = {
        t,
        format,
        load,
        setLanguage,
        applyStatic,
        bindChrome,
        get lang() { return state.lang; },
        get dir() { return state.dir; },
        get nativeName() { return state.nativeName; },
        get source() { return state.source; },
        get locales() { return state.locales; },
        get ready() { return state.ready; },
    };
})(window);
