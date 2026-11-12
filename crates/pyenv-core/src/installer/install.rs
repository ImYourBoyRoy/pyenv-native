// ./crates/pyenv-core/src/installer/install.rs
//! `pyenv self-install`: install the binaries that sit next to the running executable (or at
//! explicit paths) into a portable root, wire up shells/PATH, refresh shims, and verify the result.
//!
//! This is the single install implementation. The network bootstrap scripts, the self-updater, the
//! Python bootstrap package, Homebrew, and the GUI all end up here.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::process::{PyenvCommandExt, command_on_path};

use super::binaries::{
    BinaryInstall, install_binary, sweep_replaced_binaries, windows_wrapper_files,
};
use super::layout::{GUI_BINARY, InstallLayout, MCP_BINARY, PYENV_BINARY, executable_file_name};
use super::profile::{self, ProfileUpdate, ShellKind};
use super::report::{LineSink, Reporter, utc_stamp};
use super::user_path;
use crate::fs_util::process_exists;

/// Whether to install PowerShell 7 with winget when it is missing (Windows only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerShell7Mode {
    /// Install without asking when `yes` is set, ask when interactive, otherwise skip.
    Auto,
    Always,
    Never,
}

impl PowerShell7Mode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" | "" => Some(Self::Auto),
            "true" | "yes" | "1" | "on" | "always" => Some(Self::Always),
            "false" | "no" | "0" | "off" | "never" => Some(Self::Never),
            _ => None,
        }
    }
}

/// Everything `pyenv self-install` can be told.
#[derive(Debug, Clone)]
pub struct SelfInstallOptions {
    /// Directory holding the binaries to install (default: the running executable's directory).
    pub source_dir: Option<PathBuf>,
    pub source_pyenv: Option<PathBuf>,
    pub source_mcp: Option<PathBuf>,
    pub source_gui: Option<PathBuf>,
    pub install_root: PathBuf,
    pub shell: ShellKind,
    /// Windows: add `bin` and `shims` to the persistent user PATH.
    pub add_to_user_path: bool,
    pub update_shell_profile: bool,
    pub refresh_shims: bool,
    pub force: bool,
    pub yes: bool,
    pub log_path: Option<PathBuf>,
    pub install_powershell7: PowerShell7Mode,
    /// Wait for this process to exit first (used when the updater hands over from a running app).
    pub wait_for_pid: Option<u32>,
    /// Start the installed GUI once the install succeeds.
    pub relaunch_gui: bool,
}

impl SelfInstallOptions {
    pub fn new(install_root: impl Into<PathBuf>) -> Self {
        Self {
            source_dir: None,
            source_pyenv: None,
            source_mcp: None,
            source_gui: None,
            install_root: install_root.into(),
            shell: ShellKind::detect(),
            add_to_user_path: true,
            update_shell_profile: true,
            refresh_shims: true,
            force: false,
            yes: false,
            log_path: None,
            install_powershell7: PowerShell7Mode::Auto,
            wait_for_pid: None,
            relaunch_gui: false,
        }
    }
}

/// Result of a successful install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfInstallOutcome {
    pub layout: InstallLayout,
    pub log_path: PathBuf,
    /// Profile files that were created or changed.
    pub profiles_changed: Vec<PathBuf>,
}

#[derive(Debug)]
struct Sources {
    pyenv: PathBuf,
    mcp: Option<PathBuf>,
    gui: Option<PathBuf>,
}

fn tr(key: &str, args: &[(&str, &str)]) -> String {
    let map: HashMap<String, String> = args
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect();
    crate::i18n::format_message(key, &map)
}

/// Run the install, streaming progress lines to `sink`.
pub fn run_self_install(
    options: &SelfInstallOptions,
    sink: LineSink<'_>,
) -> Result<SelfInstallOutcome, String> {
    let mut reporter = Reporter::new(sink);
    install_with_reporter(options, &mut reporter)
}

