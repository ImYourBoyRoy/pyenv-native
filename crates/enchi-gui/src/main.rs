#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
// ./crates/enchi-gui/src/main.rs
//! Tauri v2 backend for Enchi GUI.
//!
//! All long-running or blocking core operations are dispatched via
//! `tokio::task::spawn_blocking` so the GUI thread stays responsive.
//! Commands that would normally read stdin (uninstall confirmation,
//! self-update confirmation) use force/yes flags since the GUI provides
//! its own confirmation modals.

mod desktop_integration;
mod smoke;

use enchi_core::PyenvCommandExt;

fn get_context_with_dir(workspace_dir: Option<String>) -> Result<enchi_core::AppContext, String> {
    let mut ctx = enchi_core::AppContext::from_system().map_err(|e| e.to_string())?;
    enchi_core::ensure_managed_path(&mut ctx);
    if let Some(dir_str) = workspace_dir
        && !dir_str.trim().is_empty()
    {
        let path = std::path::PathBuf::from(dir_str);
        if path.exists() && path.is_dir() {
            ctx.dir = path;
        }
    }
    Ok(ctx)
}

async fn run_blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| format!("Task panicked: {e}"))?
}

#[tauri::command]
async fn get_status(workspace_dir: Option<String>) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let report = enchi_core::cmd_status(&ctx, true);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

#[tauri::command]
async fn get_available_versions(
    workspace_dir: Option<String>,
    family: Option<String>,
    pattern: Option<String>,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let report = enchi_core::cmd_available(&ctx, family, pattern, false, true);
        if report.exit_code != 0 {
            let errs = report.stderr.join("\n");
            if errs.is_empty() {
                return Err("Failed to fetch available versions.".to_string());
            }
            return Err(errs);
        }
        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

#[tauri::command]
async fn get_installed_versions(workspace_dir: Option<String>) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let names = enchi_core::installed_version_names(&ctx).map_err(|e| e.to_string())?;
        serde_json::to_string(&names).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

#[tauri::command]
async fn get_managed_venvs(workspace_dir: Option<String>) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let report = enchi_core::cmd_venv_list(&ctx, false, true);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
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
        let options = enchi_core::InstallCommandOptions {
            list: false,
            force: false,
            dry_run: false,
            json: false,
            known: false,
            family: None,
            versions: vec![version],
        };
        let report = enchi_core::cmd_install(&ctx, &options);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }

        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

/// Forward a progress line to the UI as an `operation-progress` event keyed by operation id.
fn emit_progress(app: &tauri::AppHandle, operation_id: &str, step: &str) {
    use tauri::Emitter;
    let _ = app.emit(
        "operation-progress",
        serde_json::json!({ "id": operation_id, "step": step }),
    );
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| format!("serialize failed: {e}"))
}

/// Newest patch available for each installed CPython (no changes made).
#[tauri::command]
async fn plan_runtime_upgrades(workspace_dir: Option<String>) -> Result<String, String> {
    run_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let items = enchi_core::plan_upgrades(&ctx, &[]).map_err(|e| e.to_string())?;
        to_json(&items)
    })
    .await
}

/// Install newer patches, migrate their venvs, and re-pin. Progress arrives as events.
#[tauri::command]
async fn run_runtime_upgrades(
    app: tauri::AppHandle,
    workspace_dir: Option<String>,
    operation_id: String,
    targets: Vec<String>,
    remove_old: bool,
) -> Result<String, String> {
    run_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let options = enchi_core::UpgradeOptions {
            targets,
            remove_old,
            ..Default::default()
        };
        let report = enchi_core::run_upgrades(&ctx, &options, &mut |step| {
            emit_progress(&app, &operation_id, step);
        })
        .map_err(|e| e.to_string())?;
        to_json(&report)
    })
    .await
}

/// Which environments have outdated packages (no changes made).
#[tauri::command]
async fn plan_bulk_package_updates(workspace_dir: Option<String>) -> Result<String, String> {
    run_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let options = enchi_core::BulkUpdateOptions {
            dry_run: true,
            ..Default::default()
        };
        let report = enchi_core::update_all_packages(&ctx, &options, &mut |_| {})
            .map_err(|e| e.to_string())?;
        to_json(&report)
    })
    .await
}

