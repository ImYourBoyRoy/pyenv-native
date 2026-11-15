// ./crates/enchi-gui/src/smoke.rs
//! UI diagnostics: a log sink for webview faults and an opt-in smoke tour.
//!
//! `PYENV_GUI_SMOKE=1 enchi-gui` opens every page in turn inside the real
//! webview, reports what rendered, then exits. Faults the page reports
//! (CSP violations, script errors) land on stderr as `[ui:<kind>] ...`.

use tauri::Manager;

#[tauri::command]
pub fn ui_report(kind: String, detail: String) {
    eprintln!("[ui:{kind}] {detail}");
}

/// Script the tour injects. Clicks each sidebar page, waits for it to settle,
/// and reports the visible page plus a coarse "did CSS apply" probe.
const TOUR: &str = r#"
(async () => {
  const report = (k, d) => window.__TAURI__.core.invoke('ui_report', { kind: k, detail: String(d) });
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const pages = [...document.querySelectorAll('.nav-btn[data-view]')];
  for (const btn of pages) {
    btn.click();
    await wait(1400);
    const view = document.getElementById(btn.dataset.view);
    const sidebar = getComputedStyle(document.querySelector('.sidebar'));
    const title = view?.querySelector('.page-title')?.textContent?.trim();
    const dark = matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
    report('smoke', `${btn.dataset.view} visible=${view && getComputedStyle(view).display !== 'none'} title="${title}" sidebarBg=${sidebar.backgroundColor} scheme=${dark}`);
  }
  // Read-only planning calls: exercises the Rust commands, IPC permissions and the panels.
  for (const [navId, buttonId, panelId] of [
    ['view-installed', 'btn-check-upgrades', 'upgrade-panel'],
    ['view-venvs', 'btn-bulk-update', 'bulk-panel'],
  ]) {
    document.querySelector(`.nav-btn[data-view="${navId}"]`)?.click();
    await wait(600);
    document.getElementById(buttonId)?.click();
    const panel = document.getElementById(panelId);
    let rows = 0;
    for (let i = 0; i < 60 && !rows; i++) {
      await wait(500);
      rows = panel ? panel.querySelectorAll('.maint-row').length : 0;
      if (panel?.querySelector('.maint-error')) break;
    }
    report('smoke', `${panelId} rows=${rows} error=${panel?.querySelector('.maint-error')?.textContent || 'none'} text="${(panel?.textContent || '').slice(0, 90).replace(/\s+/g, ' ')}"`);
  }
  report('smoke-done', `${pages.length} pages`);
  await wait(300);
  window.__TAURI__.core.invoke('close_app');
})();
"#;

pub fn start(app: &tauri::App) {
    if std::env::var_os("PYENV_GUI_SMOKE").is_none() {
        return;
    }
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(3000));
        eprintln!("[ui:smoke] starting tour");
        if let Err(err) = window.eval(TOUR) {
            eprintln!("[ui:smoke-fail] {err}");
        }
    });
}