fn install_with_reporter(
    options: &SelfInstallOptions,
    reporter: &mut Reporter<'_>,
) -> Result<SelfInstallOutcome, String> {
    if let Some(pid) = options.wait_for_pid {
        wait_for_exit(pid, Duration::from_secs(120));
    }

    let sources = resolve_sources(options)?;
    let layout = InstallLayout::new(normalize_root(&options.install_root)?);
    let installed_exe = layout.pyenv_exe();

    assert_root_state(options, &layout)?;
    assert_root_access(&layout)?;
    warn_about_other_pyenv(reporter, &layout);

    let log_path = options.log_path.clone().unwrap_or_else(|| {
        layout
            .logs_dir()
            .join(format!("install-{}.log", utc_stamp()))
    });

    print_summary(reporter, options, &sources, &layout, &log_path);
    confirm(options)?;

    // Nothing is written to disk until the install is confirmed.
    reporter.attach_log(&log_path).map_err(|error| {
        format!(
            "pyenv: cannot write install log {}: {error}",
            log_path.display()
        )
    })?;

    maybe_install_powershell7(reporter, options);

    reporter.info(&tr(
        "install-installing",
        &[("path", &layout.root().display().to_string())],
    ));
    for dir in layout.managed_dirs() {
        fs::create_dir_all(&dir)
            .map_err(|error| format!("pyenv: cannot create {}: {error}", dir.display()))?;
    }
    sweep_replaced_binaries(&layout.bin_dir());

    copy_share_assets(reporter, &sources, &layout);
    install_binaries(reporter, &sources, &layout)?;

    if cfg!(windows) {
        write_windows_wrappers(reporter, &layout, sources.mcp.is_some())?;
    }

    #[cfg(target_os = "linux")]
    if sources.gui.is_some() {
        match super::desktop::install_desktop_entry(&layout) {
            Ok(lines) => {
                for line in lines {
                    reporter.info(&line);
                }
            }
            Err(error) => reporter.warn(&format!(
                "Could not install the GUI desktop launcher ({error}); the app menu entry may be missing."
            )),
        }
    }

    let profiles_changed = configure_shells(reporter, options, &layout);

    if options.refresh_shims {
        run_checked(&installed_exe, layout.root(), &["rehash"])
            .map_err(|error| format!("pyenv: rehash failed after install: {error}"))?;
        reporter.info("Refreshed shims.");
    }

    run_sanity_checks(reporter, &layout, &sources)?;

    sweep_replaced_binaries(&layout.bin_dir());
    finish(reporter, options, &layout, &log_path, &profiles_changed);

    if options.relaunch_gui && layout.gui_exe().is_file() {
        relaunch_gui(reporter, &layout);
    }

    Ok(SelfInstallOutcome {
        layout,
        log_path,
        profiles_changed,
    })
}

fn wait_for_exit(pid: u32, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while process_exists(pid) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(250));
    }
}

fn normalize_root(root: &Path) -> Result<PathBuf, String> {
    let text = root.to_string_lossy();
    let expanded = if text == "~" || text.starts_with("~/") || text.starts_with("~\\") {
        let home = profile::home_dir()
            .ok_or_else(|| "pyenv: cannot expand `~` without a home directory".to_string())?;
        home.join(text[1..].trim_start_matches(['/', '\\']))
    } else {
        root.to_path_buf()
    };
    if expanded.as_os_str().is_empty() {
        return Err("pyenv: the install root cannot be empty".to_string());
    }
    if expanded.is_absolute() {
        Ok(expanded)
    } else {
        env::current_dir()
            .map(|cwd| cwd.join(expanded))
            .map_err(|error| format!("pyenv: cannot resolve the install root: {error}"))
    }
}

