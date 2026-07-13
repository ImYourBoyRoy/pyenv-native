// ./crates/pyenv-gui/src/desktop_integration.rs
//! Platform desktop integration helpers (dock/taskbar icons, Freedesktop launchers).

use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

const APP_ID: &str = "com.pyenv-native.gui";
const ICON_SIZES: &[(&str, &[u8])] = &[
    ("32x32", include_bytes!("../icons/32x32.png")),
    ("128x128", include_bytes!("../icons/128x128.png")),
    ("256x256", include_bytes!("../icons/128x128@2x.png")),
    ("512x512", include_bytes!("../icons/icon.png")),
];

pub fn prepare_app(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    apply_window_icons(app);

    #[cfg(target_os = "linux")]
    {
        configure_linux_wm_class();
        ensure_freedesktop_integration()?;
    }

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
fn configure_linux_wm_class() {
    if gtk::init().is_ok() {
        gtk::gdk::set_program_class(APP_ID);
    }
}

#[cfg(target_os = "linux")]
fn ensure_freedesktop_integration() -> Result<(), Box<dyn std::error::Error>> {
    let exe = std::env::current_exe()?;
    let data_home = xdg_data_home();

    install_embedded_icons(&data_home)?;
    if let Some(share_root) = locate_bundled_share_root(&exe) {
        let _ = install_icons_from_share(&share_root, &data_home);
    }

    let icon_path = data_home
        .join("icons/hicolor/128x128/apps")
        .join(format!("{APP_ID}.png"));
    let desktop_path = data_home
        .join("applications")
        .join(format!("{APP_ID}.desktop"));

    write_desktop_entry(&desktop_path, &exe, &icon_path)?;
    refresh_desktop_cache(&data_home);
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn ensure_freedesktop_integration() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg(target_os = "linux")]
fn xdg_data_home() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|home| PathBuf::from(home).join(".local/share"))
                .unwrap_or_else(|_| PathBuf::from(".local/share"))
        })
}

#[cfg(target_os = "linux")]
fn locate_bundled_share_root(exe: &Path) -> Option<PathBuf> {
    let exe_dir = exe.parent()?;
    let mut candidates = vec![
        exe_dir.join("share"),
        exe_dir.join("../share"),
    ];
    if let Some(parent) = exe_dir.parent() {
        candidates.push(parent.join("share"));
    }
    for candidate in candidates {
        if candidate.join("icons/hicolor").is_dir() {
            return candidate.canonicalize().ok().or(Some(candidate));
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn install_embedded_icons(data_home: &Path) -> std::io::Result<()> {
    for (size, bytes) in ICON_SIZES {
        let dest = data_home
            .join("icons/hicolor")
            .join(size)
            .join("apps")
            .join(format!("{APP_ID}.png"));
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        if !dest.exists() || fs::metadata(&dest)?.len() != bytes.len() as u64 {
            fs::write(&dest, bytes)?;
        }
    }
    Ok(())
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
        let source_icon = size_dir.join("apps").join(format!("{APP_ID}.png"));
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
fn write_desktop_entry(
    desktop_path: &Path,
    exe: &Path,
    icon_path: &Path,
) -> std::io::Result<()> {
    if let Some(parent) = desktop_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let exec_path = exe.to_string_lossy();
    let icon_value = if icon_path.is_file() {
        icon_path.to_string_lossy().to_string()
    } else {
        APP_ID.to_string()
    };

    let desktop_body = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Version=1.0\n\
         Name=Pyenv Native\n\
         GenericName=Python Environment Manager\n\
         Comment=Manage Python versions and virtual environments\n\
         Exec={exec_path}\n\
         Icon={icon_value}\n\
         StartupWMClass={APP_ID}\n\
         Categories=Development;Utility;\n\
         Terminal=false\n"
    );

    if desktop_path.exists() {
        let existing = fs::read_to_string(desktop_path)?;
        if existing == desktop_body {
            return Ok(());
        }
    }
    fs::write(desktop_path, desktop_body)
}

#[cfg(target_os = "linux")]
fn refresh_desktop_cache(data_home: &Path) {
    let apps_dir = data_home.join("applications");
    let icons_dir = data_home.join("icons/hicolor");
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&apps_dir)
        .status();
    let _ = std::process::Command::new("gtk-update-icon-cache")
        .args(["-f", "-t"])
        .arg(&icons_dir)
        .status();
}
