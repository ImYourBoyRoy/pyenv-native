#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
// ./crates/pyenv-gui/src/main.rs
//! Tauri v2 backend for pyenv-native GUI.
//!
//! All long-running or blocking core operations are dispatched via
//! `tokio::task::spawn_blocking` so the GUI thread stays responsive.
//! Commands that would normally read stdin (uninstall confirmation,
//! self-update confirmation) use force/yes flags since the GUI provides
//! its own confirmation modals.

mod desktop_integration;

use pyenv_core::PyenvCommandExt;

fn get_context_with_dir(workspace_dir: Option<String>) -> Result<pyenv_core::AppContext, String> {
    let mut ctx = pyenv_core::AppContext::from_system().map_err(|e| e.to_string())?;
    if let Some(dir_str) = workspace_dir {
        if !dir_str.trim().is_empty() {
            let path = std::path::PathBuf::from(dir_str);
            if path.exists() && path.is_dir() {
                ctx.dir = path;
            }
        }
    }
    Ok(ctx)
}

#[tauri::command]
fn get_status(workspace_dir: Option<String>) -> Result<String, String> {
    let ctx = get_context_with_dir(workspace_dir)?;
    let report = pyenv_core::cmd_status(&ctx, true);
    if report.exit_code != 0 {
        return Err(report.stderr.join("\n"));
    }
    Ok(report.stdout.join("\n"))
}

#[tauri::command]
fn get_available_versions(
    workspace_dir: Option<String>,
    family: Option<String>,
    pattern: Option<String>,
) -> Result<String, String> {
    let ctx = get_context_with_dir(workspace_dir)?;
    let report = pyenv_core::cmd_available(&ctx, family, pattern, false, true);
    if report.exit_code != 0 {
        let errs = report.stderr.join("\n");
        if errs.is_empty() {
            return Err("Failed to fetch available versions.".to_string());
        }
        return Err(errs);
    }
    Ok(report.stdout.join("\n"))
}