fn resolve_sources(options: &SelfInstallOptions) -> Result<Sources, String> {
    let source_dir = match &options.source_dir {
        Some(dir) => dir.clone(),
        None => env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
            .ok_or_else(|| {
                "pyenv: cannot determine where the installer binaries are".to_string()
            })?,
    };
    let pick = |explicit: &Option<PathBuf>, base: &str| -> Option<PathBuf> {
        let candidate = explicit
            .clone()
            .unwrap_or_else(|| source_dir.join(executable_file_name(base)));
        candidate.is_file().then_some(candidate)
    };

    let pyenv = pick(&options.source_pyenv, PYENV_BINARY).ok_or_else(|| {
        format!(
            "pyenv-native source binary `{}` was not found in {}. Pass --source-dir or --source-path.",
            executable_file_name(PYENV_BINARY),
            source_dir.display()
        )
    })?;
    Ok(Sources {
        pyenv,
        mcp: pick(&options.source_mcp, MCP_BINARY),
        gui: pick(&options.source_gui, GUI_BINARY),
    })
}

fn assert_root_state(options: &SelfInstallOptions, layout: &InstallLayout) -> Result<(), String> {
    if layout.pyenv_exe().exists() || !layout.root().is_dir() || options.force || options.yes {
        return Ok(());
    }
    let has_foreign_content = fs::read_dir(layout.root())
        .map(|entries| entries.flatten().any(|entry| entry.file_name() != "logs"))
        .unwrap_or(false);
    if has_foreign_content {
        return Err(format!(
            "pyenv: install root `{}` already exists and is not empty. Re-run with --force or choose a different --install-root.",
            layout.root().display()
        ));
    }
    Ok(())
}

/// Fail early, before asking for confirmation, when the root cannot be written. Probes the root
/// (or its nearest existing ancestor) without creating anything.
fn assert_root_access(layout: &InstallLayout) -> Result<(), String> {
    let mut anchor = layout.root();
    while !anchor.exists() {
        match anchor.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => anchor = parent,
            _ => break,
        }
    }
    let probe = anchor.join(format!(".pyenv-native-write-test-{}", std::process::id()));
    match fs::write(&probe, b"") {
        Ok(()) => {
            let _ = fs::remove_file(&probe);
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => Err(format!(
            "pyenv: install root `{}` requires elevated permissions. Choose a user-writable --install-root.",
            layout.root().display()
        )),
        Err(error) => Err(format!(
            "pyenv: cannot use install root `{}`: {error}",
            layout.root().display()
        )),
    }
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    let file = executable_file_name(name);
    env::split_paths(&paths)
        .map(|dir| dir.join(&file))
        .find(|candidate| candidate.is_file())
}

fn warn_about_other_pyenv(reporter: &mut Reporter<'_>, layout: &InstallLayout) {
    let Some(existing) = find_on_path(PYENV_BINARY) else {
        return;
    };
    if !existing.starts_with(layout.bin_dir()) {
        reporter.warn(&format!(
            "A different pyenv command is already discoverable at `{}`. Restart your shell after install and verify PATH ordering.",
            existing.display()
        ));
    }
}

fn print_summary(
    reporter: &mut Reporter<'_>,
    options: &SelfInstallOptions,
    sources: &Sources,
    layout: &InstallLayout,
    log_path: &Path,
) {
    let optional = |path: &Option<PathBuf>| {
        path.as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<not found>".to_string())
    };
    let profile_effective = options.update_shell_profile && options.shell.has_profile();
    reporter.plain("");
    reporter.plain(&tr("install-summary-title", &[]));
    reporter.plain("============================");
    reporter.plain(&format!("source binary : {}", sources.pyenv.display()));
    reporter.plain(&format!("source mcp    : {}", optional(&sources.mcp)));
    reporter.plain(&format!("source gui    : {}", optional(&sources.gui)));
    reporter.plain(&format!("install root  : {}", layout.root().display()));
    reporter.plain(&format!("installed exe : {}", layout.pyenv_exe().display()));
    reporter.plain(&format!("shell         : {}", options.shell.as_str()));
    reporter.plain(&format!("profile update: {profile_effective}"));
    if cfg!(windows) {
        reporter.plain(&format!("user PATH     : {}", options.add_to_user_path));
    }
    reporter.plain(&format!("refresh shims : {}", options.refresh_shims));
    reporter.plain(&format!("force         : {}", options.force));
    reporter.plain(&format!("log path      : {}", log_path.display()));
    reporter.plain("");
    reporter.plain(&tr("install-summary-blurb", &[]));
    reporter.plain(&tr("install-summary-blurb-detail", &[]));
    reporter.plain(&tr(
        if profile_effective {
            "install-profile-yes"
        } else {
            "install-profile-no"
        },
        &[],
    ));
    reporter.plain("");
}

