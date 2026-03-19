// ./crates/pyenv-core/src/venv.rs
//! Managed virtual environment commands for creating, listing, inspecting, renaming, deleting,
//! and assigning pyenv-native virtual environments under `PYENV_ROOT/versions/<runtime>/envs`.
//! Use these helpers through `pyenv venv ...` to avoid name collisions, keep metadata predictable,
//! and write `.python-version` files that target explicit managed env specs like
//! `3.13.12/envs/my-project`.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::catalog::{compare_version_names, installed_version_names, latest_installed_version};
use crate::command::CommandReport;
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::runtime::find_command_in_prefix;
use crate::version::{cmd_global, cmd_local, installed_version_dir};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VenvUseScope {
    Local,
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagedVenvInfo {
    pub name: String,
    pub base_version: String,
    pub spec: String,
    pub path: PathBuf,
    pub python_path: Option<PathBuf>,
    pub pip_path: Option<PathBuf>,
}

pub fn cmd_venv_list(ctx: &AppContext, bare: bool, json: bool) -> CommandReport {
    match list_managed_venvs(ctx) {
        Ok(venvs) => {
            if json {
                return json_success(&venvs);
            }

            if bare {
                return CommandReport::success(venvs.into_iter().map(|info| info.spec).collect());
            }

            if venvs.is_empty() {
                return CommandReport::success(vec![
                    "No managed virtual environments found.".to_string(),
                    "Create one with `pyenv venv create <runtime> <name>`.".to_string(),
                ]);
            }

            let mut stdout = vec!["Managed virtual environments:".to_string()];
            stdout.extend(venvs.into_iter().map(|info| {
                format!(
                    "  - {} (base {}, python {})",
                    info.spec,
                    info.base_version,
                    info.python_path
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "missing".to_string())
                )
            }));
            CommandReport::success(stdout)
        }
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

pub fn cmd_venv_info(ctx: &AppContext, spec: &str, json: bool) -> CommandReport {
    match resolve_managed_venv(ctx, spec) {
        Ok(info) => {
            if json {
                return json_success(&info);
            }

            let mut stdout = vec![
                format!("Name: {}", info.name),
                format!("Spec: {}", info.spec),
                format!("Base runtime: {}", info.base_version),
                format!("Location: {}", info.path.display()),
            ];
            stdout.push(format!(
                "Python: {}",
                info.python_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "missing".to_string())
            ));
            stdout.push(format!(
                "Pip: {}",
                info.pip_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "missing".to_string())
            ));
            CommandReport::success(stdout)
        }
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

pub fn cmd_venv_create(
    ctx: &AppContext,
    requested_version: &str,
    name: &str,
    force: bool,
    set_local: bool,
) -> CommandReport {
    match create_managed_venv(ctx, requested_version, name, force, set_local) {
        Ok((info, local_written, progress_steps)) => {
            let mut stdout = vec!["Progress:".to_string()];
            stdout.extend(progress_steps.into_iter().map(|step| format!("  - {step}")));
            stdout.push(format!("Managed venv created: {}", info.spec));
            stdout.push(format!("Location: {}", info.path.display()));
            stdout.push(format!(
                "Python: {}",
                info.python_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "missing".to_string())
            ));
            stdout.push(format!(
                "Pip: {}",
                info.pip_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "missing".to_string())
            ));
            stdout.push(format!("Local version updated: {local_written}"));
            CommandReport::success(stdout)
        }
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

pub fn cmd_venv_delete(ctx: &AppContext, spec: &str, force: bool) -> CommandReport {
    let info = match resolve_managed_venv(ctx, spec) {
        Ok(info) => info,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };

    if !force && !confirm_action(&format!("pyenv: remove managed venv {}? [y/N] ", info.spec)) {
        return CommandReport::failure(
            vec!["pyenv: managed venv removal cancelled".to_string()],
            1,
        );
    }

    match fs::remove_dir_all(&info.path) {
        Ok(_) => CommandReport::success(vec![
            format!("Removed managed venv {}", info.spec),
            "Hint: update any `.python-version` files that pointed at this venv.".to_string(),
        ]),
        Err(error) => CommandReport::failure(
            vec![format!(
                "pyenv: failed to remove managed venv {}: {error}",
                info.path.display()
            )],
            1,
        ),
    }
}

pub fn cmd_venv_rename(ctx: &AppContext, spec: &str, new_name: &str) -> CommandReport {
    let info = match resolve_managed_venv(ctx, spec) {
        Ok(info) => info,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };

    if !is_safe_env_name(new_name) {
        return CommandReport::failure(
            vec![format!(
                "pyenv: invalid managed venv name `{new_name}`; use letters, numbers, ., _, or -"
            )],
            1,
        );
    }

    let collisions = find_env_name_matches(ctx, new_name).unwrap_or_default();
    if collisions.iter().any(|item| item.spec != info.spec) {
        return CommandReport::failure(vec![format_collision_error(new_name, &collisions)], 1);
    }

    let target = installed_version_dir(ctx, &info.base_version)
        .join("envs")
        .join(new_name);
    if target.exists() {
        return CommandReport::failure(
            vec![format!(
                "pyenv: managed venv target already exists at {}",
                target.display()
            )],
            1,
        );
    }

    match fs::rename(&info.path, &target) {
        Ok(_) => CommandReport::success(vec![
            format!(
                "Renamed managed venv {} -> {}/envs/{}",
                info.spec, info.base_version, new_name
            ),
            "Hint: update any `.python-version` files that referenced the old venv spec."
                .to_string(),
        ]),
        Err(error) => CommandReport::failure(
            vec![format!(
                "pyenv: failed to rename managed venv {}: {error}",
                info.spec
            )],
            1,
        ),
    }
}

pub fn cmd_venv_use(ctx: &AppContext, spec: &str, scope: VenvUseScope) -> CommandReport {
    let info = match resolve_managed_venv(ctx, spec) {
        Ok(info) => info,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };

    let report = match scope {
        VenvUseScope::Local => cmd_local(ctx, std::slice::from_ref(&info.spec), false, true),
        VenvUseScope::Global => cmd_global(ctx, std::slice::from_ref(&info.spec), false),
    };

    if report.exit_code != 0 {
        return report;
    }

    let scope_label = match scope {
        VenvUseScope::Local => "local",
        VenvUseScope::Global => "global",
    };
    CommandReport::success(vec![
        format!("Selected managed venv {} for {scope_label} use.", info.spec),
        format!(
            "Hint: `python` and `pip` will now resolve from {} once shims are active.",
            info.spec
        ),
    ])
}

pub fn list_managed_venvs(ctx: &AppContext) -> Result<Vec<ManagedVenvInfo>, PyenvError> {
    let mut results = Vec::new();
    for version in installed_version_names(ctx)? {
        let envs_dir = installed_version_dir(ctx, &version).join("envs");
        if !envs_dir.is_dir() {
            continue;
        }

        let mut names = fs::read_dir(&envs_dir)
            .map_err(io_error)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .filter_map(|path| {
                path.file_name()
                    .map(|value| value.to_string_lossy().to_string())
            })
            .collect::<Vec<_>>();
        names.sort_by_key(|lhs| lhs.to_ascii_lowercase());

        for name in names {
            results.push(build_managed_venv_info(
                ctx,
                version.clone(),
                name,
                envs_dir.clone(),
            ));
        }
    }

    results.sort_by(|lhs, rhs| compare_version_names(&lhs.spec, &rhs.spec));
    Ok(results)
}

fn create_managed_venv(
    ctx: &AppContext,
    requested_version: &str,
    name: &str,
    force: bool,
    set_local: bool,
) -> Result<(ManagedVenvInfo, bool, Vec<String>), PyenvError> {
    if !is_safe_env_name(name) {
        return Err(PyenvError::Io(format!(
            "pyenv: invalid managed venv name `{name}`; use letters, numbers, ., _, or -"
        )));
    }

    let resolved_version = resolve_installed_runtime_version(ctx, requested_version)?;
    let collisions = find_env_name_matches(ctx, name)?;
    if !collisions.is_empty() {
        let exact_spec = format!("{resolved_version}/envs/{name}");
        if !(force && collisions.iter().all(|item| item.spec == exact_spec)) {
            return Err(PyenvError::Io(format_collision_error(name, &collisions)));
        }
    }

    let base_prefix = installed_version_dir(ctx, &resolved_version);
    let interpreter_path = interpreter_for_prefix(&base_prefix).ok_or_else(|| {
        PyenvError::Io(format!(
            "pyenv: failed to locate a Python interpreter under {}",
            base_prefix.display()
        ))
    })?;

    let envs_dir = base_prefix.join("envs");
    fs::create_dir_all(&envs_dir).map_err(io_error)?;
    let venv_path = envs_dir.join(name);

    let mut progress_steps = vec![
        format!(
            "plan: resolved base runtime {} -> {}",
            requested_version, resolved_version
        ),
        format!("venv: target managed env path {}", venv_path.display()),
    ];

    if venv_path.exists() {
        if !force {
            let spec = format!("{resolved_version}/envs/{name}");
            return Err(PyenvError::Io(format!(
                "pyenv: managed venv `{}` already exists; use --force to recreate it",
                spec
            )));
        }
        fs::remove_dir_all(&venv_path).map_err(io_error)?;
        progress_steps.push(format!(
            "cleanup: removed existing managed env at {}",
            venv_path.display()
        ));
    }

    let status = Command::new(&interpreter_path)
        .arg("-m")
        .arg("venv")
        .arg(&venv_path)
        .status()
        .map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to run '{}' -m venv {}: {error}",
                interpreter_path.display(),
                venv_path.display()
            ))
        })?;
    if !status.success() {
        return Err(PyenvError::Io(format!(
            "pyenv: '{}' -m venv {} exited with status {:?}",
            interpreter_path.display(),
            venv_path.display(),
            status.code()
        )));
    }
    progress_steps.push(format!("venv: created managed env {}", venv_path.display()));

    let info = build_managed_venv_info(ctx, resolved_version.clone(), name.to_string(), envs_dir);

    let local_written = if set_local {
        let report = cmd_local(ctx, std::slice::from_ref(&info.spec), false, true);
        if report.exit_code != 0 {
            return Err(PyenvError::Io(report.stderr.join("\n")));
        }
        progress_steps.push(format!(
            "selection: wrote local .python-version for {}",
            info.spec
        ));
        true
    } else {
        false
    };

    Ok((info, local_written, progress_steps))
}

