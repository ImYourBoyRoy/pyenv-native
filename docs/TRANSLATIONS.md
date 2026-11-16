# Translation status

Enchi ships 13 translations next to English (`locales/<tag>/*.ftl`, generated from
`scripts/i18n/overlays/<tag>.json`; run `python3 scripts/i18n/seed_locales.py` after editing an overlay).

**None of them has been reviewed by a native speaker.** They were machine-translated, and the
strings written or changed during the Enchi rename and the GUI/CLI work were then reviewed by a
second, independent model per language (placeholders, product and command names, terminology
consistency, and old product names were checked mechanically before the corrections were applied).
That catches wrong words, inconsistent terms and leftovers, but it is not a substitute for a
person who reads the language.

If you can read one of these languages, the ids below are the ones the reviewer was least sure
about. Each id is in the `locales/en-US/*.ftl` catalogs; fix the translation in
`scripts/i18n/overlays/<tag>.json`.

## Simplified Chinese (`zh-CN`)

Please check: `gui-intel-summary-ready`, `install-summary-blurb-detail`.

Terminology the reviewer standardised on: Conventions used consistently: runtime = 运行时, virtual environment = 虚拟环境 (managed = 托管), shim kept in English, pin = 固定, upgrade = 升级, package = 软件包, dashboard = 控制台, stub = 存根, GUI = 图形界面/图形控制台. Main inconsistencies found: bare 包 vs 软件包, 详情 vs 详细信息 for Details, and full-width vs ASCII colon after the "enchi" error prefix. No dry-run strings were in this batch.

## Spanish (`es`)

Please check: `doctor-store-alias-shadowed`, `gui-status-upgraded`, `gui-upgrade-repinned`.

Terminology the reviewer standardised on: Chosen standards: runtime = 'intérprete' (GUI convention, now applied to CLI), virtual environment = 'entorno virtual' (never bare 'venv' in prose), managed = 'gestionado', shim and plugin kept in English, 'portable' for portable installs, 'terminal' masculine as in VS Code, informal 'tú' as in GitHub/Docker. The original mixed usted (CLI/doctor) with tú (GUI), 'runtime' with 'intérprete', 'administrado' with 'gestionado', and masculine with feminine 'terminal'; all were unified. 'Pin' is rendered as 'fijar' / 'versión fijada'; 'Renombrar' is used throughout though VS Code says 'Cambiar nombre'.

## Japanese (`ja`)

Please check: `gui-activity-install-requirements`, `gui-pip-check-issues`, `gui-pip-check-ok`, `gui-status-outdated`, `gui-venv-rename-global-note`.

Terminology the reviewer standardised on: Chosen terms: ランタイム (runtime), 仮想環境 (virtual environment; 環境 alone for short activity lines), shim kept in English, 固定 (pin), アップグレード (runtime upgrade) versus 更新 (package update), パッチリリース. The translation used 端末 in some strings and ターミナル in others; I recommend ターミナル throughout, as VS Code and GitHub do. 導入 appeared once next to インストール, so I unified on インストール. Keeping the English labels Shims/Versions/Managed envs in the help text is acceptable.

## Korean (`ko`)

Please check: `gui-install-core-body`, `gui-no-venvs-hint`, `gui-pip-check-ok`, `gui-venv-rename-global-note`.

Terminology the reviewer standardised on: Conventions used: runtime=런타임, virtual environment=가상 환경 (CLI 'venv' kept in cli-upgrade-about), shim kept in Latin script, pin=고정, upgrade=업그레이드 vs update=업데이트, rollback=롤백, portable=포터블, doctor kept in English. Inconsistencies found: stale 'pyenv-native'/'pyenv' product text in four strings, one '심' transliteration for shim, and three different words for portable (휴대용/이동식/이동 가능한). Spacing between a backticked or braced placeholder and the following particle (e.g. '`{ $path }` 로', '{ $path } 는') is non-standard Korean typography but consistent and harmless; I left it.