fn confirm(options: &SelfInstallOptions) -> Result<(), String> {
    if options.yes || options.force {
        return Ok(());
    }
    if !io::stdin().is_terminal() {
        return Err(tr("install-need-yes", &[]));
    }
    print!("{} ", tr("install-continue", &[]));
    let _ = io::stdout().flush();
    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .map_err(|error| format!("pyenv: failed to read confirmation: {error}"))?;
    match answer.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => Ok(()),
        _ => Err(tr("install-cancelled", &[])),
    }
}

fn maybe_install_powershell7(reporter: &mut Reporter<'_>, options: &SelfInstallOptions) {
    if !cfg!(windows) || command_on_path("pwsh") {
        return;
    }
    if !command_on_path("winget") {
        reporter.warn(
            "PowerShell 7 (pwsh) is not installed and winget was not found. Install later with: winget install --id Microsoft.PowerShell",
        );
        return;
    }
    let install = match options.install_powershell7 {
        PowerShell7Mode::Always => true,
        PowerShell7Mode::Never => false,
        PowerShell7Mode::Auto if options.yes || options.force => true,
        PowerShell7Mode::Auto => ask_to_install_powershell7(),
    };
    if !install {
        reporter.warn(
            "Skipping PowerShell 7 install. You can run: winget install --id Microsoft.PowerShell",
        );
        return;
    }
    reporter.info("Installing PowerShell 7 via winget (Microsoft.PowerShell)...");
    let status = Command::new("winget")
        .headless()
        .args([
            "install",
            "--id",
            "Microsoft.PowerShell",
            "-e",
            "--accept-package-agreements",
            "--accept-source-agreements",
            "--disable-interactivity",
        ])
        .status();
    match status {
        Ok(status) if status.success() => reporter.info("Installed PowerShell 7."),
        Ok(status) => reporter.warn(&format!(
            "winget could not install PowerShell 7 (exit {:?}); continuing without it.",
            status.code()
        )),
        Err(error) => reporter.warn(&format!(
            "Could not run winget ({error}); continuing without PowerShell 7."
        )),
    }
}

fn ask_to_install_powershell7() -> bool {
    if !io::stdin().is_terminal() {
        return false;
    }
    print!("PowerShell 7 is recommended for pyenv-native. Install with winget now? [Y/n] ");
    let _ = io::stdout().flush();
    let mut answer = String::new();
    if io::stdin().read_line(&mut answer).is_err() {
        return false;
    }
    !matches!(answer.trim().to_ascii_lowercase().as_str(), "n" | "no")
}

fn copy_share_assets(reporter: &mut Reporter<'_>, sources: &Sources, layout: &InstallLayout) {
    let Some(source_share) = sources.pyenv.parent().map(|dir| dir.join("share")) else {
        return;
    };
    if !source_share.is_dir() || source_share == layout.share_dir() {
        return;
    }
    match copy_dir_recursive(&source_share, &layout.share_dir()) {
        Ok(()) => reporter.info(&format!(
            "Installed desktop integration assets into {}",
            layout.share_dir().display()
        )),
        Err(error) => reporter.warn(&format!("Could not copy desktop assets: {error}")),
    }
}