/// Update outdated packages across every runtime and venv (or `only`). Progress arrives as events.
#[tauri::command]
async fn run_bulk_package_updates(
    app: tauri::AppHandle,
    workspace_dir: Option<String>,
    operation_id: String,
    only: Vec<String>,
) -> Result<String, String> {
    run_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let options = enchi_core::BulkUpdateOptions {
            only,
            ..Default::default()
        };
        let report = enchi_core::update_all_packages(&ctx, &options, &mut |step| {
            emit_progress(&app, &operation_id, step);
        })
        .map_err(|e| e.to_string())?;
        to_json(&report)
    })
    .await
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
        let report = enchi_core::cmd_uninstall(&ctx, &[version], true);
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
async fn set_global(workspace_dir: Option<String>, version: String) -> Result<String, String> {
    run_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let report = enchi_core::cmd_global(&ctx, &[version], false);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        let rehash = enchi_core::cmd_rehash(&ctx);
        if rehash.exit_code != 0 {
            return Err(rehash.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
}

#[tauri::command]
async fn set_local(version: String, path: String) -> Result<String, String> {
    run_blocking(move || {
        let ctx = get_context_with_dir(Some(path.clone()))?;
        let target = std::path::Path::new(&path).join(".python-version");
        let report = enchi_core::cmd_version_file_write(&ctx, &target, &[version], false);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
}

#[tauri::command]
async fn get_config(workspace_dir: Option<String>) -> Result<String, String> {
    run_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        serde_json::to_string(&ctx.config).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
async fn set_config(
    workspace_dir: Option<String>,
    key: String,
    value: String,
) -> Result<String, String> {
    run_blocking(move || {
        let mut ctx = get_context_with_dir(workspace_dir)?;
        let report = enchi_core::cmd_config_set(&mut ctx, &key, &value);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
}

#[tauri::command]
async fn create_venv(
    workspace_dir: Option<String>,
    base_version: String,
    name: String,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let report = enchi_core::cmd_venv_create(&ctx, &base_version, &name, false, false);
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
        let report = enchi_core::cmd_venv_delete(&ctx, &spec, true);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

#[tauri::command]
async fn upgrade_venv(
    workspace_dir: Option<String>,
    spec: String,
    new_runtime: String,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let report =
            enchi_core::cmd_venv_upgrade_noninteractive(&ctx, &spec, &new_runtime, true, false);
        let output = [report.stdout.join("\n"), report.stderr.join("\n")]
            .into_iter()
            .filter(|block| !block.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if report.exit_code != 0 {
            return Err(if output.is_empty() {
                "enchi: venv upgrade failed".to_string()
            } else {
                output
            });
        }
        Ok(output)
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

/// The sentence, hints, and warnings the core prints after a managed-venv rename.
#[derive(serde::Serialize, Debug, PartialEq, Eq)]
struct VenvRenameOutcomeGui {
    old_spec: String,
    new_spec: String,
    message: String,
    hints: Vec<String>,
    warnings: Vec<String>,
}

/// Split the core's rename report into the success sentence, `Hint:` lines, and `Warning:` lines.
fn split_rename_lines(lines: &[String]) -> (String, Vec<String>, Vec<String>) {
    let mut message = Vec::new();
    let mut hints = Vec::new();
    let mut warnings = Vec::new();
    for line in lines
        .iter()
        .map(|line| line.trim())
        .filter(|l| !l.is_empty())
    {
        if let Some(rest) = line.strip_prefix("Warning:") {
            warnings.push(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("Hint:") {
            hints.push(rest.trim().to_string());
        } else {
            message.push(line.to_string());
        }
    }
    (message.join("\n"), hints, warnings)
}

/// What renaming a managed venv would do (no changes made). `Err` carries the core's reason,
/// for example an invalid name or a collision with another environment.
fn plan_venv_rename_sync(
    ctx: &enchi_core::AppContext,
    spec: &str,
    new_name: &str,
) -> Result<String, String> {
    // The core validates the trimmed name but would build the folder from the raw one.
    let new_name = new_name.trim();
    let report = enchi_core::plan_venv_rename(ctx, spec, new_name);
    if report.exit_code != 0 {
        return Err(report.stderr.join("\n"));
    }
    Ok(report
        .stdout
        .iter()
        .filter(|line| !line.starts_with("Dry run"))
        .cloned()
        .collect::<Vec<_>>()
        .join("\n"))
}

fn rename_venv_sync(
    ctx: &enchi_core::AppContext,
    spec: &str,
    new_name: &str,
) -> Result<VenvRenameOutcomeGui, String> {
    let new_name = new_name.trim();
    let report = enchi_core::cmd_venv_rename(ctx, spec, new_name);
    if report.exit_code != 0 {
        return Err(report.stderr.join("\n"));
    }
    let (message, hints, warnings) = split_rename_lines(&report.stdout);
    let new_spec = enchi_core::list_managed_venvs(ctx)
        .ok()
        .and_then(|venvs| venvs.into_iter().find(|venv| venv.name == new_name))
        .map(|venv| venv.spec)
        .unwrap_or_else(|| new_name.to_string());
    Ok(VenvRenameOutcomeGui {
        old_spec: spec.to_string(),
        new_spec,
        message,
        hints,
        warnings,
    })
}

/// Preview a managed-venv rename (core `plan_venv_rename`); nothing is changed.
#[tauri::command]
async fn plan_venv_rename(
    workspace_dir: Option<String>,
    spec: String,
    new_name: String,
) -> Result<String, String> {
    run_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        plan_venv_rename_sync(&ctx, &spec, &new_name)
    })
    .await
}

/// Rename a managed venv. The UI previews first and tracks the call in the Activity tray.
#[tauri::command]
async fn rename_venv(
    workspace_dir: Option<String>,
    spec: String,
    new_name: String,
) -> Result<VenvRenameOutcomeGui, String> {
    run_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        rename_venv_sync(&ctx, &spec, &new_name)
    })
    .await
}

/// Answer to "where does this command come from?".
#[derive(serde::Serialize, Debug, PartialEq, Eq)]
struct WhichResultGui {
    command: String,
    found: bool,
    /// Resolved executable when `found`.
    path: Option<String>,
    /// The core's explanation when the command is not found (empty otherwise).
    message: String,
    /// Installed runtimes that contain the command (the `enchi whence` answer).
    also_in: Vec<String>,
}

/// Commands are bare names (`python`, `pip3.12`, `pytest`), never paths.
fn validate_lookup_command(raw: &str) -> Result<String, String> {
    let command = raw.trim();
    if command.is_empty() {
        return Err("Enter a command name such as python or pip.".to_string());
    }
    if command.starts_with('-')
        || command
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err("Enter just the command name, without a path or options.".to_string());
    }
    Ok(command.to_string())
}

fn which_command_sync(
    ctx: &enchi_core::AppContext,
    command: &str,
) -> Result<WhichResultGui, String> {
    let command = validate_lookup_command(command)?;
    let report = enchi_core::cmd_which(ctx, &command, false, true);
    let also_in = {
        let whence = enchi_core::cmd_whence(ctx, &command, false);
        if whence.exit_code == 0 {
            whence.stdout
        } else {
            Vec::new()
        }
    };
    if report.exit_code == 0 {
        let path = report.stdout.join("\n").trim().to_string();
        return Ok(WhichResultGui {
            command,
            found: true,
            path: Some(path),
            message: String::new(),
            also_in,
        });
    }
    let message = report
        .stderr
        .iter()
        .map(|line| line.trim_end())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    Ok(WhichResultGui {
        command,
        found: false,
        path: None,
        message,
        also_in,
    })
}

/// Resolve a command the way shims would for the project folder (`enchi which`).
#[tauri::command]
async fn which_command(
    workspace_dir: Option<String>,
    command: String,
) -> Result<WhichResultGui, String> {
    run_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        which_command_sync(&ctx, &command)
    })
    .await
}

#[derive(serde::Serialize, Debug, PartialEq, Eq)]
struct RuntimePrefixGui {
    version: String,
    path: String,
}

/// Install location of each managed runtime (core `prefix`); `system` and failures are skipped.
fn runtime_prefixes_sync(
    ctx: &enchi_core::AppContext,
    versions: &[String],
) -> Vec<RuntimePrefixGui> {
    versions
        .iter()
        .filter(|version| version.as_str() != "system")
        .filter_map(|version| {
            let report = enchi_core::cmd_prefix(ctx, std::slice::from_ref(version));
            if report.exit_code != 0 {
                return None;
            }
            let path = report.stdout.join("\n").trim().to_string();
            (!path.is_empty()).then(|| RuntimePrefixGui {
                version: version.clone(),
                path,
            })
        })
        .collect()
}

#[tauri::command]
async fn get_runtime_prefixes(
    workspace_dir: Option<String>,
    versions: Vec<String>,
) -> Result<Vec<RuntimePrefixGui>, String> {
    run_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        Ok(runtime_prefixes_sync(&ctx, &versions))
    })
    .await
}

/// Folder of an installed runtime (`3.12.4`) or managed venv (`3.12.4/envs/demo`). Only things
/// Enchi manages resolve, so the UI cannot be pointed at an arbitrary path.
fn resolve_managed_folder(
    ctx: &enchi_core::AppContext,
    target: &str,
) -> Result<std::path::PathBuf, String> {
    let wanted = target.trim().replace('\\', "/");
    let wanted = wanted.strip_prefix("venv:").unwrap_or(&wanted);
    if wanted.is_empty() || wanted == "system" {
        return Err("Nothing to open for this entry.".to_string());
    }
    if wanted.contains("/envs/") {
        let venvs = enchi_core::list_managed_venvs(ctx).map_err(|e| e.to_string())?;
        return venvs
            .into_iter()
            .find(|venv| venv.spec.replace('\\', "/") == wanted)
            .map(|venv| venv.path)
            .ok_or_else(|| format!("enchi: managed venv `{wanted}` not found"));
    }
    let installed = enchi_core::installed_version_names(ctx).map_err(|e| e.to_string())?;
    if !installed.iter().any(|name| name == wanted) {
        return Err(format!("enchi: version `{wanted}` not installed"));
    }
    Ok(enchi_core::installed_version_dir(ctx, wanted))
}

/// Show a runtime's or managed venv's folder in the system file manager.
#[tauri::command]
async fn open_managed_folder(workspace_dir: Option<String>, target: String) -> Result<(), String> {
    run_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let folder = resolve_managed_folder(&ctx, &target)?;
        if !folder.is_dir() {
            return Err(format!("{} is not a folder.", folder.display()));
        }
        opener::open(&folder).map_err(|error| format!("failed to open folder: {error}"))
    })
    .await
}

/// Check for Enchi updates using the core self-update API (check-only mode).
#[tauri::command]
async fn check_for_updates(
    workspace_dir: Option<String>,
) -> Result<enchi_core::SelfUpdateCheck, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let options = enchi_core::SelfUpdateOptions {
            check: true,
            yes: false,
            force: false,
            github_repo: None,
            tag: None,
            restart_gui: false,
        };
        enchi_core::inspect_self_update(&ctx, &options)
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

/// Perform the actual self-update with `yes=true` to skip interactive confirmation.
#[tauri::command]
async fn perform_update(workspace_dir: Option<String>) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let options = enchi_core::SelfUpdateOptions {
            check: false,
            yes: true,
            force: false,
            github_repo: None,
            tag: None,
            restart_gui: true,
        };
        let report = enchi_core::cmd_self_update(&ctx, &options);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
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
        let py_path = enchi_core::resolve_interpreter_path(&ctx, &target)?;

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
        let report = enchi_core::cmd_pip_list(&ctx, &target, true);
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
        let report = enchi_core::cmd_pip_outdated(&ctx, &target, true);
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
        let report = enchi_core::cmd_pip_check(&ctx, &target, true);
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
        let report = enchi_core::cmd_pip_precheck_requirements(&ctx, &target, &path_or_url);
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
        let report = enchi_core::cmd_pip_install(&ctx, &target, &path_or_url);
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
        let report = enchi_core::cmd_pip_update(&ctx, &target, &packages, is_all);
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
        let report = enchi_core::cmd_pip_analyze_imports(&ctx, &target, &dir_path);
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

fn is_safe_http_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        return false;
    }
    !url.chars()
        .any(|c| c.is_control() || matches!(c, '"' | '\'' | '|' | '&' | '<' | '>' | '`'))
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    let url = url.trim();
    if !is_safe_http_url(url) {
        return Err("only plain http(s) URLs can be opened".to_string());
    }
    opener::open(url).map_err(|error| format!("failed to open URL: {error}"))
}

