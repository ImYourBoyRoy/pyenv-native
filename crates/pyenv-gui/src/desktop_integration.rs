// ./crates/pyenv-gui/src/desktop_integration.rs
//! Platform desktop integration helpers (dock/taskbar icons, Freedesktop launchers).

use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

const APP_ID: &str = "com.pyenv-native.gui";
const STARTUP_WM_CLASS: &str = "pyenv-gui";

pub fn prepare_app(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    apply_window_icons(app);

    #[cfg(target_os = "linux")]
    ensure_freedesktop_integration()?;

    Ok(())
}

fn apply_window_icons(app: &tauri::App) {
    let Some(icon) = app.default_window_icon().cloned() else {
        return;
    };

    for window in app.webview_windows().values() {
        let _ = window.set_icon(icon.clone());
    }
}

#[cfg(target_os = "linux")]
fn ensure_freedesktop_integration() -> Result<(), Box<dyn std::error::Error>> {
    let exe = std::env::current_exe()?;
    let data_home = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|home| PathBuf::from(home).join(".local/share"))
                .unwrap_or_else(|_| PathBuf::from(".local/share"))
        });

    let desktop_path = data_home
        .join("applications")
        .join(format!("{APP_ID}.desktop"));

    if desktop_path.exists() {
        return Ok(());
    }

    let share_root = locate_bundled_share_root(&exe);
    let Some(share_root) = share_root else {
        return Ok(());
    };

    install_icons_from_share(&share_root, &data_home)?;
    write_desktop_entry(&desktop_path, &exe)?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn ensure_freedesktop_integration() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg(target_os = "linux")]
fn locate_bundled_share_root(exe: &Path) -> Option<PathBuf> {
    let exe_dir = exe.parent()?;
    for candidate in [
        exe_dir.join("share"),
        exe_dir.join("../share"),
    ] {
        if candidate.join("icons/hicolor").is_dir() {
            return candidate.canonicalize().ok().or(Some(candidate));
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn install_icons_from_share(share_root: &Path, data_home: &Path) -> std::io::Result<()> {
    let source_icons = share_root.join("icons/hicolor");
    let target_icons = data_home.join("icons/hicolor");
    if !source_icons.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(&source_icons)? {
        let entry = entry?;
        let size_dir = entry.path();
        if !size_dir.is_dir() {
            continue;
        }
        let apps_dir = size_dir.join("apps");
        if !apps_dir.is_dir() {
            continue;
        }
        let source_icon = apps_dir.join(format!("{APP_ID}.png"));
        if !source_icon.is_file() {
            continue;
        }
        let dest_apps = target_icons
            .join(size_dir.file_name().unwrap())
            .join("apps");
        fs::create_dir_all(&dest_apps)?;
        fs::copy(&source_icon, dest_apps.join(format!("{APP_ID}.png")))?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn write_desktop_entry(desktop_path: &Path, exe: &Path) -> std::io::Result<()> {
    if let Some(parent) = desktop_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let exec_path = exe.to_string_lossy();
    let desktop_body = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Version=1.0\n\
         Name=Pyenv Native\n\
         GenericName=Python Environment Manager\n\
         Comment=Manage Python versions and virtual environments\n\
         Exec={exec_path}\n\
         Icon={APP_ID}\n\
         StartupWMClass={STARTUP_WM_CLASS}\n\
         Categories=Development;Utility;\n\
         Terminal=false\n"
    );
    fs::write(desktop_path, desktop_body)
}