fn copy_dir_recursive(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn install_one(
    reporter: &mut Reporter<'_>,
    label: &str,
    source: &Path,
    destination: PathBuf,
) -> Result<(), String> {
    match install_binary(source, &destination) {
        Ok(BinaryInstall::Installed) => {
            reporter.info(&format!("Installed {label} into {}", destination.display()));
            Ok(())
        }
        Ok(BinaryInstall::AlreadyInPlace) => {
            reporter.info(&format!(
                "{label} is already in place at {}",
                destination.display()
            ));
            Ok(())
        }
        Err(error) => Err(format!(
            "pyenv: failed to install {label} to {}: {error}",
            destination.display()
        )),
    }
}

fn install_binaries(
    reporter: &mut Reporter<'_>,
    sources: &Sources,
    layout: &InstallLayout,
) -> Result<(), String> {
    install_one(reporter, "pyenv", &sources.pyenv, layout.pyenv_exe())?;
    match &sources.mcp {
        Some(mcp) => install_one(
            reporter,
            "the MCP server (pyenv-mcp)",
            mcp,
            layout.mcp_exe(),
        )?,
        None => reporter.warn("pyenv-mcp source binary was not found; installing pyenv CLI only."),
    }
    if let Some(gui) = &sources.gui {
        install_one(
            reporter,
            "the GUI companion (pyenv-gui)",
            gui,
            layout.gui_exe(),
        )?;
    }
    Ok(())
}

fn write_windows_wrappers(
    reporter: &mut Reporter<'_>,
    layout: &InstallLayout,
    include_mcp: bool,
) -> Result<(), String> {
    for (name, contents) in windows_wrapper_files(include_mcp) {
        let path = layout.bin_dir().join(name);
        fs::write(&path, contents)
            .map_err(|error| format!("pyenv: cannot write {}: {error}", path.display()))?;
    }
    reporter.info(&format!(
        "Installed command wrappers into {}",
        layout.bin_dir().display()
    ));
    Ok(())
}

/// PATH and shell-profile integration. Failures here are warnings: the binaries are installed
/// and usable, and the user can finish integration with `pyenv self-install` or by hand.
fn configure_shells(
    reporter: &mut Reporter<'_>,
    options: &SelfInstallOptions,
    layout: &InstallLayout,
) -> Vec<PathBuf> {
    let mut changed = Vec::new();

    if cfg!(windows) && options.add_to_user_path {
        match user_path::add_user_path_entries(&[layout.bin_dir(), layout.shims_dir()]) {
            Ok(true) => {
                reporter.info("Updated user PATH to include the install bin and shims directories.")
            }
            Ok(false) => {
                reporter.info("User PATH already includes the install bin and shims directories.")
            }
            Err(error) => reporter.warn(&format!("Could not update the user PATH: {error}")),
        }
    }

    if !options.update_shell_profile || !options.shell.has_profile() {
        if options.add_to_user_path && !cfg!(windows) {
            reporter.warn(
                "Persistent PATH integration on POSIX systems happens through your shell profile. Add the install bin directory to PATH manually if needed.",
            );
        }
        return changed;
    }

    let Some(home) = profile::home_dir() else {
        reporter.warn("Could not resolve the home directory; skipping shell profile update.");
        return changed;
    };

    let mut targets = Vec::new();
    if let Some(path) = profile::profile_path(options.shell, &home) {
        targets.push((options.shell, path));
    }
    if cfg!(windows) && options.shell == ShellKind::Pwsh {
        targets.push((
            ShellKind::Pwsh,
            profile::powershell_profile_path(&home, false),
        ));
    }

    for (kind, path) in targets {
        match profile::apply_profile_block(kind, &path, &layout.pyenv_exe(), &layout.bin_dir()) {
            Ok(ProfileUpdate::Unchanged) => {
                reporter.info(&format!(
                    "Shell profile already current: {}",
                    path.display()
                ));
            }
            Ok(ProfileUpdate::AlreadyConfigured) => reporter.info(&format!(
                "Shell profile {} already initialises pyenv; left unchanged.",
                path.display()
            )),
            Ok(_) => {
                reporter.info(&format!("Updated shell profile at {}", path.display()));
                changed.push(path);
            }
            Err(error) => reporter.warn(&format!(
                "Could not update the shell profile {}: {error}",
                path.display()
            )),
        }
    }
    changed
}

fn run_checked(exe: &Path, root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new(exe)
        .headless()
        .env("PYENV_ROOT", root)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("could not run {}: {error}", exe.display()))?;
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).trim().to_string();
    if output.status.success() {
        Ok(text(&output.stdout))
    } else {
        let stderr = text(&output.stderr);
        let stdout = text(&output.stdout);
        Err(format!(
            "exit {:?}: {}",
            output.status.code(),
            [stderr, stdout]
                .into_iter()
                .find(|part| !part.is_empty())
                .unwrap_or_default()
        ))
    }
}