#[cfg(windows)]
fn open_ms_settings(uri: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const ALLOWED: &[&str] = &["ms-settings:advanced-apps", "ms-settings:appsfeatures"];
    if !ALLOWED.contains(&uri) {
        return Err("unsupported Windows Settings page".to_string());
    }
    let status = std::process::Command::new("cmd")
        .args(["/c", "start", "", uri])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|error| format!("failed to open Windows Settings: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Windows Settings returned {status}"))
    }
}

#[tauri::command]
fn open_app_execution_aliases() -> Result<(), String> {
    #[cfg(windows)]
    {
        open_ms_settings("ms-settings:advanced-apps")
            .or_else(|_| open_ms_settings("ms-settings:appsfeatures"))
    }
    #[cfg(not(windows))]
    {
        Err("App execution aliases are only available in Windows Settings".to_string())
    }
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
        enchi_core::lookup_i18n("gui-shell-path-active")
    } else if is_configured {
        enchi_core::lookup_i18n("gui-shell-restart-terminal")
    } else {
        enchi_core::lookup_i18n("gui-shell-shims-missing")
    }
}

fn known_shell_profile_paths(home: &std::path::Path) -> Vec<std::path::PathBuf> {
    enchi_core::known_profile_paths(home)
        .into_iter()
        .map(|(_, path)| path)
        .collect()
}

