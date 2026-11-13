// ./crates/pyenv-core/src/installer/uninstall.rs
//! `pyenv self-uninstall`: remove pyenv-native, its shell integration, and (optionally) its data.
//!
//! Safe by construction: the root must look like a pyenv-native install and may never be `/`, a
//! home directory, or one of its ancestors. On Windows a running executable cannot delete its own
//! directory, so removal is handed to a temporary copy of the binary that waits for this process
//! to exit.

use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Component, Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use crate::fs_util::{process_exists, remove_dir_all_with_retry};

use super::layout::InstallLayout;
use super::profile::{self, ShellKind};
use super::report::{LineSink, Reporter};
use super::user_path;

#[derive(Debug, Clone)]
pub struct SelfUninstallOptions {
    pub root: PathBuf,
    pub yes: bool,
    /// Keep installed Python versions, virtual environments, caches, and configuration; remove
    /// only the program files, shims, and shell integration.
    pub keep_data: bool,
    /// Leave shell profile blocks, user PATH entries, and desktop launchers in place.
    pub keep_shell_integration: bool,
    /// Remove the program files of a macOS `.pkg` install instead of the per-user root
    /// (`sudo pyenv self-uninstall --system`).
    pub system: bool,
    /// Internal: this process is the detached helper finishing removal after its parent exited.
    pub finish_after_pid: Option<u32>,
}

impl SelfUninstallOptions {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            yes: false,
            keep_data: false,
            keep_shell_integration: false,
            system: false,
            finish_after_pid: None,
        }
    }
}

/// Run the uninstall, streaming progress lines to `sink`.
pub fn run_self_uninstall(
    options: &SelfUninstallOptions,
    sink: LineSink<'_>,
) -> Result<(), String> {
    let mut reporter = Reporter::new(sink);
    if options.system {
        return super::system_pkg::remove_system_package(&mut reporter);
    }
    let layout = InstallLayout::new(options.root.clone());
    assert_safe_root(&layout)?;

    if let Some(pid) = options.finish_after_pid {
        wait_for_exit(pid, Duration::from_secs(120));
        remove_program_files_patiently(&mut reporter, &layout, options.keep_data)?;
        schedule_helper_cleanup();
        return Ok(());
    }

    confirm(options, &layout)?;

    if !options.keep_shell_integration {
        remove_shell_integration(&mut reporter, &layout);
    }
    match super::arp::unregister() {
        Ok(true) => reporter.info("Removed pyenv-native from Apps & features."),
        Ok(false) => {}
        Err(error) => reporter.warn(&format!(
            "Could not remove the Apps & features entry: {error}"
        )),
    }
    remove_program_files_or_defer(&mut reporter, &layout, options)?;
    reporter.plain(&format!(
        "Uninstalled pyenv-native from {}.",
        layout.root().display()
    ));
    if let Some(source) = std::env::current_exe()
        .ok()
        .and_then(|exe| super::source::package_source_for(&exe))
    {
        reporter.plain(&format!(
            "The pyenv program itself was installed by a {source} package; remove it with: {}",
            super::source::remove_command(&source)
        ));
    }
    Ok(())
}

/// Refuse roots that are not plausibly a pyenv-native install.
fn assert_safe_root(layout: &InstallLayout) -> Result<(), String> {
    let root = layout.root();
    let normal_components = root
        .components()
        .filter(|component| matches!(component, Component::Normal(_)))
        .count();
    let refuse = |why: &str| {
        Err(format!(
            "pyenv: refusing to uninstall `{}`: {why}",
            root.display()
        ))
    };
    if !root.is_absolute() {
        return refuse("the install root must be an absolute path");
    }
    if normal_components < 2 {
        return refuse("the path is too close to the filesystem root");
    }
    if let Some(home) = profile::home_dir()
        && (same_path(root, &home) || home.starts_with(root))
    {
        return refuse("it is, or contains, your home directory");
    }
    if !layout.looks_like_install() {
        return refuse("it does not look like a pyenv-native install (no bin, shims, or versions)");
    }
    Ok(())
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(a), Ok(b)) => a == b,
        _ => left == right,
    }
}