fn sanity_check(
    reporter: &mut Reporter<'_>,
    root: &Path,
    name: &str,
    exe: &Path,
    args: &[&str],
) -> Result<(), String> {
    match run_checked(exe, root, args) {
        Ok(output) => {
            match output.lines().next().filter(|line| !line.is_empty()) {
                Some(first) => reporter.info(&format!("Sanity check passed: {name} -> {first}")),
                None => reporter.info(&format!("Sanity check passed: {name}")),
            }
            Ok(())
        }
        Err(error) => {
            reporter.error(&format!("Sanity check failed: {name}"));
            reporter.plain(&error);
            Err(format!("Sanity check failed: {name}: {error}"))
        }
    }
}

fn run_sanity_checks(
    reporter: &mut Reporter<'_>,
    layout: &InstallLayout,
    sources: &Sources,
) -> Result<(), String> {
    let root = layout.root();
    let exe = layout.pyenv_exe();
    sanity_check(reporter, root, "pyenv --version", &exe, &["--version"])?;
    sanity_check(reporter, root, "pyenv root", &exe, &["root"])?;
    sanity_check(reporter, root, "pyenv commands", &exe, &["commands"])?;
    if sources.mcp.is_some() {
        sanity_check(
            reporter,
            root,
            "pyenv-mcp guide",
            &layout.mcp_exe(),
            &["guide"],
        )?;
    }
    if sources.gui.is_some() {
        if layout.gui_exe().is_file() {
            reporter.info(&format!(
                "Sanity check passed: pyenv-gui exists at {}",
                layout.gui_exe().display()
            ));
        } else {
            reporter.error("Sanity check failed: pyenv-gui was not installed");
            return Err("Sanity check failed: pyenv-gui was not installed".to_string());
        }
    } else {
        reporter.warn(
            "pyenv-gui was not installed; GUI features stay unavailable until a GUI bundle is installed.",
        );
    }
    Ok(())
}

fn finish(
    reporter: &mut Reporter<'_>,
    options: &SelfInstallOptions,
    layout: &InstallLayout,
    log_path: &Path,
    profiles_changed: &[PathBuf],
) {
    reporter.info("Install completed successfully.");
    reporter.plain(&tr("install-done", &[]));
    reporter.plain("");
    reporter.plain(&tr(
        "install-installed-to",
        &[("path", &layout.root().display().to_string())],
    ));
    reporter.plain(&tr(
        "install-installed-command",
        &[("path", &layout.pyenv_exe().display().to_string())],
    ));
    if layout.mcp_exe().is_file() {
        let mcp = layout.mcp_exe().display().to_string();
        reporter.plain(&tr("install-installed-mcp", &[("path", &mcp)]));
        reporter.plain(&tr(
            "install-mcp-helper",
            &[("command", &format!("{mcp} print-config"))],
        ));
    }
    if layout.gui_exe().is_file() {
        reporter.plain(&tr(
            "install-installed-gui",
            &[("path", &layout.gui_exe().display().to_string())],
        ));
    }
    reporter.plain(&tr(
        "install-log-file",
        &[("path", &log_path.display().to_string())],
    ));

    if options.shell == ShellKind::Cmd {
        reporter.plain(&format!(
            "CMD note: run '{}\\pyenv-init.cmd' in each interactive CMD session, or wire it into your preferred startup flow.",
            layout.bin_dir().display()
        ));
    }
    if cfg!(windows) && options.add_to_user_path {
        reporter.plain("PATH note: restart your shell to pick up the updated user PATH.");
    }
    if let Some(hint) =
        profile::current_shell_hint(options.shell, &layout.pyenv_exe(), &layout.bin_dir())
    {
        reporter.plain("");
        reporter
            .warn("This installer ran in its own process and cannot modify an already-open shell.");
        reporter.info("To use pyenv in your current shell right now, run:");
        reporter.plain(&hint);
    }
    if let Some(profile_path) = profiles_changed.first() {
        let reload = if options.shell == ShellKind::Fish {
            "source"
        } else {
            "."
        };
        reporter.info(&format!(
            "Or reload your profile instead: {reload} {}",
            profile_path.display()
        ));
    }
}