## Brazilian Portuguese (`pt-BR`)

Please check: `gui-migrate-capturing`, `gui-status-up-to-date`, `gui-upgrade-repinned`, `gui-venv-rename-global-note`.

Terminology the reviewer standardised on: The file mixes 'runtime' (CLI/doctor items) and 'interpretador' (most GUI items) for the English 'runtime'; I standardized on 'runtime', which pt-BR developer tools commonly keep untranslated (the 'interpretador' in gui-windows-help-long is left alone because the English says 'interpreters'). Other terms kept: 'upgrade' (distinct from 'atualizar' = update), 'shim', 'venv'/'ambiente virtual', 'fixação' for pin, 'soma de verificação' for checksum, and 'stub'. Tooltip titles mixed infinitive and present tense; I standardized on the infinitive.

## French (`fr`)

Please check: `cli-agent-skills-about`, `gui-page-about`, `gui-upgrade-repinned`, `gui-venv-rename-global-note`.

Terminology the reviewer standardised on: I kept "runtime" (as the English does and as French dev docs commonly do) rather than "interpréteur", which the GUI strings had mixed in; venv stays "venv" or "environnement virtuel"; shim stays "shim"; pin is "épingler"; upgrade is "mettre à niveau", and update is "mettre à jour". Because Fluent cannot elide, "de" before the brand must be hand-written as d’{ $brand }, and hardcoded "Enchi" or "enchi" also needs d’; this is safe only while the brand begins with a vowel. Tooltips should consistently use the infinitive, and requirements in pip check should be "dépendances".

## German (`de`)

Please check: `cli-help-concepts`, `gui-status-upgraded`, `gui-upgrade-all`, `gui-upgrade-done`, `gui-upgrade-repinned`, `gui-upgrade-running`.

Terminology the reviewer standardised on: Standard dev-German choices: Runtime (kept English, as in cli-about/cli-upgrade-about; I replaced the inconsistent 'Laufzeit' elsewhere), virtuelle Umgebung / venv, Shim (kept), Upgrade (patch-level runtime move) vs Aktualisieren/Update (packages), Prüfsumme, Abhängigkeiten. Style is the impersonal infinitive imperative; I removed the stray formal 'Ihr/Sie' and informal 'aktualisiere'. 'Stamm' and 'Installationsstamm' for root were wrong (use Stammverzeichnis/Installationsverzeichnis), and two doctor strings wrongly said 'pyenv-CLI-Starter'. Plural forms with (n) are preserved to avoid '1 Umgebungen'.

## Russian (`ru`)

Please check: `cli-agent-skills-about`, `doctor-store-alias-shadowed`, `gui-upgrade-all`, `gui-venv-rename-global-note`.

Terminology the reviewer standardised on: Chosen terms: runtime = "среда выполнения", virtual environment = "виртуальное окружение" (short form "окружение", "venv" kept), shim = "шим", pin = "привязка", upgrade (runtime version) = "обновление" and package update = "обновление пакетов", root directory = "корневой каталог". Inconsistencies found and fixed: "shim/shim’ы/shims" mixed with "шим", "корень" vs "корневой каталог", "версий сред" vs "сред выполнения", "требования" vs "зависимости", and "backend" mistranslated as "сервер". The cli-help-concepts block had English labels left in and altered whitespace.

## Persian (`fa`)

Please check: `cli-agent-skills-about`, `gui-about-blurb-2`, `gui-announce-upgrade-panel`, `gui-migrate-capturing`, `gui-status-skipped`.