fn confirm(options: &SelfUninstallOptions, layout: &InstallLayout) -> Result<(), String> {
    if options.yes {
        return Ok(());
    }
    if !io::stdin().is_terminal() {
        return Err(
            "pyenv: self-uninstall requires confirmation; rerun with `pyenv self-uninstall --yes` for non-interactive use"
                .to_string(),
        );
    }
    if options.keep_data {
        println!(
            "This removes pyenv-native's programs, shims, and shell integration. Installed Python versions and virtual environments under {} are kept.",
            layout.root().display()
        );
        print!("Continue? [y/N] ");
    } else {
        println!(
            "WARNING: This will completely remove pyenv-native and ALL installed Python versions and virtual environments."
        );
        print!(
            "Are you sure you want to completely remove {} ? [y/N] ",
            layout.root().display()
        );
    }
    let _ = io::stdout().flush();
    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .map_err(|error| format!("pyenv: failed to read confirmation: {error}"))?;
    match answer.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => Ok(()),
        _ => Err("pyenv: self-uninstall cancelled".to_string()),
    }
}

fn wait_for_exit(pid: u32, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while process_exists(pid) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(250));
    }
}

/// Profile blocks, user PATH entries, `PYENV_ROOT`, and the Linux desktop launcher.
fn remove_shell_integration(reporter: &mut Reporter<'_>, layout: &InstallLayout) {
    if let Some(home) = profile::home_dir() {
        for (kind, path) in profile::known_profile_paths(&home) {
            if kind == ShellKind::None {
                continue;
            }
            match profile::remove_profile_block(&path) {
                Ok(true) => reporter.info(&format!(
                    "Removed pyenv-native block from {}",
                    path.display()
                )),
                Ok(false) => {}
                Err(error) => reporter.warn(&format!(
                    "Could not clean {}: {error}. Remove the pyenv-native init lines by hand.",
                    path.display()
                )),
            }
        }
    }

    if cfg!(windows) {
        match user_path::remove_user_path_entries_under(layout.root()) {
            Ok(true) => reporter.info("Removed pyenv-native entries from the user PATH."),
            Ok(false) => {}
            Err(error) => reporter.warn(&format!("Could not update the user PATH: {error}")),
        }
        if let Err(error) = user_path::clear_user_env_var_if("PYENV_ROOT", layout.root()) {
            reporter.warn(&format!(
                "Could not clear the user PYENV_ROOT variable: {error}"
            ));
        }
    }

    #[cfg(target_os = "linux")]
    if super::desktop::remove_desktop_entry() {
        reporter.info("Removed the GUI desktop launcher and icons.");
    }
}

/// Paths to delete: everything for a full uninstall, program files only when keeping data.
fn removal_targets(layout: &InstallLayout, keep_data: bool) -> Vec<PathBuf> {
    if keep_data {
        vec![layout.bin_dir(), layout.shims_dir(), layout.share_dir()]
    } else {
        vec![layout.root().to_path_buf()]
    }
}

fn remove_program_files(
    reporter: &mut Reporter<'_>,
    layout: &InstallLayout,
    keep_data: bool,
) -> Result<(), String> {
    for target in removal_targets(layout, keep_data) {
        if !target.exists() {
            continue;
        }
        remove_dir_all_with_retry(&target)
            .map_err(|error| format!("pyenv: failed to remove {}: {error}", target.display()))?;
        reporter.info(&format!("Removed {}", target.display()));
    }
    Ok(())
}

/// Used by the detached helper: the parent has just exited and antivirus or the indexer may
/// still hold files, so keep trying for up to a minute before giving up.
fn remove_program_files_patiently(
    reporter: &mut Reporter<'_>,
    layout: &InstallLayout,
    keep_data: bool,
) -> Result<(), String> {
    const ATTEMPTS: u32 = 30;
    let mut attempt = 1;
    loop {
        match remove_program_files(reporter, layout, keep_data) {
            Ok(()) => return Ok(()),
            Err(error) if attempt >= ATTEMPTS => return Err(error),
            Err(error) => {
                reporter.warn(&format!("{error} (attempt {attempt}/{ATTEMPTS}; retrying)"));
                thread::sleep(Duration::from_secs(2));
                attempt += 1;
            }
        }
    }
}

#[cfg(unix)]
fn remove_program_files_or_defer(
    reporter: &mut Reporter<'_>,
    layout: &InstallLayout,
    options: &SelfUninstallOptions,
) -> Result<(), String> {
    // Unlinking a running executable is fine on POSIX.
    remove_program_files(reporter, layout, options.keep_data)
}

