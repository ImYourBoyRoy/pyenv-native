#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
// ./crates/pyenv-gui/src/main.rs
//! Tauri v2 backend for pyenv-native GUI.
//!
//! All long-running or blocking core operations are dispatched via
//! `tokio::task::spawn_blocking` so the GUI thread stays responsive.
//! Commands that would normally read stdin (uninstall confirmation,
//! self-update confirmation) use force/yes flags since the GUI provides
//! its own confirmation modals.

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
            open_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