#[tauri::command]
fn get_installed_versions(workspace_dir: Option<String>) -> Result<String, String> {
    let ctx = get_context_with_dir(workspace_dir)?;
    let names = pyenv_core::installed_version_names(&ctx).map_err(|e| e.to_string())?;
    serde_json::to_string(&names).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_managed_venvs(workspace_dir: Option<String>) -> Result<String, String> {
    let ctx = get_context_with_dir(workspace_dir)?;
    let report = pyenv_core::cmd_venv_list(&ctx, false, true);
    if report.exit_code != 0 {
        return Err(report.stderr.join("\n"));
    }
    Ok(report.stdout.join("\n"))
}

#[tauri::command]
fn close_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn minimize_app(window: tauri::Window) {
    let _ = window.minimize();
}

#[tauri::command]
fn maximize_app(window: tauri::Window) {
    if window.is_maximized().unwrap_or(false) {
        let _ = window.unmaximize();
    } else {
        let _ = window.maximize();
    }
}

/// Install a Python version asynchronously so the GUI stays responsive.
#[tauri::command]
async fn install_version(workspace_dir: Option<String>, version: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let options = pyenv_core::InstallCommandOptions {
            list: false,
            force: false,
            dry_run: false,
            json: false,
            known: false,
            family: None,
            versions: vec![version],
        };
        let report = pyenv_core::cmd_install(&ctx, &options);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }

        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

/// Uninstall a Python version asynchronously.
/// Uses `force=true` because the GUI provides its own confirmation modal,
/// avoiding the stdin-blocking `confirm_uninstall()` prompt.
#[tauri::command]
async fn uninstall_version(
    workspace_dir: Option<String>,
    version: String,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let report = pyenv_core::cmd_uninstall(&ctx, &[version], true);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

#[tauri::command]
fn select_directory() -> Result<Option<String>, String> {
    if let Some(path) = rfd::FileDialog::new().pick_folder() {
        Ok(Some(path.display().to_string()))
    } else {
        Ok(None)
    }
}

#[tauri::command]
fn set_global(workspace_dir: Option<String>, version: String) -> Result<String, String> {
    let ctx = get_context_with_dir(workspace_dir)?;
    let report = pyenv_core::cmd_global(&ctx, &[version], false);
    if report.exit_code != 0 {
        return Err(report.stderr.join("\n"));
    }
    let rehash = pyenv_core::cmd_rehash(&ctx);
    if rehash.exit_code != 0 {
        return Err(rehash.stderr.join("\n"));
    }
    Ok(report.stdout.join("\n"))
}

#[tauri::command]
fn set_local(version: String, path: String) -> Result<String, String> {
    let ctx = get_context_with_dir(Some(path.clone()))?;
    let target = std::path::Path::new(&path).join(".python-version");
    let report = pyenv_core::cmd_version_file_write(&ctx, &target, &[version], false);
    if report.exit_code != 0 {
        return Err(report.stderr.join("\n"));
    }
    Ok(report.stdout.join("\n"))
}

#[tauri::command]
fn get_config(workspace_dir: Option<String>) -> Result<String, String> {
    let ctx = get_context_with_dir(workspace_dir)?;
    serde_json::to_string(&ctx.config).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_config(workspace_dir: Option<String>, key: String, value: String) -> Result<String, String> {
    let mut ctx = get_context_with_dir(workspace_dir)?;
    let report = pyenv_core::cmd_config_set(&mut ctx, &key, &value);
    if report.exit_code != 0 {
        return Err(report.stderr.join("\n"));
    }
    Ok(report.stdout.join("\n"))
}

#[tauri::command]
async fn create_venv(
    workspace_dir: Option<String>,
    base_version: String,
    name: String,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let report = pyenv_core::cmd_venv_create(&ctx, &base_version, &name, false, false);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

#[tauri::command]
async fn delete_venv(workspace_dir: Option<String>, spec: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let report = pyenv_core::cmd_venv_delete(&ctx, &spec, true);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

/// Check for pyenv-native updates using the core self-update API (check-only mode).
/// Returns a human-readable status string.
#[tauri::command]
async fn check_for_updates(workspace_dir: Option<String>) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let options = pyenv_core::SelfUpdateOptions {
            check: true,
            yes: false,
            force: false,
            github_repo: None,
            tag: None,
            restart_gui: false,
        };
        let report = pyenv_core::cmd_self_update(&ctx, &options);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

/// Perform the actual self-update with `yes=true` to skip interactive confirmation.
#[tauri::command]
async fn perform_update(workspace_dir: Option<String>) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let options = pyenv_core::SelfUpdateOptions {
            check: false,
            yes: true,
            force: false,
            github_repo: None,
            tag: None,
            restart_gui: true,
        };
        let report = pyenv_core::cmd_self_update(&ctx, &options);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

#[derive(serde::Serialize)]
struct VenvUpgradeInfo {
    name: String,
    base_version: String,
    packages: Vec<String>,
}

#[tauri::command]
async fn get_venv_upgrade_info(
    workspace_dir: Option<String>,
    spec: String,
) -> Result<VenvUpgradeInfo, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let info = pyenv_core::resolve_managed_venv(&ctx, &spec).map_err(|e| e.to_string())?;

        let py_path = info
            .python_path
            .as_ref()
            .ok_or_else(|| format!("pyenv: interpreter for managed venv '{}' is missing.", spec))?;

        let mut packages = Vec::new();
        if let Ok(output) = std::process::Command::new(py_path)
            .headless()
            .args(["-m", "pip", "list", "--format=json"])
            .output()
        {
            if output.status.success() {
                let stdout_str = String::from_utf8_lossy(&output.stdout);
                if let Ok(pkgs) = serde_json::from_str::<Vec<pyenv_core::PipPackage>>(&stdout_str) {
                    packages = pkgs;
                }
            }
        }

        let custom_packages: Vec<String> = packages
            .into_iter()
            .filter(|p| {
                let name_lower = p.name.to_lowercase();
                name_lower != "pip"
                    && name_lower != "setuptools"
                    && name_lower != "wheel"
                    && name_lower != "distribute"
            })
            .map(|p| format!("{}=={}", p.name, p.version))
            .collect();

        Ok(VenvUpgradeInfo {
            name: info.name,
            base_version: info.base_version,
            packages: custom_packages,
        })
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

#[tauri::command]
async fn install_pip_packages(
    workspace_dir: Option<String>,
    target: String,
    packages: Vec<String>,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let py_path = pyenv_core::resolve_interpreter_path(&ctx, &target)?;

        let mut stdout = Vec::new();
        for pkg in packages {
            let output = std::process::Command::new(&py_path)
                .headless()
                .args(["-m", "pip", "install", &pkg])
                .output()
                .map_err(|e| format!("Failed to run pip install for {pkg}: {e}"))?;

            if !output.status.success() {
                return Err(format!(
                    "Failed to install package {pkg}: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            stdout.push(format!("Successfully installed {pkg}"));
        }
        Ok(stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

#[tauri::command]
async fn get_pip_packages(workspace_dir: Option<String>, target: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let report = pyenv_core::cmd_pip_list(&ctx, &target, true);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

#[tauri::command]
async fn get_outdated_packages(
    workspace_dir: Option<String>,
    target: String,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let report = pyenv_core::cmd_pip_outdated(&ctx, &target, true);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

#[tauri::command]
async fn check_pip_conflicts(
    workspace_dir: Option<String>,
    target: String,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let report = pyenv_core::cmd_pip_check(&ctx, &target, true);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

#[tauri::command]
async fn precheck_requirements(
    workspace_dir: Option<String>,
    target: String,
    path_or_url: String,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let report = pyenv_core::cmd_pip_precheck_requirements(&ctx, &target, &path_or_url);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

#[tauri::command]
async fn install_requirements(
    workspace_dir: Option<String>,
    target: String,
    path_or_url: String,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let report = pyenv_core::cmd_pip_install(&ctx, &target, &path_or_url);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

#[tauri::command]
async fn update_pip_packages(
    workspace_dir: Option<String>,
    target: String,
    packages: Vec<String>,
    all: Option<bool>,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let is_all = all.unwrap_or(false);
        let report = pyenv_core::cmd_pip_update(&ctx, &target, &packages, is_all);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

#[tauri::command]
async fn analyze_codebase_imports(
    workspace_dir: Option<String>,
    target: String,
    dir_path: String,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let report = pyenv_core::cmd_pip_analyze_imports(&ctx, &target, &dir_path);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

/// Returns the app version from the Cargo package metadata at compile time.
#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", &url])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct ShellStatus {
    name: String,
    profile_path: String,
    is_configured: bool,
    active_in_path: bool,
    is_installed: bool,
    /// Human-readable PATH status for the GUI badges.
    path_label: String,
}

fn is_executable_in_path(name: &str) -> bool {
    let name_ext = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };

    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let target = dir.join(&name_ext);
            if target.is_file() {
                return true;
            }
            if cfg!(windows) {
                let target_no_ext = dir.join(name);
                if target_no_ext.is_file() {
                    return true;
                }
            }
        }
    }
    false
}

fn get_home_dir() -> Result<std::path::PathBuf, String> {
    if cfg!(windows) {
        std::env::var("USERPROFILE")
            .map(std::path::PathBuf::from)
            .map_err(|_| "Could not resolve USERPROFILE environment variable".to_string())
    } else {
        std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .map_err(|_| "Could not resolve HOME environment variable".to_string())
    }
}

fn shell_path_label(is_configured: bool, shims_in_path: bool) -> String {
    if shims_in_path {
        "PATH Active".to_string()
    } else if is_configured {
        // Desktop apps often don't inherit shell-profile PATH; profiles can still be correct.
        "Restart terminal to activate".to_string()
    } else {
        "Shims not on PATH".to_string()
    }
}

#[tauri::command]
fn get_shell_statuses(workspace_dir: Option<String>) -> Result<Vec<ShellStatus>, String> {
    let ctx = get_context_with_dir(workspace_dir)?;
    let mut statuses = Vec::new();
    let shims_dir = ctx.shims_dir();
    let path_val = std::env::var("PATH").unwrap_or_default();
    let shims_in_path = path_has_entry(&path_val, &shims_dir);

    // Resolve home directory
    let home = get_home_dir()?;

    // 1. PowerShell 7 Profile
    let pwsh_profile = if cfg!(windows) {
        home.join("Documents")
            .join("PowerShell")
            .join("Microsoft.PowerShell_profile.ps1")
    } else {
        home.join(".config")
            .join("powershell")
            .join("Microsoft.PowerShell_profile.ps1")
    };
    let pwsh_configured = pwsh_profile.is_file() && {
        let content = std::fs::read_to_string(&pwsh_profile).unwrap_or_default();
        content.contains("pyenv init")
    };
    statuses.push(ShellStatus {
        name: "PowerShell 7 (pwsh)".to_string(),
        profile_path: pwsh_profile.to_string_lossy().to_string(),
        is_configured: pwsh_configured,
        active_in_path: shims_in_path,
        is_installed: is_executable_in_path("pwsh"),
        path_label: shell_path_label(pwsh_configured, shims_in_path),
    });

    // 2. Windows PowerShell 5.1 Profile (Windows only)
    if cfg!(windows) {
        let win_ps_profile = home
            .join("Documents")
            .join("WindowsPowerShell")
            .join("Microsoft.PowerShell_profile.ps1");
        let win_ps_configured = win_ps_profile.is_file() && {
            let content = std::fs::read_to_string(&win_ps_profile).unwrap_or_default();
            content.contains("pyenv init")
        };
        statuses.push(ShellStatus {
            name: "Windows PowerShell 5.1".to_string(),
            profile_path: win_ps_profile.to_string_lossy().to_string(),
            is_configured: win_ps_configured,
            active_in_path: shims_in_path,
            is_installed: true, // Always true on Windows
            path_label: shell_path_label(win_ps_configured, shims_in_path),
        });
    }

    // 3. Zsh Profile
    let zsh_profile = home.join(".zshrc");
    let zsh_configured = zsh_profile.is_file() && {
        let content = std::fs::read_to_string(&zsh_profile).unwrap_or_default();
        content.contains("pyenv init")
    };
    statuses.push(ShellStatus {
        name: "Zsh".to_string(),
        profile_path: zsh_profile.to_string_lossy().to_string(),
        is_configured: zsh_configured,
        active_in_path: shims_in_path,
        is_installed: if cfg!(target_os = "macos") {
            true
        } else {
            is_executable_in_path("zsh")
        },
        path_label: shell_path_label(zsh_configured, shims_in_path),
    });

    // 4. Bash Profile
    let bash_profile = home.join(".bashrc");
    let bash_configured = bash_profile.is_file() && {
        let content = std::fs::read_to_string(&bash_profile).unwrap_or_default();
        content.contains("pyenv init")
    };
    statuses.push(ShellStatus {
        name: "Bash".to_string(),
        profile_path: bash_profile.to_string_lossy().to_string(),
        is_configured: bash_configured,
        active_in_path: shims_in_path,
        is_installed: if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
            true
        } else {
            is_executable_in_path("bash")
        },
        path_label: shell_path_label(bash_configured, shims_in_path),
    });

    // 5. Fish Profile
    let fish_profile = home.join(".config").join("fish").join("config.fish");
    let fish_configured = fish_profile.is_file() && {
        let content = std::fs::read_to_string(&fish_profile).unwrap_or_default();
        content.contains("pyenv init")
    };
    statuses.push(ShellStatus {
        name: "Fish".to_string(),
        profile_path: fish_profile.to_string_lossy().to_string(),
        is_configured: fish_configured,
        active_in_path: shims_in_path,
        is_installed: is_executable_in_path("fish"),
        path_label: shell_path_label(fish_configured, shims_in_path),
    });

    Ok(statuses)
}

#[tauri::command]
fn configure_shell(shell_name: String, profile_path: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(&profile_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {e}"))?;
    }

    let mut content = if path.exists() {
        std::fs::read_to_string(&path).unwrap_or_default()
    } else {
        String::new()
    };

    if content.contains("pyenv init") {
        return Ok(());
    }

    // Format the initialization block to append
    let init_block = if shell_name.contains("PowerShell") {
        "\n# pyenv-native shell initialization\niex ((pyenv init - pwsh) -join \"`n\")\n"
    } else if shell_name.contains("Zsh") {
        "\n# pyenv-native shell initialization\neval \"$(pyenv init - zsh)\"\n"
    } else if shell_name.contains("Bash") {
        "\n# pyenv-native shell initialization\neval \"$(pyenv init - bash)\"\n"
    } else if shell_name.contains("Fish") {
        "\n# pyenv-native shell initialization\npyenv init - fish | source\n"
    } else {
        return Err(format!(
            "Unsupported shell for automatic configuration: {shell_name}"
        ));
    };

    content = pyenv_core::append_text_block(content, init_block);

    std::fs::write(&path, content).map_err(|e| format!("Failed to write profile: {e}"))?;
    Ok(())
}

#[derive(serde::Serialize)]
struct InstallStatus {
    /// Managed binaries exist under PYENV_ROOT/bin (or a portable sibling layout).
    is_installed: bool,
    root: Option<String>,
    is_portable: bool,
    /// `pyenv` resolves on the GUI process PATH (often false for desktop launches).
    cli_on_path: bool,
    /// Shell profiles still need pyenv init / a new terminal session.
    needs_shell_setup: bool,
    platform: String,
}

fn path_has_entry(path_env: &str, target: &std::path::Path) -> bool {
    std::env::split_paths(path_env).any(|entry| {
        if cfg!(windows) {
            entry
                .to_string_lossy()
                .replace('/', "\\")
                .eq_ignore_ascii_case(&target.to_string_lossy().replace('/', "\\"))
        } else {
            entry == target
        }
    })
}

fn managed_binaries_present(root: &std::path::Path) -> bool {
    let bin = root.join("bin");
    let pyenv_name = if cfg!(windows) { "pyenv.exe" } else { "pyenv" };
    bin.join(pyenv_name).is_file()
}

fn any_shell_profile_configured(home: &std::path::Path) -> bool {
    let candidates = [
        home.join(".bashrc"),
        home.join(".zshrc"),
        home.join(".config").join("fish").join("config.fish"),
        home.join(".config")
            .join("powershell")
            .join("Microsoft.PowerShell_profile.ps1"),
        home.join("Documents")
            .join("PowerShell")
            .join("Microsoft.PowerShell_profile.ps1"),
        home.join("Documents")
            .join("WindowsPowerShell")
            .join("Microsoft.PowerShell_profile.ps1"),
    ];
    candidates.into_iter().any(|path| {
        path.is_file()
            && std::fs::read_to_string(&path)
                .map(|content| content.contains("pyenv init"))
                .unwrap_or(false)
    })
}

fn find_installer_script(parent: &std::path::Path, names: &[&str]) -> Option<std::path::PathBuf> {
    let mut curr = parent.to_path_buf();
    for _ in 0..8 {
        for name in names {
            let test = curr.join(name);
            if test.is_file() {
                return Some(test);
            }
        }
        // Also check install root share / sibling layouts from a managed install
        if let Some(root) = curr.parent() {
            for name in names {
                let test = root.join(name);
                if test.is_file() {
                    return Some(test);
                }
            }
        }
        if let Some(p) = curr.parent() {
            curr = p.to_path_buf();
        } else {
            break;
        }
    }
    None
}

fn finalize_managed_install(ctx: &pyenv_core::AppContext) -> Result<String, String> {
    let rehash = pyenv_core::cmd_rehash(ctx);
    if rehash.exit_code != 0 {
        return Err(rehash.stderr.join("\n"));
    }

    let home = get_home_dir()?;
    let mut configured = Vec::new();

    #[cfg(windows)]
    let shells = [
        (
            "PowerShell 7 (pwsh)",
            home.join("Documents")
                .join("PowerShell")
                .join("Microsoft.PowerShell_profile.ps1"),
            true,
        ),
        (
            "Windows PowerShell 5.1",
            home.join("Documents")
                .join("WindowsPowerShell")
                .join("Microsoft.PowerShell_profile.ps1"),
            true,
        ),
    ];
    #[cfg(not(windows))]
    let shells = [
        ("Bash", home.join(".bashrc"), true),
        ("Zsh", home.join(".zshrc"), is_executable_in_path("zsh")),
        (
            "Fish",
            home.join(".config").join("fish").join("config.fish"),
            is_executable_in_path("fish"),
        ),
    ];

    for (name, profile, present) in shells {
        if !present {
            continue;
        }
        let already = profile.is_file()
            && std::fs::read_to_string(&profile)
                .map(|c| c.contains("pyenv init"))
                .unwrap_or(false);
        if already {
            continue;
        }
        if let Ok(()) = configure_shell(name.to_string(), profile.to_string_lossy().to_string()) {
            configured.push(name.to_string());
        }
    }

    let mut lines = vec![
        format!("Core binaries are ready under {}.", ctx.root.display()),
        "Shim directory refreshed.".to_string(),
    ];
    if configured.is_empty() {
        lines.push(
            "Shell profiles already include pyenv init. Open a new terminal so shims appear on PATH."
                .to_string(),
        );
    } else {
        lines.push(format!(
            "Configured shell profile(s): {}. Open a new terminal to activate shims.",
            configured.join(", ")
        ));
    }
    Ok(lines.join("\n"))
}

#[tauri::command]
fn check_install_status() -> InstallStatus {
    let platform = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
    .to_string();

    let cli_on_path = if cfg!(windows) {
        std::process::Command::new("where.exe")
            .headless()
            .arg("pyenv")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        std::process::Command::new("which")
            .arg("pyenv")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };

    let exe_path = std::env::current_exe().unwrap_or_default();
    let parent = exe_path.parent().unwrap_or(std::path::Path::new("."));
    let pyenv_name = if cfg!(windows) { "pyenv.exe" } else { "pyenv" };
    let local_pyenv = parent.join(pyenv_name);
    let is_portable = local_pyenv.exists();

    let ctx = pyenv_core::AppContext::from_system();
    let home = get_home_dir().ok();

    match ctx {
        Ok(c) => {
            let binaries = managed_binaries_present(&c.root) || is_portable;
            let shims_dir = c.shims_dir();
            let path_val = std::env::var("PATH").unwrap_or_default();
            let shims_active = path_has_entry(&path_val, &shims_dir);
            let profile_ok = home
                .as_ref()
                .map(|h| any_shell_profile_configured(h))
                .unwrap_or(false);
            InstallStatus {
                is_installed: binaries,
                root: Some(c.root.to_string_lossy().to_string()),
                is_portable,
                cli_on_path,
                needs_shell_setup: binaries && (!profile_ok || !shims_active),
                platform,
            }
        }
        Err(_) => InstallStatus {
            is_installed: is_portable || cli_on_path,
            root: if is_portable {
                Some(parent.to_string_lossy().to_string())
            } else {
                None
            },
            is_portable,
            cli_on_path,
            needs_shell_setup: is_portable,
            platform,
        },
    }
}

#[tauri::command]
async fn install_local_pyenv() -> Result<String, String> {
    tokio::task::spawn_blocking(|| {
        let exe_path = std::env::current_exe().unwrap_or_default();
        let parent = exe_path.parent().unwrap_or(std::path::Path::new("."));

        // Prefer finalizing an already-managed install (typical after network install).
        if let Ok(ctx) = pyenv_core::AppContext::from_system() {
            if managed_binaries_present(&ctx.root) {
                return finalize_managed_install(&ctx);
            }
        }

        #[cfg(target_os = "windows")]
        {
            let script_names = [
                "install-pyenv-native.ps1",
                "scripts/install-pyenv-native.ps1",
            ];
            let script = find_installer_script(parent, &script_names).ok_or_else(|| {
                "Installer script not found. Your install is incomplete — re-run the network installer from https://github.com/ImYourBoyRoy/pyenv-native/releases/latest/download/install.ps1".to_string()
            })?;

            let pyenv_bin = parent.join("pyenv.exe");
            let mcp_bin = parent.join("pyenv-mcp.exe");
            let gui_bin = parent.join("pyenv-gui.exe");

            let mut args = vec![
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-File".to_string(),
                script.to_string_lossy().to_string(),
                "-Yes".to_string(),
            ];

            if pyenv_bin.exists() {
                args.push("-SourcePath".to_string());
                args.push(pyenv_bin.to_string_lossy().to_string());
            }
            if mcp_bin.exists() {
                args.push("-SourceMcpPath".to_string());
                args.push(mcp_bin.to_string_lossy().to_string());
            }
            if gui_bin.exists() {
                args.push("-SourceGuiPath".to_string());
                args.push(gui_bin.to_string_lossy().to_string());
            }

            let output = std::process::Command::new("powershell")
                .headless()
                .args(&args)
                .output()
                .map_err(|e| e.to_string())?;

            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).to_string());
            }
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        #[cfg(not(target_os = "windows"))]
        {
            let script_names = ["install-pyenv-native.sh", "scripts/install-pyenv-native.sh"];
            let script = find_installer_script(parent, &script_names).ok_or_else(|| {
                "Installer script not found. Your install is incomplete — re-run:\ncurl -fsSL https://github.com/ImYourBoyRoy/pyenv-native/releases/latest/download/install.sh | sh".to_string()
            })?;

            let mut args = vec![script.to_string_lossy().to_string(), "--yes".to_string()];

            let pyenv_bin = parent.join("pyenv");
            if pyenv_bin.exists() {
                args.push("--source-path".to_string());
                args.push(pyenv_bin.to_string_lossy().to_string());
            }
            let mcp_bin = parent.join("pyenv-mcp");
            if mcp_bin.exists() {
                args.push("--source-mcp-path".to_string());
                args.push(mcp_bin.to_string_lossy().to_string());
            }
            let gui_bin = parent.join("pyenv-gui");
            if gui_bin.exists() {
                args.push("--source-gui-path".to_string());
                args.push(gui_bin.to_string_lossy().to_string());
            }

            let output = std::process::Command::new("sh")
                .args(&args)
                .output()
                .map_err(|e| e.to_string())?;

            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).to_string());
            }
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(serde::Serialize)]
struct DoctorCheckGui {
    name: String,
    status: String,
    detail: String,
}

#[tauri::command]
async fn run_doctor(workspace_dir: Option<String>) -> Result<Vec<DoctorCheckGui>, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let checks = pyenv_core::collect_checks(&ctx);
        let gui_checks = checks
            .into_iter()
            .map(|c| DoctorCheckGui {
                name: c.name,
                status: c.status.label().to_string(),
                detail: c.detail,
            })
            .collect();
        Ok(gui_checks)
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

#[tauri::command]
async fn run_doctor_fix(workspace_dir: Option<String>) -> Result<Vec<String>, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let outcome = pyenv_core::apply_doctor_fixes(&ctx).map_err(|e| e.to_string())?;
        Ok(outcome.applied)
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

#[derive(serde::Serialize)]
struct CacheEntryGui {
    name: String,
    path: String,
    bytes: u64,
    exists: bool,
}

#[derive(serde::Serialize)]
struct CacheStatsGui {
    total_bytes: u64,
    entries: Vec<CacheEntryGui>,
}

fn dir_size_bytes(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            total = total.saturating_add(dir_size_bytes(&path));
        } else if let Ok(meta) = entry.metadata() {
            total = total.saturating_add(meta.len());
        }
    }
    total
}

fn cache_entry(name: &str, path: std::path::PathBuf) -> CacheEntryGui {
    let exists = path.exists();
    let bytes = if exists {
        if path.is_dir() {
            dir_size_bytes(&path)
        } else {
            std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
        }
    } else {
        0
    };
    CacheEntryGui {
        name: name.to_string(),
        path: path.to_string_lossy().to_string(),
        bytes,
        exists,
    }
}

#[tauri::command]
fn get_cache_stats(workspace_dir: Option<String>) -> Result<CacheStatsGui, String> {
    let ctx = get_context_with_dir(workspace_dir)?;
    let cache_root = ctx.cache_dir();
    let mut entries = vec![
        cache_entry("Python downloads / packages", cache_root.join("packages")),
        cache_entry("python-build cache", cache_root.join("python-build")),
        cache_entry("Metadata cache", cache_root.join("metadata")),
    ];

    // Best-effort pip cache via `pip cache dir` for the active global interpreter.
    if let Ok(names) = pyenv_core::installed_version_names(&ctx) {
        if let Some(version) = names.first() {
            if let Ok(py) = pyenv_core::resolve_interpreter_path(&ctx, version) {
                if let Ok(output) = std::process::Command::new(&py)
                    .headless()
                    .args(["-m", "pip", "cache", "dir"])
                    .output()
                {
                    if output.status.success() {
                        let pip_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !pip_dir.is_empty() {
                            entries.push(cache_entry(
                                "Pip cache (active runtime)",
                                std::path::PathBuf::from(pip_dir),
                            ));
                        }
                    }
                }
            }
        }
    }

    let total_bytes = entries.iter().map(|e| e.bytes).sum();
    entries.insert(0, cache_entry("All pyenv cache", cache_root.clone()));
    Ok(CacheStatsGui {
        total_bytes,
        entries,
    })
}

#[tauri::command]
fn purge_cache(workspace_dir: Option<String>, target: String) -> Result<String, String> {
    let ctx = get_context_with_dir(workspace_dir)?;
    let cache_root = ctx.cache_dir();
    let allowed = [
        ("packages", cache_root.join("packages")),
        ("python-build", cache_root.join("python-build")),
        ("metadata", cache_root.join("metadata")),
        ("all", cache_root.clone()),
    ];
    let (_, path) = allowed
        .into_iter()
        .find(|(name, _)| *name == target.as_str())
        .ok_or_else(|| format!("Unknown cache target `{target}`"))?;

    if !path.exists() {
        return Ok(format!("Nothing to purge at {}", path.display()));
    }
    // Safety: never delete outside the resolved cache root.
    let canon_root = cache_root
        .canonicalize()
        .unwrap_or_else(|_| cache_root.clone());
    let canon_path = path.canonicalize().map_err(|e| e.to_string())?;
    if !canon_path.starts_with(&canon_root) {
        return Err("Refusing to purge a path outside the pyenv cache root.".to_string());
    }

    if canon_path == canon_root {
        for child in std::fs::read_dir(&canon_path).map_err(|e| e.to_string())? {
            let child = child.map_err(|e| e.to_string())?.path();
            if child.is_dir() {
                std::fs::remove_dir_all(&child).map_err(|e| e.to_string())?;
            } else {
                std::fs::remove_file(&child).map_err(|e| e.to_string())?;
            }
        }
    } else if canon_path.is_dir() {
        std::fs::remove_dir_all(&canon_path).map_err(|e| e.to_string())?;
        let _ = std::fs::create_dir_all(&canon_path);
    } else {
        std::fs::remove_file(&canon_path).map_err(|e| e.to_string())?;
    }

    Ok(format!("Purged cache: {target} ({})", path.display()))
}

fn main() {
    #[cfg(target_os = "linux")]
    desktop_integration::prepare_linux_runtime();

    tauri::Builder::default()
        .setup(|app| desktop_integration::prepare_app(app))
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_available_versions,
            get_installed_versions,
            get_managed_venvs,
            install_version,
            select_directory,
            set_global,
            set_local,
            create_venv,
            delete_venv,
            check_for_updates,
            perform_update,
            get_config,
            set_config,
            close_app,
            minimize_app,
            maximize_app,
            uninstall_version,
            get_app_version,
            open_url,
            check_install_status,
            install_local_pyenv,
            get_pip_packages,
            get_outdated_packages,
            check_pip_conflicts,
            precheck_requirements,
            install_requirements,
            update_pip_packages,
            get_shell_statuses,
            configure_shell,
            analyze_codebase_imports,
            run_doctor,
            run_doctor_fix,
            get_venv_upgrade_info,
            install_pip_packages,
            get_cache_stats,
            purge_cache
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
