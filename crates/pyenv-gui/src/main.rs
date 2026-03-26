#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
// ./crates/pyenv-gui/src/main.rs
//! Tauri v2 backend for pyenv-native GUI.
//!
//! All long-running or blocking core operations are dispatched via
//! `tokio::task::spawn_blocking` so the GUI thread stays responsive.
//! Commands that would normally read stdin (uninstall confirmation,
//! self-update confirmation) use force/yes flags since the GUI provides
//! its own confirmation modals.

use pyenv_core::CommandExt;

#[tauri::command]
fn get_status() -> Result<String, String> {
    let ctx = pyenv_core::AppContext::from_system().map_err(|e| e.to_string())?;
    let report = pyenv_core::cmd_status(&ctx, true);
    if report.exit_code != 0 {
        return Err(report.stderr.join("\n"));
    }
    Ok(report.stdout.join("\n"))
}

#[tauri::command]
fn get_available_versions(
    family: Option<String>,
    pattern: Option<String>,
) -> Result<String, String> {
    let ctx = pyenv_core::AppContext::from_system().map_err(|e| e.to_string())?;
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
fn get_installed_versions() -> Result<String, String> {
    let ctx = pyenv_core::AppContext::from_system().map_err(|e| e.to_string())?;
    let names = pyenv_core::installed_version_names(&ctx).map_err(|e| e.to_string())?;
    serde_json::to_string(&names).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_managed_venvs() -> Result<String, String> {
    let ctx = pyenv_core::AppContext::from_system().map_err(|e| e.to_string())?;
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
async fn install_version(version: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = pyenv_core::AppContext::from_system().map_err(|e| e.to_string())?;
        let ver_clone = version.clone();
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

        // Auto-update pip after successful install
        let py_path = ctx.versions_dir().join(&ver_clone).join("python.exe");
        if py_path.exists() {
            let _ = std::process::Command::new(&py_path)
                .headless()
                .args(["-m", "pip", "install", "-U", "pip"])
                .output();
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
async fn uninstall_version(version: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = pyenv_core::AppContext::from_system().map_err(|e| e.to_string())?;
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
fn set_global(version: String) -> Result<String, String> {
    let ctx = pyenv_core::AppContext::from_system().map_err(|e| e.to_string())?;
    let report = pyenv_core::cmd_global(&ctx, &[version], false);
    if report.exit_code != 0 {
        return Err(report.stderr.join("\n"));
    }
    Ok(report.stdout.join("\n"))
}

#[tauri::command]
fn set_local(version: String, path: String) -> Result<String, String> {
    let ctx = pyenv_core::AppContext::from_system().map_err(|e| e.to_string())?;
    let target = std::path::Path::new(&path).join(".python-version");
    let report = pyenv_core::cmd_version_file_write(&ctx, &target, &[version], false);
    if report.exit_code != 0 {
        return Err(report.stderr.join("\n"));
    }
    Ok(report.stdout.join("\n"))
}

#[tauri::command]
fn get_config() -> Result<String, String> {
    let ctx = pyenv_core::AppContext::from_system().map_err(|e| e.to_string())?;
    serde_json::to_string(&ctx.config).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_config(key: String, value: String) -> Result<String, String> {
    let mut ctx = pyenv_core::AppContext::from_system().map_err(|e| e.to_string())?;
    let report = pyenv_core::cmd_config_set(&mut ctx, &key, &value);
    if report.exit_code != 0 {
        return Err(report.stderr.join("\n"));
    }
    Ok(report.stdout.join("\n"))
}

#[tauri::command]
fn create_venv(base_version: String, name: String) -> Result<String, String> {
    let ctx = pyenv_core::AppContext::from_system().map_err(|e| e.to_string())?;
    let report = pyenv_core::cmd_venv_create(&ctx, &base_version, &name, false, false);
    if report.exit_code != 0 {
        return Err(report.stderr.join("\n"));
    }
    Ok(report.stdout.join("\n"))
}

#[tauri::command]
fn delete_venv(spec: String) -> Result<String, String> {
    let ctx = pyenv_core::AppContext::from_system().map_err(|e| e.to_string())?;
    let report = pyenv_core::cmd_venv_delete(&ctx, &spec, true);
    if report.exit_code != 0 {
        return Err(report.stderr.join("\n"));
    }
    Ok(report.stdout.join("\n"))
}

/// Check for pyenv-native updates using the core self-update API (check-only mode).
/// Returns a human-readable status string.
#[tauri::command]
async fn check_for_updates() -> Result<String, String> {
    tokio::task::spawn_blocking(|| {
        let ctx = pyenv_core::AppContext::from_system().map_err(|e| e.to_string())?;
        let options = pyenv_core::SelfUpdateOptions {
            check: true,
            yes: false,
            force: false,
            github_repo: None,
            tag: None,
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
async fn perform_update() -> Result<String, String> {
    tokio::task::spawn_blocking(|| {
        let ctx = pyenv_core::AppContext::from_system().map_err(|e| e.to_string())?;
        let options = pyenv_core::SelfUpdateOptions {
            check: false,
            yes: true,
            force: false,
            github_repo: None,
            tag: None,
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

fn main() {
    tauri::Builder::default()
        .setup(|_app| Ok(()))
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
            install_local_pyenv
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
