// ./crates/pyenv-core/src/installer/system_pkg.rs
//! Removing a macOS `.pkg` install: `sudo pyenv self-uninstall --system`.
//!
//! macOS has no package manager that uninstalls. The installer receipt, though, records every file
//! the package placed, so removal replays that list (`pkgutil --files`), keeps to the paths this
//! package is known to own, and finally forgets the receipt so a later `.pkg` installs cleanly.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use super::report::Reporter;
use super::source::package_source_for;

/// Receipt identifier given to the package by `cargo xtask installer pkg`.
pub(crate) const PKG_IDENTIFIER: &str = "com.pyenv-native.pkg";

/// Files owned individually, relative to `/` (the package ships nothing else in `bin`).
const OWNED_FILES: [&str; 2] = ["usr/local/bin/pyenv", "usr/local/bin/pyenv-mcp"];

/// Directories owned wholesale, relative to `/`.
const OWNED_TREES: [&str; 3] = [
    "usr/local/share/pyenv-native",
    "usr/local/share/doc/pyenv-native",
    "Applications/Pyenv Native.app",
];

/// Absolute paths to remove for a `pkgutil --files` listing, children before parents.
///
/// Anything outside the paths the package owns is ignored, so a corrupt or hostile receipt cannot
/// delete shared directories such as `/usr/local/bin`.
pub(crate) fn removal_plan(listing: &str) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = listing
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| is_owned(line))
        .map(|line| Path::new("/").join(line))
        .collect();
    paths.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    paths.dedup();
    paths
}

fn is_owned(relative: &str) -> bool {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return false;
    }
    OWNED_FILES.contains(&relative)
        || OWNED_TREES.iter().any(|tree| {
            relative == *tree
                || relative
                    .strip_prefix(tree)
                    .is_some_and(|rest| rest.starts_with('/'))
        })
}

fn is_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .is_some_and(|output| String::from_utf8_lossy(&output.stdout).trim() == "0")
}

/// Remove the files the `.pkg` installed. Per-user data under `PYENV_ROOT` is not touched.
pub(crate) fn remove_system_package(reporter: &mut Reporter<'_>) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err(
            "pyenv: --system removes the macOS .pkg install only; on other systems use your package manager"
                .to_string(),
        );
    }
    let exe = std::env::current_exe()
        .map_err(|error| format!("pyenv: cannot locate this executable: {error}"))?;
    if package_source_for(&exe).as_deref() != Some("pkg") {
        return Err(
            "pyenv: this pyenv was not installed by the macOS .pkg, so there are no system files to remove"
                .to_string(),
        );
    }
    if !is_root() {
        return Err(
            "pyenv: removing system files needs administrator rights; run `sudo pyenv self-uninstall --system`"
                .to_string(),
        );
    }

    let listing = Command::new("pkgutil")
        .args(["--files", PKG_IDENTIFIER])
        .output()
        .map_err(|error| format!("pyenv: cannot run pkgutil: {error}"))?;
    if !listing.status.success() {
        return Err(format!(
            "pyenv: pkgutil has no receipt for {PKG_IDENTIFIER}; nothing to remove"
        ));
    }

    let plan = removal_plan(&String::from_utf8_lossy(&listing.stdout));
    let mut removed = 0usize;
    for path in &plan {
        match remove_entry(path) {
            Ok(true) => removed += 1,
            Ok(false) => {}
            Err(error) => reporter.warn(&format!("Could not remove {}: {error}", path.display())),
        }
    }
    reporter.info(&format!("Removed {removed} installed files."));

    let forget = Command::new("pkgutil")
        .args(["--forget", PKG_IDENTIFIER])
        .output();
    match forget {
        Ok(output) if output.status.success() => {
            reporter.info("Forgot the installer receipt.");
        }
        _ => reporter.warn(&format!(
            "Could not forget the installer receipt; run `sudo pkgutil --forget {PKG_IDENTIFIER}`."
        )),
    }
    reporter.plain(
        "Removed the pyenv-native program files. Python versions under ~/.pyenv are kept; run `pyenv self-uninstall` first to remove them too.",
    );
    Ok(())
}

/// Delete one receipt entry. Directories are removed only when empty. `Ok(false)`: already gone or
/// not empty yet.
fn remove_entry(path: &Path) -> io::Result<bool> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if metadata.is_dir() {
        match fs::remove_dir(path) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::DirectoryNotEmpty => Ok(false),
            Err(error) => Err(error),
        }
    } else {
        fs::remove_file(path).map(|()| true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING: &str = "\
Applications
Applications/Pyenv Native.app
Applications/Pyenv Native.app/Contents
Applications/Pyenv Native.app/Contents/MacOS/pyenv-gui
usr
usr/local
usr/local/bin
usr/local/bin/pyenv
usr/local/bin/pyenv-mcp
usr/local/share
usr/local/share/pyenv-native
usr/local/share/pyenv-native/install-source
usr/local/share/doc/pyenv-native/README.md
";

    #[test]
    fn plan_keeps_to_owned_paths_and_never_touches_shared_directories() {
        let plan = removal_plan(LISTING);
        let shown: Vec<String> = plan.iter().map(|p| p.display().to_string()).collect();
        assert!(shown.contains(&"/usr/local/bin/pyenv".to_string()));
        assert!(shown.contains(&"/Applications/Pyenv Native.app".to_string()));
        for shared in [
            "/Applications",
            "/usr",
            "/usr/local",
            "/usr/local/bin",
            "/usr/local/share",
        ] {
            assert!(!shown.contains(&shared.to_string()), "{shared} is shared");
        }
    }

    #[test]
    fn children_come_before_their_parents() {
        let plan = removal_plan(LISTING);
        let position = |needle: &str| {
            plan.iter()
                .position(|path| path.display().to_string() == needle)
                .unwrap()
        };
        assert!(
            position("/usr/local/share/pyenv-native/install-source")
                < position("/usr/local/share/pyenv-native")
        );
        assert!(
            position("/Applications/Pyenv Native.app/Contents/MacOS/pyenv-gui")
                < position("/Applications/Pyenv Native.app")
        );
    }

    #[test]
    fn hostile_entries_are_ignored() {
        let plan = removal_plan(
            "../etc/passwd\n/etc/passwd\nusr/local/share/pyenv-native/../../bin/sh\nusr/local/share/pyenv-native-evil\nApplications/Pyenv Native.app.bak\n",
        );
        assert!(plan.is_empty(), "{plan:?}");
    }

    #[test]
    fn entries_that_are_gone_are_not_errors() {
        let temp = tempfile::TempDir::new().unwrap();
        assert!(!remove_entry(&temp.path().join("missing")).unwrap());
        let dir = temp.path().join("dir");
        fs::create_dir_all(dir.join("child")).unwrap();
        assert!(!remove_entry(&dir).unwrap(), "non-empty directories stay");
        assert!(remove_entry(&dir.join("child")).unwrap());
        assert!(remove_entry(&dir).unwrap());
    }
}