fn resolve_managed_venv(ctx: &AppContext, spec: &str) -> Result<ManagedVenvInfo, PyenvError> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err(PyenvError::Io(
            "pyenv: managed venv spec cannot be empty".to_string(),
        ));
    }

    if let Some((base_version, name)) = split_env_spec(trimmed) {
        let envs_dir = installed_version_dir(ctx, &base_version).join("envs");
        let info = build_managed_venv_info(ctx, base_version.clone(), name.clone(), envs_dir);
        if info.path.is_dir() {
            return Ok(info);
        }
        return Err(PyenvError::Io(format!(
            "pyenv: managed venv `{}` is not installed",
            info.spec
        )));
    }

    let matches = find_env_name_matches(ctx, trimmed)?;
    match matches.len() {
        0 => Err(PyenvError::Io(format!(
            "pyenv: no managed venv named `{trimmed}` was found"
        ))),
        1 => Ok(matches[0].clone()),
        _ => Err(PyenvError::Io(format!(
            "pyenv: managed venv name `{trimmed}` is ambiguous; use one of: {}",
            matches
                .iter()
                .map(|info| format!("`{}`", info.spec))
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

fn find_env_name_matches(ctx: &AppContext, name: &str) -> Result<Vec<ManagedVenvInfo>, PyenvError> {
    let normalize = |value: &str| {
        if cfg!(windows) {
            value.to_ascii_lowercase()
        } else {
            value.to_string()
        }
    };
    let requested = normalize(name);
    Ok(list_managed_venvs(ctx)?
        .into_iter()
        .filter(|info| normalize(&info.name) == requested)
        .collect())
}

fn build_managed_venv_info(
    _ctx: &AppContext,
    base_version: String,
    name: String,
    envs_dir: PathBuf,
) -> ManagedVenvInfo {
    let path = envs_dir.join(&name);
    ManagedVenvInfo {
        spec: format!("{base_version}/envs/{name}"),
        base_version,
        name,
        python_path: interpreter_for_prefix(&path),
        pip_path: pip_for_prefix(&path),
        path,
    }
}

fn resolve_installed_runtime_version(
    ctx: &AppContext,
    requested_version: &str,
) -> Result<String, PyenvError> {
    let normalized = requested_version
        .strip_prefix("python-")
        .unwrap_or(requested_version)
        .trim();

    if installed_version_dir(ctx, normalized).is_dir() {
        return Ok(normalized.to_string());
    }

    if let Some(resolved) = latest_installed_version(ctx, normalized) {
        return Ok(resolved);
    }

    Err(PyenvError::VersionNotInstalled(
        normalized.to_string(),
        "pyenv venv create".to_string(),
    ))
}

fn interpreter_for_prefix(prefix: &Path) -> Option<PathBuf> {
    for command in ["python", "python3", "pypy3"] {
        if let Some(path) = find_command_in_prefix(prefix, command, None) {
            return Some(path);
        }
    }
    None
}

fn pip_for_prefix(prefix: &Path) -> Option<PathBuf> {
    for command in ["pip", "pip3"] {
        if let Some(path) = find_command_in_prefix(prefix, command, None) {
            return Some(path);
        }
    }
    None
}

fn split_env_spec(spec: &str) -> Option<(String, String)> {
    let normalized = spec.replace('\\', "/");
    let marker = "/envs/";
    let (base, name) = normalized.split_once(marker)?;
    let trimmed_base = base.trim().trim_matches('/');
    let trimmed_name = name.trim().trim_matches('/');
    if trimmed_base.is_empty() || trimmed_name.is_empty() {
        return None;
    }
    Some((trimmed_base.to_string(), trimmed_name.to_string()))
}

fn is_safe_env_name(value: &str) -> bool {
    !value.trim().is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
}

fn format_collision_error(name: &str, collisions: &[ManagedVenvInfo]) -> String {
    format!(
        "pyenv: managed venv name `{name}` already exists; use a unique name or reference one of: {}",
        collisions
            .iter()
            .map(|info| format!("`{}`", info.spec))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn confirm_action(prompt: &str) -> bool {
    let _ = write!(io::stdout(), "{prompt}");
    let _ = io::stdout().flush();

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes"),
        Err(_) => false,
    }
}

fn io_error(error: std::io::Error) -> PyenvError {
    PyenvError::Io(format!("pyenv: {error}"))
}

fn json_success<T: Serialize>(value: &T) -> CommandReport {
    match serde_json::to_string_pretty(value) {
        Ok(payload) => CommandReport::success(payload.lines().map(ToOwned::to_owned).collect()),
        Err(error) => CommandReport::failure(
            vec![format!("pyenv: failed to serialize JSON output: {error}")],
            1,
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::{VenvUseScope, cmd_venv_create, cmd_venv_info, cmd_venv_list, cmd_venv_use};

    fn python_file_name() -> &'static str {
        if cfg!(windows) {
            "python.exe"
        } else {
            "python"
        }
    }

    fn pip_file_name() -> &'static str {
        if cfg!(windows) { "pip.exe" } else { "pip" }
    }

    fn test_context() -> (TempDir, AppContext) {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        let dir = temp.path().join("work");
        fs::create_dir_all(root.join("versions")).expect("versions");
        fs::create_dir_all(&dir).expect("work");

        let ctx = AppContext {
            root,
            dir,
            exe_path: PathBuf::from("pyenv"),
            env_version: None,
            env_shell: None,
            path_env: None,
            path_ext: None,
            config: AppConfig::default(),
        };

        (temp, ctx)
    }

    fn create_fake_runtime(ctx: &AppContext, version: &str) {
        let version_dir = ctx.versions_dir().join(version);
        if cfg!(windows) {
            fs::create_dir_all(version_dir.join("Scripts")).expect("scripts");
            fs::write(version_dir.join(python_file_name()), "").expect("python");
            fs::write(version_dir.join("Scripts").join(pip_file_name()), "").expect("pip");
        } else {
            fs::create_dir_all(version_dir.join("bin")).expect("bin");
            fs::write(version_dir.join("bin").join(python_file_name()), "").expect("python");
            fs::write(version_dir.join("bin").join(pip_file_name()), "").expect("pip");
        }
    }

    fn create_fake_managed_env(ctx: &AppContext, version: &str, name: &str) {
        let env_dir = ctx.versions_dir().join(version).join("envs").join(name);
        if cfg!(windows) {
            fs::create_dir_all(env_dir.join("Scripts")).expect("scripts");
            fs::write(env_dir.join("Scripts").join("python.exe"), "").expect("python");
            fs::write(env_dir.join("Scripts").join("pip.exe"), "").expect("pip");
        } else {
            fs::create_dir_all(env_dir.join("bin")).expect("bin");
            fs::write(env_dir.join("bin").join("python"), "").expect("python");
            fs::write(env_dir.join("bin").join("pip"), "").expect("pip");
        }
    }

    #[test]
    fn venv_list_reports_managed_env_specs() {
        let (_temp, ctx) = test_context();
        create_fake_runtime(&ctx, "3.12.6");
        create_fake_managed_env(&ctx, "3.12.6", "demo");

        let report = cmd_venv_list(&ctx, true, false);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec!["3.12.6/envs/demo".to_string()]);
    }

    #[test]
    fn venv_info_accepts_short_name_when_unique() {
        let (_temp, ctx) = test_context();
        create_fake_runtime(&ctx, "3.12.6");
        create_fake_managed_env(&ctx, "3.12.6", "demo");

        let report = cmd_venv_info(&ctx, "demo", false);
        assert_eq!(report.exit_code, 0);
        assert!(
            report
                .stdout
                .iter()
                .any(|line| line == "Spec: 3.12.6/envs/demo")
        );
    }

    #[test]
    fn venv_use_local_writes_env_spec_to_python_version_file() {
        let (_temp, ctx) = test_context();
        create_fake_runtime(&ctx, "3.12.6");
        create_fake_managed_env(&ctx, "3.12.6", "demo");

        let report = cmd_venv_use(&ctx, "demo", VenvUseScope::Local);
        assert_eq!(report.exit_code, 0);
        let file = fs::read_to_string(ctx.dir.join(".python-version")).expect("version file");
        assert_eq!(file, "3.12.6/envs/demo\n");
    }

    #[test]
    fn venv_create_rejects_duplicate_name_collisions() {
        let (_temp, ctx) = test_context();
        create_fake_runtime(&ctx, "3.12.6");
        create_fake_runtime(&ctx, "3.13.1");
        create_fake_managed_env(&ctx, "3.12.6", "demo");

        let report = cmd_venv_create(&ctx, "3.13", "demo", false, false);
        assert_eq!(report.exit_code, 1);
        assert!(report.stderr[0].contains("managed venv name `demo` already exists"));
    }
}