#[cfg(not(unix))]
fn remove_program_files_or_defer(
    reporter: &mut Reporter<'_>,
    layout: &InstallLayout,
    options: &SelfUninstallOptions,
) -> Result<(), String> {
    use std::process::{Command, Stdio};

    use crate::process::PyenvCommandExt;

    let current = std::env::current_exe()
        .map_err(|error| format!("pyenv: cannot locate this executable: {error}"))?;
    let helper_dir =
        std::env::temp_dir().join(format!("pyenv-native-uninstall-{}", std::process::id()));
    fs::create_dir_all(&helper_dir)
        .map_err(|error| format!("pyenv: failed to create uninstall helper directory: {error}"))?;
    let helper = helper_dir.join(
        current
            .file_name()
            .map(|name| name.to_os_string())
            .unwrap_or_else(|| "pyenv.exe".into()),
    );
    fs::copy(&current, &helper)
        .map_err(|error| format!("pyenv: failed to stage the uninstall helper: {error}"))?;

    // The helper outlives this process, so its output goes to a log next to it. The directory is
    // deleted when the removal succeeds; if it fails, the log is left behind to explain why.
    let log_path = helper_dir.join("uninstall.log");
    let log = fs::File::create(&log_path)
        .map_err(|error| format!("pyenv: cannot create the uninstall log: {error}"))?;
    let log_err = log
        .try_clone()
        .map_err(|error| format!("pyenv: cannot open the uninstall log: {error}"))?;

    let mut command = Command::new(&helper);
    command
        .headless()
        .arg("self-uninstall")
        .arg("--yes")
        .arg("--finish-after-pid")
        .arg(std::process::id().to_string())
        .env("PYENV_ROOT", layout.root())
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err));
    if options.keep_data {
        command.arg("--keep-data");
    }
    command
        .spawn()
        .map_err(|error| format!("pyenv: failed to launch the uninstall helper: {error}"))?;
    reporter.info("Program files will be removed as soon as this process exits.");
    reporter.plain(&format!(
        "Started pyenv-native uninstallation for install root {}.",
        layout.root().display()
    ));
    Ok(())
}

/// Windows only: once removal is finished, delete the helper's own temporary copy.
fn schedule_helper_cleanup() {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};

        use crate::process::PyenvCommandExt;

        let Ok(current) = std::env::current_exe() else {
            return;
        };
        let Some(dir) = current.parent() else {
            return;
        };
        let is_helper_dir = dir
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("pyenv-native-uninstall-"));
        if !is_helper_dir {
            return;
        }
        let _ = Command::new("cmd")
            .headless()
            .raw_arg(format!(
                "/C ping -n 4 127.0.0.1 >nul & rmdir /s /q \"{}\"",
                dir.display()
            ))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fake_install(temp: &TempDir) -> InstallLayout {
        let layout = InstallLayout::new(temp.path().join("work").join(".pyenv"));
        for dir in layout.managed_dirs() {
            fs::create_dir_all(dir).unwrap();
        }
        fs::create_dir_all(layout.versions_dir().join("3.12.0")).unwrap();
        fs::write(layout.pyenv_exe(), "x").unwrap();
        layout
    }

    #[test]
    fn refuses_unsafe_roots() {
        let temp = TempDir::new().unwrap();
        let empty = temp.path().join("not-an-install");
        fs::create_dir_all(&empty).unwrap();
        for bad in [
            PathBuf::from("/"),
            PathBuf::from("relative/path"),
            empty.clone(),
        ] {
            assert!(
                assert_safe_root(&InstallLayout::new(bad.clone())).is_err(),
                "{bad:?} must be refused"
            );
        }
        if let Some(home) = profile::home_dir() {
            assert!(assert_safe_root(&InstallLayout::new(home)).is_err());
        }
    }

    #[test]
    fn accepts_a_real_install_root() {
        let temp = TempDir::new().unwrap();
        assert!(assert_safe_root(&fake_install(&temp)).is_ok());
    }

    #[test]
    fn keep_data_removes_only_program_files() {
        let temp = TempDir::new().unwrap();
        let layout = fake_install(&temp);
        let mut lines = Vec::new();
        let mut sink = |line: &str| lines.push(line.to_string());
        let mut reporter = Reporter::new(&mut sink);
        remove_program_files(&mut reporter, &layout, true).unwrap();
        assert!(!layout.bin_dir().exists());
        assert!(!layout.shims_dir().exists());
        assert!(layout.versions_dir().join("3.12.0").is_dir());
    }

    #[test]
    fn full_removal_deletes_the_root() {
        let temp = TempDir::new().unwrap();
        let layout = fake_install(&temp);
        let mut sink = |_: &str| {};
        let mut reporter = Reporter::new(&mut sink);
        remove_program_files(&mut reporter, &layout, false).unwrap();
        assert!(!layout.root().exists());
    }

    #[test]
    fn noninteractive_uninstall_requires_yes() {
        let temp = TempDir::new().unwrap();
        let layout = fake_install(&temp);
        let options = SelfUninstallOptions::new(layout.root());
        if !io::stdin().is_terminal() {
            assert!(confirm(&options, &layout).is_err());
        }
    }
}
