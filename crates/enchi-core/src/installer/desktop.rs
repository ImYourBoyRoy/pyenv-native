// ./crates/enchi-core/src/installer/desktop.rs
//! Freedesktop launcher and icon integration for the GUI on desktop Linux.
//!
//! Installs `io.github.imyourboyroy.enchi.desktop` plus hicolor icons under `$XDG_DATA_HOME` so the GUI
//! shows up in application menus and docks. Other platforms do not use this module.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::layout::InstallLayout;

pub const APP_ID: &str = "io.github.imyourboyroy.enchi";
/// The id pyenv-native used; its launcher and icons are replaced on install and removed on uninstall.
const LEGACY_APP_ID: &str = "com.pyenv-native.gui";
const WM_CLASS: &str = "enchi-gui";
const ICON_SIZES: [&str; 4] = ["32x32", "128x128", "256x256", "512x512"];

fn data_home() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| super::profile::home_dir().map(|home| home.join(".local").join("share")))
}

/// Quote an executable path for the `Exec=` key of a desktop entry.
pub(crate) fn exec_quote(path: &str) -> String {
    if path.chars().any(|ch| " \t\"'\\><~|&;$*?#()`".contains(ch)) {
        let escaped = path
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('`', "\\`")
            .replace('$', "\\$");
        format!("\"{escaped}\"")
    } else {
        path.to_string()
    }
}

pub(crate) fn render_desktop_entry(gui_exe: &Path, icon: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nVersion=1.0\nName=Enchi\nGenericName=Python Environment Manager\nComment=Manage Python versions and virtual environments\nExec={}\nIcon={icon}\nStartupWMClass={WM_CLASS}\nCategories=Development;Utility;\nTerminal=false\n",
        exec_quote(&gui_exe.to_string_lossy())
    )
}

/// Install the launcher and icons. Returns human-readable result lines.
pub(crate) fn install_desktop_entry(layout: &InstallLayout) -> io::Result<Vec<String>> {
    let data_home = data_home().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "could not resolve XDG_DATA_HOME or HOME",
        )
    })?;
    let applications = data_home.join("applications");
    let icons_root = data_home.join("icons").join("hicolor");
    fs::create_dir_all(&applications)?;

    // Release bundles stage icons under <root>/share/icons/hicolor/<size>/apps/.
    let staged = layout.share_dir().join("icons").join("hicolor");
    let mut installed_icons = 0;
    for size in ICON_SIZES {
        let from = staged.join(size).join("apps").join(format!("{APP_ID}.png"));
        if !from.is_file() {
            continue;
        }
        let to_dir = icons_root.join(size).join("apps");
        fs::create_dir_all(&to_dir)?;
        fs::copy(&from, to_dir.join(format!("{APP_ID}.png")))?;
        installed_icons += 1;
    }

    let icon_file = icons_root
        .join("128x128")
        .join("apps")
        .join(format!("{APP_ID}.png"));
    let icon_value = if icon_file.is_file() {
        icon_file.to_string_lossy().into_owned()
    } else {
        APP_ID.to_string()
    };
    let desktop_path = applications.join(format!("{APP_ID}.desktop"));
    fs::write(
        &desktop_path,
        render_desktop_entry(&layout.gui_exe(), &icon_value),
    )?;

    remove_launcher_files(LEGACY_APP_ID, &applications, &icons_root);
    refresh_caches(&applications, &icons_root);

    Ok(vec![
        format!("Installed desktop launcher: {}", desktop_path.display()),
        format!(
            "Installed {installed_icons} icon size(s) under: {}",
            icons_root.display()
        ),
    ])
}

/// Remove the launcher and icons written by `install_desktop_entry`. Returns whether anything
/// was removed.
pub(crate) fn remove_desktop_entry() -> bool {
    let Some(data_home) = data_home() else {
        return false;
    };
    let applications = data_home.join("applications");
    let icons_root = data_home.join("icons").join("hicolor");
    let mut removed = false;
    for id in [APP_ID, LEGACY_APP_ID] {
        removed |= remove_launcher_files(id, &applications, &icons_root);
    }
    if removed {
        refresh_caches(&applications, &icons_root);
    }
    removed
}

/// Delete the `.desktop` file and icons for one app id. Returns whether anything was removed.
fn remove_launcher_files(id: &str, applications: &Path, icons_root: &Path) -> bool {
    let mut removed = fs::remove_file(applications.join(format!("{id}.desktop"))).is_ok();
    for size in ICON_SIZES {
        let icon = icons_root.join(size).join("apps").join(format!("{id}.png"));
        removed |= fs::remove_file(icon).is_ok();
    }
    removed
}

fn refresh_caches(applications: &Path, icons_root: &Path) {
    let quiet = |mut command: Command| {
        let _ = command.stdout(Stdio::null()).stderr(Stdio::null()).status();
    };
    let mut update_db = Command::new("update-desktop-database");
    update_db.arg(applications);
    quiet(update_db);
    let mut icon_cache = Command::new("gtk-update-icon-cache");
    icon_cache.args(["-f", "-t"]).arg(icons_root);
    quiet(icon_cache);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_paths_are_not_quoted() {
        assert_eq!(
            exec_quote("/home/u/.pyenv/bin/enchi-gui"),
            "/home/u/.pyenv/bin/enchi-gui"
        );
    }

    #[test]
    fn paths_with_spaces_or_specials_are_quoted_and_escaped() {
        assert_eq!(
            exec_quote("/home/my user/bin/gui"),
            "\"/home/my user/bin/gui\""
        );
        assert_eq!(exec_quote("/a/$b/gui"), "\"/a/\\$b/gui\"");
    }

    #[test]
    fn installing_replaces_the_pyenv_native_launcher() {
        let data = tempfile::TempDir::new().unwrap();
        let apps = data.path().join("applications");
        let icons = data.path().join("icons/hicolor");
        fs::create_dir_all(&apps).unwrap();
        fs::create_dir_all(icons.join("128x128/apps")).unwrap();
        let old_desktop = apps.join(format!("{LEGACY_APP_ID}.desktop"));
        let old_icon = icons.join(format!("128x128/apps/{LEGACY_APP_ID}.png"));
        fs::write(&old_desktop, "x").unwrap();
        fs::write(&old_icon, "x").unwrap();

        assert!(remove_launcher_files(LEGACY_APP_ID, &apps, &icons));
        assert!(!old_desktop.exists() && !old_icon.exists());
        assert!(
            !remove_launcher_files(LEGACY_APP_ID, &apps, &icons),
            "second pass finds nothing"
        );
    }

    #[test]
    fn desktop_entry_has_required_keys() {
        let entry = render_desktop_entry(Path::new("/r/bin/enchi-gui"), "/icons/x.png");
        for needle in [
            "[Desktop Entry]",
            "Type=Application",
            "Exec=/r/bin/enchi-gui",
            "Icon=/icons/x.png",
            "StartupWMClass=enchi-gui",
            "Terminal=false",
        ] {
            assert!(entry.contains(needle), "missing {needle}");
        }
    }
}