fn profile_paths_equal(left: &std::path::Path, right: &std::path::Path) -> bool {
    let left_s = left.to_string_lossy().replace('\\', "/");
    let right_s = right.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        left_s.eq_ignore_ascii_case(&right_s)
    } else {
        left_s == right_s
    }
}

fn profile_path_is_allowed(home: &std::path::Path, requested: &std::path::Path) -> bool {
    if requested
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return false;
    }
    known_shell_profile_paths(home)
        .iter()
        .any(|allowed| profile_paths_equal(allowed, requested))
}

#[tauri::command]
async fn get_shell_statuses(workspace_dir: Option<String>) -> Result<Vec<ShellStatus>, String> {
    run_blocking(move || get_shell_statuses_sync(workspace_dir)).await
}

fn get_shell_statuses_sync(workspace_dir: Option<String>) -> Result<Vec<ShellStatus>, String> {
    let ctx = get_context_with_dir(workspace_dir)?;
    let mut statuses = Vec::new();
    let shims_dir = ctx.shims_dir();
    let path_val = enchi_core::launch_path().to_string_lossy().into_owned();
    let shims_in_path = path_has_entry(&path_val, &shims_dir);

    // Resolve home directory
    let home = get_home_dir()?;

    // 1. PowerShell 7 Profile
    let pwsh_profile = enchi_core::powershell_profile_path(&home, true);
    let pwsh_configured = pwsh_profile.is_file() && {
        let content = std::fs::read_to_string(&pwsh_profile).unwrap_or_default();
        enchi_core::has_pyenv_init(&content)
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
        let win_ps_profile = enchi_core::powershell_profile_path(&home, false);
        let win_ps_configured = win_ps_profile.is_file() && {
            let content = std::fs::read_to_string(&win_ps_profile).unwrap_or_default();
            enchi_core::has_pyenv_init(&content)
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
        enchi_core::has_pyenv_init(&content)
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
        enchi_core::has_pyenv_init(&content)
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
        enchi_core::has_pyenv_init(&content)
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
async fn configure_shell(shell_name: String, profile_path: String) -> Result<(), String> {
    run_blocking(move || configure_shell_impl(&shell_name, &profile_path)).await
}

fn configure_shell_impl(shell_name: &str, profile_path: &str) -> Result<(), String> {
    let home = get_home_dir()?;
    let path = std::path::PathBuf::from(profile_path);
    if !profile_path_is_allowed(&home, &path) {
        return Err(
            "Refusing to write a shell profile outside the known user profile set.".to_string(),
        );
    }
    let kind = if shell_name.contains("PowerShell") {
        enchi_core::ShellKind::Pwsh
    } else if shell_name.contains("Zsh") {
        enchi_core::ShellKind::Zsh
    } else if shell_name.contains("Bash") {
        enchi_core::ShellKind::Bash
    } else if shell_name.contains("Fish") {
        enchi_core::ShellKind::Fish
    } else {
        return Err(format!(
            "Unsupported shell for automatic configuration: {shell_name}"
        ));
    };

    // One shared, delimited block (also removed cleanly by `enchi self-uninstall`).
    let ctx = enchi_core::AppContext::from_system().map_err(|e| e.to_string())?;
    let layout = enchi_core::InstallLayout::new(ctx.root.clone());
    enchi_core::apply_profile_block(kind, &path, &layout.cli_exe(), &layout.bin_dir())
        .map(|_| ())
        .map_err(|e| format!("Failed to write profile: {e}"))
}

#[derive(serde::Serialize)]
struct InstallStatus {
    /// Managed binaries exist under PYENV_ROOT/bin (or a portable sibling layout).
    is_installed: bool,
    root: Option<String>,
    is_portable: bool,
    /// `enchi` resolves on the GUI process PATH (often false for desktop launches).
    cli_on_path: bool,
    /// At least one shell profile already contains `enchi init`.
    profiles_configured: bool,
    /// Shims appear on this GUI process PATH (desktop launches usually lack them even when setup is done).
    shims_on_gui_path: bool,
    /// True only when binaries exist but no shell profile has been configured yet.
    /// Desktop apps not inheriting shell PATH must NOT trigger this.
    needs_shell_setup: bool,
    platform: String,
}

fn needs_shell_setup(binaries_installed: bool, profiles_configured: bool) -> bool {
    binaries_installed && !profiles_configured
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
    [enchi_core::brand::CLI, enchi_core::brand::LEGACY_CLI]
        .iter()
        .any(|name| bin.join(enchi_core::brand::exe_name(name)).is_file())
}

/// Whether `name` (this tool's command, current or pre-Enchi) resolves on the launch PATH.
fn command_on_launch_path(name: &str) -> bool {
    let program = if cfg!(windows) { "where.exe" } else { "which" };
    let mut command = std::process::Command::new(program);
    #[cfg(windows)]
    command.headless();
    command
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn any_shell_profile_configured(home: &std::path::Path) -> bool {
    let candidates = [
        home.join(".bashrc"),
        home.join(".bash_profile"),
        home.join(".profile"),
        home.join(".zshrc"),
        home.join(".zprofile"),
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
                .map(|content| {
                    let lower = content.to_ascii_lowercase();
                    lower.contains("enchi init")
                        || lower.contains("pyenv init")
                        || lower.contains("pyenv-native")
                        || lower.contains("enchi_root")
                        || (lower.contains("pyenv_root") && lower.contains("shims"))
                })
                .unwrap_or(false)
    })
}

fn finalize_managed_install(ctx: &enchi_core::AppContext) -> Result<String, String> {
    let rehash = enchi_core::cmd_rehash(ctx);
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
                .map(|c| enchi_core::has_pyenv_init(&c))
                .unwrap_or(false);
        if already {
            continue;
        }
        if let Ok(()) = configure_shell_impl(name, &profile.to_string_lossy()) {
            configured.push(name.to_string());
        }
    }

    let mut lines = vec![
        format!("Core binaries are ready under {}.", ctx.root.display()),
        "Shim directory refreshed.".to_string(),
    ];
    if configured.is_empty() {
        lines.push(
            "Shell profiles already include enchi init. Open a new terminal so shims appear on PATH."
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
async fn check_install_status() -> Result<InstallStatus, String> {
    run_blocking(|| Ok(check_install_status_sync())).await
}

fn check_install_status_sync() -> InstallStatus {
    let platform = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
    .to_string();

    let cli_on_path = command_on_launch_path(enchi_core::brand::CLI)
        || command_on_launch_path(enchi_core::brand::LEGACY_CLI);

    let exe_path = std::env::current_exe().unwrap_or_default();
    let parent = exe_path.parent().unwrap_or(std::path::Path::new("."));
    let is_portable = [enchi_core::brand::CLI, enchi_core::brand::LEGACY_CLI]
        .iter()
        .any(|name| parent.join(enchi_core::brand::exe_name(name)).exists());

    let ctx = enchi_core::AppContext::from_system();
    let home = get_home_dir().ok();

    match ctx {
        Ok(c) => {
            let binaries = managed_binaries_present(&c.root) || is_portable;
            let shims_dir = c.shims_dir();
            let path_val = std::env::var("PATH").unwrap_or_default();
            let shims_on_gui_path = path_has_entry(&path_val, &shims_dir);
            let profiles_configured = home
                .as_ref()
                .map(|h| any_shell_profile_configured(h))
                .unwrap_or(false);
            InstallStatus {
                is_installed: binaries,
                root: Some(c.root.to_string_lossy().to_string()),
                is_portable,
                cli_on_path,
                profiles_configured,
                shims_on_gui_path,
                // Shell setup is about terminal profiles, not this GUI process PATH.
                // The app prepends bin/shims onto its own PATH at startup.
                needs_shell_setup: needs_shell_setup(binaries, profiles_configured),
                platform,
            }
        }
        Err(_) => {
            let profiles_configured = home
                .as_ref()
                .map(|h| any_shell_profile_configured(h))
                .unwrap_or(false);
            let binaries = is_portable || cli_on_path;
            InstallStatus {
                is_installed: binaries,
                root: if is_portable {
                    Some(parent.to_string_lossy().to_string())
                } else {
                    None
                },
                is_portable,
                cli_on_path,
                profiles_configured,
                shims_on_gui_path: false,
                needs_shell_setup: needs_shell_setup(binaries, profiles_configured),
                platform,
            }
        }
    }
}

#[tauri::command]
async fn install_local_pyenv() -> Result<String, String> {
    let result = tokio::task::spawn_blocking(|| {
        let ctx = enchi_core::AppContext::from_system().map_err(|e| e.to_string())?;

        // Prefer finalizing an already-managed install (typical after network install).
        if managed_binaries_present(&ctx.root) {
            return finalize_managed_install(&ctx);
        }

        // Otherwise install the binaries that ship next to this GUI executable.
        let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
        let source_dir = exe_path
            .parent()
            .ok_or_else(|| "Could not locate the GUI executable directory.".to_string())?
            .to_path_buf();
        let mut options = enchi_core::SelfInstallOptions::new(ctx.root.clone());
        options.source_dir = Some(source_dir);
        options.yes = true;
        // A desktop app should not install other software behind the user's back.
        options.install_powershell7 = enchi_core::PowerShell7Mode::Never;

        let report = enchi_core::cmd_self_install(&options);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        Ok(report.stdout.join("\n"))
    })
    .await
    .map_err(|e| e.to_string())?;
    if result.is_ok()
        && let Ok(mut ctx) = enchi_core::AppContext::from_system()
    {
        enchi_core::ensure_managed_path(&mut ctx);
    }
    result
}

#[derive(serde::Serialize)]
struct DoctorCheckGui {
    name: String,
    status: String,
    detail: String,
}

#[derive(serde::Serialize)]
struct DoctorFixGui {
    key: String,
    description: String,
    command_hint: Option<String>,
}

#[derive(serde::Serialize)]
struct DoctorFixOutcomeGui {
    applied: Vec<String>,
    manual: Vec<DoctorFixGui>,
}

#[tauri::command]
async fn run_doctor(workspace_dir: Option<String>) -> Result<Vec<DoctorCheckGui>, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let checks = enchi_core::collect_checks_with_options(
            &ctx,
            enchi_core::DoctorOptions {
                desktop_session: true,
                profiles_configured: None,
            },
        );
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
async fn run_doctor_fix(workspace_dir: Option<String>) -> Result<DoctorFixOutcomeGui, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let outcome = enchi_core::apply_doctor_fixes_with_options(
            &ctx,
            enchi_core::DoctorOptions {
                desktop_session: true,
                profiles_configured: None,
            },
        )
        .map_err(|e| e.to_string())?;
        Ok(DoctorFixOutcomeGui {
            applied: outcome.applied,
            manual: outcome
                .manual
                .into_iter()
                .map(|item| DoctorFixGui {
                    key: item.key,
                    description: item.description,
                    command_hint: item.command_hint,
                })
                .collect(),
        })
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

#[derive(serde::Serialize)]
struct PlatformFactGui {
    key: String,
    label: String,
    value: String,
}

#[derive(serde::Serialize)]
struct PlatformIntelligenceGui {
    os: String,
    arch: String,
    os_pretty_name: String,
    install_strategy: String,
    ready_to_install: bool,
    verdict: String,
    summary: String,
    facts: Vec<PlatformFactGui>,
    blocking_issues: Vec<String>,
    warnings: Vec<String>,
}

#[tauri::command]
async fn get_platform_intelligence(
    workspace_dir: Option<String>,
) -> Result<PlatformIntelligenceGui, String> {
    tokio::task::spawn_blocking(move || {
        let ctx = get_context_with_dir(workspace_dir)?;
        let intel = enchi_core::build_platform_intelligence(&ctx);
        Ok(PlatformIntelligenceGui {
            os: intel.os,
            arch: intel.arch,
            os_pretty_name: intel.os_pretty_name,
            install_strategy: intel.install_strategy,
            ready_to_install: intel.ready_to_install,
            verdict: match intel.verdict {
                enchi_core::PreflightVerdict::Ready => "ready".to_string(),
                enchi_core::PreflightVerdict::NeedsAttention => "needs-attention".to_string(),
                enchi_core::PreflightVerdict::Blocked => "blocked".to_string(),
            },
            summary: intel.summary,
            facts: intel
                .facts
                .into_iter()
                .map(|fact| PlatformFactGui {
                    key: fact.key,
                    label: fact.label,
                    value: fact.value,
                })
                .collect(),
            blocking_issues: intel.blocking_issues,
            warnings: intel.warnings,
        })
    })
    .await
    .map_err(|e| format!("Task panicked: {e}"))?
}

#[derive(serde::Serialize)]
struct CacheEntryGui {
    id: String,
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

fn cache_entry(id: &str, name: &str, path: std::path::PathBuf) -> CacheEntryGui {
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
        id: id.to_string(),
        name: name.to_string(),
        path: path.to_string_lossy().to_string(),
        bytes,
        exists,
    }
}

#[tauri::command]
async fn get_cache_stats(workspace_dir: Option<String>) -> Result<CacheStatsGui, String> {
    run_blocking(move || get_cache_stats_sync(workspace_dir)).await
}

fn get_cache_stats_sync(workspace_dir: Option<String>) -> Result<CacheStatsGui, String> {
    let ctx = get_context_with_dir(workspace_dir)?;
    let cache_root = ctx.cache_dir();
    let mut entries = vec![
        cache_entry(
            "packages",
            "Python downloads / packages",
            cache_root.join("packages"),
        ),
        cache_entry(
            "python-build",
            "python-build cache",
            cache_root.join("python-build"),
        ),
        cache_entry("metadata", "Metadata cache", cache_root.join("metadata")),
    ];

    // Best-effort pip cache via `pip cache dir` for the active global interpreter.
    if let Ok(names) = enchi_core::installed_version_names(&ctx)
        && let Some(version) = names.first()
        && let Ok(py) = enchi_core::resolve_interpreter_path(&ctx, version)
        && let Ok(output) = std::process::Command::new(&py)
            .headless()
            .args(["-m", "pip", "cache", "dir"])
            .output()
        && output.status.success()
    {
        let pip_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !pip_dir.is_empty() {
            entries.push(cache_entry(
                "pip",
                "Pip cache (active runtime)",
                std::path::PathBuf::from(pip_dir),
            ));
        }
    }

    let total_bytes = entries.iter().map(|e| e.bytes).sum();
    entries.insert(0, cache_entry("all", "All Enchi cache", cache_root.clone()));
    Ok(CacheStatsGui {
        total_bytes,
        entries,
    })
}

#[tauri::command]
async fn purge_cache(workspace_dir: Option<String>, target: String) -> Result<String, String> {
    run_blocking(move || purge_cache_sync(workspace_dir, target)).await
}

fn purge_cache_sync(workspace_dir: Option<String>, target: String) -> Result<String, String> {
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
        return Err("Refusing to purge a path outside the Enchi cache root.".to_string());
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

#[tauri::command]
async fn install_powershell_7() -> Result<String, String> {
    run_blocking(|| enchi_core::install_powershell_7().map_err(|error| error.to_string())).await
}

#[tauri::command]
async fn i18n_bundle() -> Result<enchi_core::LocaleBundle, String> {
    run_blocking(|| {
        if let Ok(ctx) = enchi_core::AppContext::from_system() {
            enchi_core::apply_i18n(&ctx.config);
        }
        Ok(enchi_core::locale_bundle())
    })
    .await
}

#[tauri::command]
async fn i18n_format(
    id: String,
    args: std::collections::HashMap<String, String>,
) -> Result<String, String> {
    Ok(enchi_core::format_message(&id, &args))
}

#[tauri::command]
async fn set_ui_language(tag: String) -> Result<enchi_core::LocaleBundle, String> {
    run_blocking(move || {
        let mut ctx = enchi_core::AppContext::from_system().map_err(|e| e.to_string())?;
        let report = enchi_core::cmd_config_set(&mut ctx, "ui.language", &tag);
        if report.exit_code != 0 {
            return Err(report.stderr.join("\n"));
        }
        enchi_core::apply_i18n(&ctx.config);
        Ok(enchi_core::locale_bundle())
    })
    .await
}

fn main() {
    enchi_core::init_i18n();
    if let Ok(mut ctx) = enchi_core::AppContext::from_system() {
        enchi_core::apply_i18n(&ctx.config);
        enchi_core::ensure_managed_path(&mut ctx);
    }

    #[cfg(target_os = "linux")]
    desktop_integration::prepare_linux_runtime();

    tauri::Builder::default()
        .setup(|app| {
            smoke::start(app);
            desktop_integration::prepare_app(app)
        })
        .invoke_handler(tauri::generate_handler![
            smoke::ui_report,
            plan_runtime_upgrades,
            run_runtime_upgrades,
            plan_bulk_package_updates,
            run_bulk_package_updates,
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
            upgrade_venv,
            plan_venv_rename,
            rename_venv,
            which_command,
            get_runtime_prefixes,
            open_managed_folder,
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
            open_app_execution_aliases,
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
            install_powershell_7,
            i18n_bundle,
            i18n_format,
            set_ui_language,
            analyze_codebase_imports,
            run_doctor,
            run_doctor_fix,
            get_platform_intelligence,
            install_pip_packages,
            get_cache_stats,
            purge_cache
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::is_safe_http_url;

    #[test]
    fn accepts_plain_https_urls() {
        assert!(is_safe_http_url(
            "https://github.com/imyourboyroy/pyenv-native"
        ));
        assert!(is_safe_http_url("http://127.0.0.1:8080/docs"));
    }

    #[test]
    fn rejects_non_http_and_unsafe_characters() {
        assert!(!is_safe_http_url("javascript:alert(1)"));
        assert!(!is_safe_http_url("file:///etc/passwd"));
        assert!(!is_safe_http_url("https://example.com/\"onclick"));
        assert!(!is_safe_http_url("https://example.com/path?q=1&x=2"));
        assert!(!is_safe_http_url("https://example.com/\nnext"));
    }

    /// A throwaway `ENCHI_ROOT` with fake runtimes `3.12.0` (global) and `3.13.1`, each holding a
    /// fake `python`, plus managed venvs `demo` and `other` on 3.12.0.
    fn fake_context() -> (tempfile::TempDir, enchi_core::AppContext) {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = temp.path().join("root");
        let work = temp.path().join("work");
        std::fs::create_dir_all(&work).expect("work dir");
        for version in ["3.12.0", "3.13.1"] {
            let prefix = root.join("versions").join(version);
            let bin = if cfg!(windows) {
                prefix.clone()
            } else {
                prefix.join("bin")
            };
            std::fs::create_dir_all(&bin).expect("runtime dir");
            std::fs::write(
                bin.join(enchi_core::brand::exe_name("python")),
                "fake python",
            )
            .expect("fake python");
        }
        for name in ["demo", "other"] {
            std::fs::create_dir_all(root.join("venvs").join("3.12.0").join(name))
                .expect("venv dir");
        }
        std::fs::write(root.join("version"), "3.12.0\n").expect("global version");
        let ctx = enchi_core::AppContext {
            root,
            dir: work,
            exe_path: std::path::PathBuf::from("enchi"),
            env_version: None,
            env_shell: None,
            path_env: None,
            path_ext: None,
            config: enchi_core::AppConfig::default(),
        };
        (temp, ctx)
    }

    #[test]
    fn rename_report_is_split_into_message_hints_and_warnings() {
        let lines = [
            "Renamed managed venv 3.12.0/envs/a -> 3.12.0/envs/b".to_string(),
            "Hint: update any `.python-version` files.".to_string(),
            "  Warning: entry points keep the old path.".to_string(),
            String::new(),
        ];
        let (message, hints, warnings) = super::split_rename_lines(&lines);
        assert_eq!(
            message,
            "Renamed managed venv 3.12.0/envs/a -> 3.12.0/envs/b"
        );
        assert_eq!(hints, ["update any `.python-version` files."]);
        assert_eq!(warnings, ["entry points keep the old path."]);
    }

    #[test]
    fn rename_preview_reports_the_move_or_the_cores_reason() {
        let (_temp, ctx) = fake_context();
        let plan = super::plan_venv_rename_sync(&ctx, "3.12.0/envs/demo", "fresh").expect("plan");
        assert!(plan.contains("Would rename managed venv"), "{plan}");
        assert!(plan.contains("3.12.0/envs/fresh"), "{plan}");
        assert!(!plan.contains("Dry run"), "{plan}");

        let collision = super::plan_venv_rename_sync(&ctx, "3.12.0/envs/demo", "other")
            .expect_err("name already used");
        assert!(collision.contains("other"), "{collision}");

        let invalid = super::plan_venv_rename_sync(&ctx, "3.12.0/envs/demo", "bad name!")
            .expect_err("invalid characters");
        assert!(invalid.contains("invalid managed venv name"), "{invalid}");

        let missing = super::plan_venv_rename_sync(&ctx, "3.12.0/envs/ghost", "fresh")
            .expect_err("unknown venv");
        assert!(!missing.is_empty());
        assert!(
            ctx.root.join("venvs/3.12.0/demo").is_dir(),
            "preview must not rename"
        );
    }

    #[test]
    fn rename_moves_the_venv_and_returns_the_new_spec_with_the_stale_entry_point_warning() {
        let (_temp, ctx) = fake_context();
        let outcome =
            super::rename_venv_sync(&ctx, "3.12.0/envs/demo", " fresh.v2 ").expect("rename");
        assert_eq!(outcome.old_spec, "3.12.0/envs/demo");
        assert_eq!(outcome.new_spec, "3.12.0/envs/fresh.v2");
        assert!(
            outcome.message.starts_with("Renamed managed venv"),
            "{outcome:?}"
        );
        assert!(
            outcome
                .warnings
                .iter()
                .any(|w| w.contains("entry points") || w.contains("console-script")),
            "{outcome:?}"
        );
        assert!(!outcome.hints.is_empty(), "{outcome:?}");
        assert!(!ctx.root.join("venvs/3.12.0/demo").exists());
        assert!(ctx.root.join("venvs/3.12.0/fresh.v2").is_dir());

        let error = super::rename_venv_sync(&ctx, "3.12.0/envs/fresh.v2", "other")
            .expect_err("collision is refused");
        assert!(error.contains("other"), "{error}");
        assert!(ctx.root.join("venvs/3.12.0/fresh.v2").is_dir());
    }

    #[test]
    fn lookup_commands_must_be_bare_names() {
        assert_eq!(
            super::validate_lookup_command("  python3 ").unwrap(),
            "python3"
        );
        assert_eq!(
            super::validate_lookup_command("pip3.12").unwrap(),
            "pip3.12"
        );
        for bad in ["", "   ", "../bin/sh", "a/b", "a\\b", "--help", "py\nthon"] {
            assert!(super::validate_lookup_command(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn which_resolves_through_the_selection_and_explains_misses() {
        let (_temp, ctx) = fake_context();
        let hit = super::which_command_sync(&ctx, "python").expect("lookup");
        assert!(hit.found, "{hit:?}");
        let path = hit.path.expect("path");
        assert!(path.contains("3.12.0"), "global 3.12.0 is selected: {path}");
        assert!(hit.message.is_empty());
        assert_eq!(hit.also_in, ["3.12.0", "3.13.1"]);

        let miss = super::which_command_sync(&ctx, "no-such-tool-xyz").expect("lookup");
        assert!(!miss.found, "{miss:?}");
        assert!(miss.path.is_none());
        assert!(
            miss.message.contains("no-such-tool-xyz: command not found"),
            "{miss:?}"
        );
        assert!(miss.also_in.is_empty());

        assert!(super::which_command_sync(&ctx, "../python").is_err());
    }

    #[test]
    fn prefixes_cover_managed_runtimes_only() {
        let (_temp, ctx) = fake_context();
        let versions: Vec<String> = ["system", "3.12.0", "9.9.9", "3.13.1"]
            .iter()
            .map(|v| v.to_string())
            .collect();
        let prefixes = super::runtime_prefixes_sync(&ctx, &versions);
        let names: Vec<&str> = prefixes.iter().map(|p| p.version.as_str()).collect();
        assert_eq!(names, ["3.12.0", "3.13.1"]);
        assert!(
            std::path::Path::new(&prefixes[0].path).ends_with("versions/3.12.0"),
            "{prefixes:?}"
        );
    }

    #[test]
    fn open_folder_only_resolves_things_enchi_manages() {
        let (_temp, ctx) = fake_context();
        let runtime = super::resolve_managed_folder(&ctx, "3.13.1").expect("runtime");
        assert_eq!(runtime, ctx.root.join("versions").join("3.13.1"));
        let venv = super::resolve_managed_folder(&ctx, "3.12.0/envs/demo").expect("venv");
        assert_eq!(venv, ctx.root.join("venvs").join("3.12.0").join("demo"));
        let prefixed =
            super::resolve_managed_folder(&ctx, "venv:3.12.0/envs/other").expect("venv:");
        assert_eq!(
            prefixed,
            ctx.root.join("venvs").join("3.12.0").join("other")
        );

        for bad in ["system", "", "9.9.9", "../..", "/etc", "3.12.0/envs/ghost"] {
            assert!(super::resolve_managed_folder(&ctx, bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn shell_profile_allowlist_rejects_escape_paths() {
        use super::{known_shell_profile_paths, profile_path_is_allowed};
        use std::path::PathBuf;

        let home = PathBuf::from("/home/tester");
        let allowed = known_shell_profile_paths(&home);
        assert!(profile_path_is_allowed(&home, &allowed[0]));
        assert!(!profile_path_is_allowed(
            &home,
            &home.join("..").join("etc").join("profile")
        ));
        assert!(!profile_path_is_allowed(
            &home,
            &PathBuf::from("/tmp/evil.rc")
        ));
    }
}
