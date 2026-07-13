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

#[tauri::command]
fn get_shell_statuses(workspace_dir: Option<String>) -> Result<Vec<ShellStatus>, String> {
    let ctx = get_context_with_dir(workspace_dir)?;
    let mut statuses = Vec::new();
    let shims_str = ctx.shims_dir().to_string_lossy().to_string().to_lowercase();

    // Check if shims are currently present in PATH
    let path_val = std::env::var("PATH").unwrap_or_default().to_lowercase();
    let shims_in_path = path_val.contains(&shims_str);

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
    is_installed: bool,
    root: Option<String>,
    is_portable: bool,
}

#[tauri::command]
fn check_install_status() -> InstallStatus {
    // Check if pyenv is in the system PATH
    let pyenv_in_path = if cfg!(windows) {
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

    let ctx = pyenv_core::AppContext::from_system();

    // Check if we are in a portable bundle (pyenv executable next to us)
    let exe_path = std::env::current_exe().unwrap_or_default();
    let parent = exe_path.parent().unwrap_or(std::path::Path::new("."));

    let pyenv_name = if cfg!(windows) { "pyenv.exe" } else { "pyenv" };
    let local_pyenv = parent.join(pyenv_name);
    let is_portable = local_pyenv.exists();

    match ctx {
        Ok(c) => InstallStatus {
            is_installed: pyenv_in_path,
            root: Some(c.root.to_string_lossy().to_string()),
            is_portable,
        },
        Err(_) => InstallStatus {
            is_installed: pyenv_in_path,
            root: if is_portable {
                Some(parent.to_string_lossy().to_string())
            } else {
                None
            },
            is_portable,
        },
    }
}

#[tauri::command]
async fn install_local_pyenv() -> Result<String, String> {
    tokio::task::spawn_blocking(|| {
        let exe_path = std::env::current_exe().unwrap_or_default();
        let parent = exe_path.parent().unwrap_or(std::path::Path::new("."));

        #[cfg(target_os = "windows")]
        {
            let script_names = [
                "install-pyenv-native.ps1",
                "scripts/install-pyenv-native.ps1",
            ];
            let mut script_path = None;

            // Search up the tree for scripts/ (useful for dev builds in target/debug)
            let mut curr = parent.to_path_buf();
            for _ in 0..5 {
                for name in script_names {
                    let test = curr.join(name);
                    if test.exists() {
                        script_path = Some(test);
                        break;
                    }
                }
                if script_path.is_some() {
                    break;
                }
                if let Some(p) = curr.parent() {
                    curr = p.to_path_buf();
                } else {
                    break;
                }
            }

            let script = script_path
                .ok_or_else(|| "Installer script not found in bundle or workspace.".to_string())?;

            // Detect if we have bundled binaries near the EXE for offline install
            let pyenv_bin = parent.join("pyenv.exe");
            let mcp_bin = parent.join("pyenv-mcp.exe");

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
            let mut script_path = None;

            let mut curr = parent.to_path_buf();
            for _ in 0..5 {
                for name in script_names {
                    let test = curr.join(name);
                    if test.exists() {
                        script_path = Some(test);
                        break;
                    }
                }
                if script_path.is_some() {
                    break;
                }
                if let Some(p) = curr.parent() {
                    curr = p.to_path_buf();
                } else {
                    break;
                }
            }

            let script = script_path
                .ok_or_else(|| "Installer script not found in bundle or workspace.".to_string())?;

            // Offline install for POSIX (assuming script handles it or we pass env/args)
            let mut args = vec![script.to_string_lossy().to_string(), "--yes".to_string()];

            let pyenv_bin = parent.join("pyenv");
            if pyenv_bin.exists() {
                args.push("--source-path".to_string());
                args.push(pyenv_bin.to_string_lossy().to_string());
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
            install_pip_packages
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