Terminology the reviewer standardised on: Standard choices here: ران‌تایم for runtime (GUI already uses it; the two زمان‌اجرا occurrences were harmonised), محیط مجازی for virtual environment (venv may stay Latin), پوسته for shell, پایانه for terminal, نمایه for profile, سنجاق for pin, ارتقا for upgrade vs به‌روزرسانی for update, چک‌سام for checksum, بک‌اند for backend (پشتیبان means 'backup' and was wrong), shim and PATH kept in Latin. The file mixed شل/پوسته, ترمینال/پایانه, فرمان/دستور, پرونده/فایل, Python/پایتون and منسوخ/قدیمی; the fixes standardise on the first-listed-in-notes forms (دستور, فایل, پایتون, قدیمی). Dry run and pin-as-noun do not occur in this batch beyond سنجاق.

## Arabic (`ar`)

Please check: `gui-about-blurb-2`, `gui-upgrade-note`, `gui-upgrade-repinned`, `gui-venv-rename-global-note`.

Terminology the reviewer standardised on: Chosen terms: runtime = وقت التشغيل (plural أوقات التشغيل), virtual environment = بيئة افتراضية, shell = الصدفة, terminal = الطرفية, shim and stub kept in Latin, pin = تحديد الإصدار (to avoid clashing with تثبيت = install), upgrade = ترقية vs update = تحديث, catalog = الفهرس, profile = ملف التعريف, and Python/Rust/Windows/Linux in Latin. Inconsistencies found and fixed: بايثون vs Python, شيمات/الشيمز vs shim, بيئات التشغيل vs أوقات التشغيل, الكتالوج vs الفهرس, ملف شخصي vs ملف تعريف, ويندوز/لينكس vs Windows/Linux, and diacritics on الصَدفة. "Dry run" does not appear in this batch.

## Hindi (`hi`)

Please check: `doctor-func-shim-missing`, `doctor-store-alias`, `gui-status-up-to-date`, `gui-venv-rename-global-note`.

Terminology the reviewer standardised on: Chosen standard: रनटाइम, वर्चुअल एनवायरनमेंट / एनवायरनमेंट (not आभासी वातावरण/वातावरण), इंस्टॉल (not स्थापित/स्थापना), अप-टू-डेट (not अद्यतन), shim/pin(पिन)/अपग्रेड/रोलबैक/pip kept transliterated or Latin, Python and Rust in Latin (not पाइथन/रस्ट), संस्करण for version, वैश्विक/स्थानीय for global/local. The machine translation mixed पाइथन/Python, स्थापित/इंस्टॉल and आभासी वातावरण/वातावरण/venv; I normalised these. Shim is treated as feminine (शिम ... खोलेगी/मेल नहीं खाती) in the existing strings; I aligned the one masculine use.

## Italian (`it`)

Please check: `cli-help-concepts`, `doctor-store-alias-shadowed`, `gui-status-updated`, `gui-venv-rename-global-note`.

Terminology the reviewer standardised on: Used here: shim (kept in English), venv / ambiente virtuale / ambiente gestito, pin (kept), checksum, backend, bundle. GUI strings use 'interprete' for runtime and the 'tu' imperative, while CLI, doctor and error strings use the infinitive. I changed the three outliers that said 'runtime' to 'interprete', and aligned the GUI strings that used the infinitive. 'Aggiornare' translates both upgrade and update; the upgrade strings disambiguate with 'di versione' or 'patch'. Plural '(s)' strings now avoid number agreement.

## Turkish (`tr`)

Please check: `gui-upgrade-all`, `gui-venv-rename-global-note`, `gui-which-found`.

Terminology the reviewer standardised on: Used: runtime = 'çalışma zamanı' (Microsoft convention), virtual environment = 'sanal ortam' (venv kept in CLI strings), shim/PATH/pip/doctor kept in English, pin = 'sabitle', upgrade = 'yükselt', update = 'güncelle', check = 'denetle', patch release = 'yama sürümü'. The main recurring defect was the wrong accusative on Enchi (vowel-final, so -yi/-nin) and suffixes attached to placeholders; 'outdated' was rendered as 'eski' and is now 'güncel olmayan'. Tooltips mix descriptive ('bulur') and polite imperative ('arayın') styles, while buttons use the bare imperative, which is acceptable but not uniform.