fn relaunch_gui(reporter: &mut Reporter<'_>, layout: &InstallLayout) {
    match Command::new(layout.gui_exe())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(_) => reporter.info("Relaunched the GUI."),
        Err(error) => reporter.warn(&format!("Could not relaunch the GUI: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parses_powershell7_modes() {
        assert_eq!(PowerShell7Mode::parse("auto"), Some(PowerShell7Mode::Auto));
        assert_eq!(
            PowerShell7Mode::parse("TRUE"),
            Some(PowerShell7Mode::Always)
        );
        assert_eq!(PowerShell7Mode::parse("off"), Some(PowerShell7Mode::Never));
        assert_eq!(PowerShell7Mode::parse("maybe"), None);
    }

    #[test]
    fn expands_tilde_and_rejects_empty_roots() {
        assert!(normalize_root(Path::new("")).is_err());
        let absolute = if cfg!(windows) {
            "C:\\pyenv"
        } else {
            "/opt/pyenv"
        };
        assert_eq!(
            normalize_root(Path::new(absolute)).unwrap(),
            Path::new(absolute)
        );
        if let Some(home) = profile::home_dir() {
            assert_eq!(normalize_root(Path::new("~/x")).unwrap(), home.join("x"));
        }
    }

    #[test]
    fn missing_primary_binary_is_a_clear_error() {
        let temp = TempDir::new().expect("tempdir");
        let mut options = SelfInstallOptions::new(temp.path().join("root"));
        options.source_dir = Some(temp.path().join("empty"));
        let error = resolve_sources(&options).unwrap_err();
        assert!(error.contains("was not found"), "{error}");
    }

    #[test]
    fn optional_binaries_are_discovered_beside_the_primary() {
        let temp = TempDir::new().expect("tempdir");
        for base in ["pyenv", "pyenv-mcp"] {
            fs::write(temp.path().join(executable_file_name(base)), "x").unwrap();
        }
        let mut options = SelfInstallOptions::new(temp.path().join("root"));
        options.source_dir = Some(temp.path().to_path_buf());
        let sources = resolve_sources(&options).unwrap();
        assert!(sources.mcp.is_some());
        assert!(sources.gui.is_none());
    }

    #[test]
    fn non_empty_foreign_roots_need_force() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join("root");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("notes.txt"), "mine").unwrap();
        let layout = InstallLayout::new(&root);

        let mut options = SelfInstallOptions::new(&root);
        assert!(assert_root_state(&options, &layout).is_err());
        options.force = true;
        assert!(assert_root_state(&options, &layout).is_ok());

        let logs_only = temp.path().join("logs-only");
        fs::create_dir_all(logs_only.join("logs")).unwrap();
        let options = SelfInstallOptions::new(&logs_only);
        assert!(assert_root_state(&options, &InstallLayout::new(&logs_only)).is_ok());
    }

    #[test]
    fn noninteractive_runs_without_confirmation_flags_fail_cleanly() {
        let mut options = SelfInstallOptions::new("/tmp/never-used");
        options.yes = false;
        options.force = false;
        // Test runners have no terminal on stdin, so the prompt must refuse rather than block.
        if !io::stdin().is_terminal() {
            assert!(confirm(&options).is_err());
        }
    }
}
