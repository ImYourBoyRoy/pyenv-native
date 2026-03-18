// ./crates/pyenv-core/src/shim.rs
//! Shim generation, executable dispatch, and Windows-first rehash groundwork.

use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::PyenvError;
use crate::catalog::installed_version_names;
use crate::command::CommandReport;
use crate::context::AppContext;
use crate::executable::{find_command_in_version, find_system_command};
use crate::plugin::{collect_rehash_hook_names, parse_hook_actions, run_hook_scripts};
use crate::runtime::{
    inventory_roots_for_version, managed_search_roots_for_version, prefix_bin_dirs,
};
use crate::version::resolve_selected_versions;

const SHIM_MANIFEST_FILE: &str = ".pyenv-shims.json";
const SHIM_LOCK_FILE: &str = ".pyenv-shims.lock";
const SHIM_LOCK_STALE_SECS: u64 = 60 * 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
struct ShimManifest {
    generated_at_epoch_seconds: u64,
    commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExecTarget {
    executable: PathBuf,
    prefix_dirs: Vec<PathBuf>,
    version_name: Option<String>,
}

#[derive(Debug)]
struct RehashLockGuard {
    path: PathBuf,
}

impl Drop for RehashLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn cmd_rehash(ctx: &AppContext) -> CommandReport {
    match rehash_shims(ctx) {
        Ok(_) => CommandReport::empty_success(),
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

pub fn cmd_exec(ctx: &AppContext, command: &str, args: &[String]) -> CommandReport {
    let target = match resolve_exec_target(ctx, command) {
        Ok(target) => target,
        Err(report) => return report,
    };

    let origin = crate::version::version_origin(ctx).to_string();
    let selected = resolve_selected_versions(ctx, false);
    let selected_value = selected.versions.join(":");
    let hook_results = match run_hook_scripts(
        ctx,
        "exec",
        &[
            ("PYENV_COMMAND", command.to_string()),
            (
                "PYENV_COMMAND_PATH",
                target.executable.display().to_string(),
            ),
            ("PYENV_VERSION", selected_value),
            ("PYENV_VERSION_ORIGIN", origin),
            (
                "PYENV_VERSION_RESOLVED",
                target
                    .version_name
                    .clone()
                    .unwrap_or_else(|| "system".to_string()),
            ),
        ],
    ) {
        Ok(results) => results,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };
    let hook_actions = parse_hook_actions(
        &hook_results
            .into_iter()
            .flat_map(|result| result.stdout)
            .collect::<Vec<_>>(),
    );

    let executable = hook_actions.command_path.unwrap_or(target.executable);
    let mut prefix_dirs = hook_actions.prepend_paths;
    prefix_dirs.extend(target.prefix_dirs);

    let mut child = Command::new(&executable);
    child.args(args);
    child.current_dir(&ctx.dir);
    child.env("PYENV_COMMAND", command);

    if let Some(path) = adjusted_path(ctx, &prefix_dirs) {
        child.env("PATH", path);
    }

    for (key, value) in hook_actions.env_pairs {
        child.env(key, value);
    }

    match child.status() {
        Ok(status) => CommandReport {
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit_code: status.code().unwrap_or(1),
        },
        Err(error) => CommandReport::failure(
            vec![format!(
                "pyenv: failed to execute {}: {error}",
                executable.display()
            )],
            1,
        ),
    }
}

pub(crate) fn rehash_shims(ctx: &AppContext) -> Result<usize, PyenvError> {
    let shims_dir = ctx.shims_dir();
    fs::create_dir_all(&shims_dir).map_err(io_error)?;
    let _lock = acquire_rehash_lock(&shims_dir)?;

    let commands = collect_rehash_commands(ctx)?;
    let previous = read_shim_manifest(&shims_dir).unwrap_or_default();
    let current = commands.iter().cloned().collect::<HashSet<_>>();

    for command in &commands {
        write_shim_artifacts(ctx, &shims_dir, command)?;
    }

    for stale in previous
        .commands
        .into_iter()
        .filter(|name| !current.contains(name))
    {
        remove_shim_artifacts(&shims_dir, &stale);
    }

    write_shim_manifest(&shims_dir, &commands)?;
    Ok(commands.len())
}

fn resolve_exec_target(ctx: &AppContext, command: &str) -> Result<ExecTarget, CommandReport> {
    let selected = resolve_selected_versions(ctx, false);
    let origin = selected.origin.to_string();
    let mut searched_system = false;

    for version in &selected.versions {
        if version == "system" {
            searched_system = true;
            if let Some(path) = find_system_command(ctx, command) {
                return Ok(ExecTarget {
                    executable: path,
                    prefix_dirs: Vec::new(),
                    version_name: Some("system".to_string()),
                });
            }
            continue;
        }

        if let Some(path) = find_command_in_version(ctx, version, command) {
            return Ok(ExecTarget {
                executable: path,
                prefix_dirs: managed_search_roots_for_version(ctx, version)
                    .into_iter()
                    .flat_map(|prefix| prefix_bin_dirs(&prefix))
                    .collect(),
                version_name: Some(version.clone()),
            });
        }
    }

    if !searched_system
        && let Some(path) = find_system_command(ctx, command) {
            return Ok(ExecTarget {
                executable: path,
                prefix_dirs: Vec::new(),
                version_name: Some("system".to_string()),
            });
        }

    let mut stderr = selected
        .missing
        .into_iter()
        .map(|version| format!("pyenv: version `{version}' is not installed (set by {origin})"))
        .collect::<Vec<_>>();
    stderr.push(format!("pyenv: {command}: command not found"));
    Err(CommandReport::failure(stderr, 127))
}

fn collect_rehash_commands(ctx: &AppContext) -> Result<Vec<String>, PyenvError> {
    let versions = installed_version_names(ctx)?;
    let mut commands = HashSet::new();

    for version in versions {
        for prefix in inventory_roots_for_version(ctx, &version) {
            for directory in prefix_bin_dirs(&prefix) {
                if !directory.is_dir() {
                    continue;
                }

                for entry in fs::read_dir(&directory).map_err(io_error)? {
                    let entry = entry.map_err(io_error)?;
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }

                    if let Some(name) =
                        crate::runtime::normalize_shim_name(&path, ctx.path_ext.as_deref())
                    {
                        commands.insert(name);
                    }
                }
            }
        }
    }

    for hook_name in collect_rehash_hook_names(ctx, &[])? {
        commands.insert(hook_name);
    }

    let mut values = commands.into_iter().collect::<Vec<_>>();
    values.sort_by_key(|value| value.to_ascii_lowercase());
    Ok(values)
}

fn adjusted_path(ctx: &AppContext, prefix_dirs: &[PathBuf]) -> Option<OsString> {
    let mut combined = Vec::new();
    let mut seen = HashSet::new();

    for path in prefix_dirs {
        if !path.as_os_str().is_empty()
            && !paths_equal(path, &ctx.shims_dir())
            && seen.insert(path_key(path))
        {
            combined.push(path.clone());
        }
    }

    for path in ctx
        .path_env
        .clone()
        .or_else(|| env::var_os("PATH"))
        .as_ref()
        .map(env::split_paths)
        .into_iter()
        .flatten()
    {
        if !path.as_os_str().is_empty()
            && !paths_equal(&path, &ctx.shims_dir())
            && seen.insert(path_key(&path))
        {
            combined.push(path);
        }
    }

    env::join_paths(combined).ok()
}

fn write_shim_artifacts(
    ctx: &AppContext,
    shims_dir: &Path,
    command: &str,
) -> Result<(), PyenvError> {
    if cfg!(windows) {
        if ctx.exe_path.is_file() {
            create_windows_native_shim(&ctx.exe_path, &shim_native_path(shims_dir, command))?;
        }
        fs::write(shim_cmd_path(shims_dir, command), render_cmd_shim()).map_err(io_error)?;
        fs::write(shim_bat_path(shims_dir, command), render_bat_shim()).map_err(io_error)?;
        fs::write(shim_ps1_path(shims_dir, command), render_ps1_shim()).map_err(io_error)?;
    } else {
        let shim_path = shim_posix_path(shims_dir, command);
        fs::write(&shim_path, render_posix_shim(&ctx.exe_path)).map_err(io_error)?;
        make_executable(&shim_path)?;
    }
    Ok(())
}

fn render_cmd_shim() -> &'static str {
    "@echo off\r\n\"%~dp0%~n0.exe\" %*\r\n"
}

fn render_bat_shim() -> &'static str {
    "@echo off\r\n\"%~dp0%~n0.exe\" %*\r\n"
}

fn render_ps1_shim() -> &'static str {
    "$exe = Join-Path $PSScriptRoot ([System.IO.Path]::GetFileNameWithoutExtension($MyInvocation.MyCommand.Name) + '.exe')\r\n& $exe @args\r\nexit $LASTEXITCODE\r\n"
}

fn render_posix_shim(pyenv_exe: &Path) -> String {
    format!(
        "#!/usr/bin/env sh\nexec '{}' exec \"$(basename \"$0\")\" \"$@\"\n",
        sh_single_quote(&pyenv_exe.display().to_string())
    )
}

fn create_windows_native_shim(source: &Path, destination: &Path) -> Result<(), PyenvError> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    if destination.exists() {
        let _ = fs::remove_file(destination);
    }

    match fs::hard_link(source, destination) {
        Ok(_) => Ok(()),
        Err(_) => {
            fs::copy(source, destination).map_err(io_error)?;
            Ok(())
        }
    }
}

fn make_executable(path: &Path) -> Result<(), PyenvError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let metadata = fs::metadata(path).map_err(io_error)?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).map_err(io_error)?;
    }

    #[cfg(not(unix))]
    {
        let _ = path;
    }

    Ok(())
}

fn remove_shim_artifacts(shims_dir: &Path, command: &str) {
    for path in [
        shim_native_path(shims_dir, command),
        shim_cmd_path(shims_dir, command),
        shim_bat_path(shims_dir, command),
        shim_ps1_path(shims_dir, command),
        shim_posix_path(shims_dir, command),
    ] {
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }
}

fn shim_native_path(shims_dir: &Path, command: &str) -> PathBuf {
    if cfg!(windows) {
        shims_dir.join(format!("{command}.exe"))
    } else {
        shims_dir.join(command)
    }
}

fn shim_cmd_path(shims_dir: &Path, command: &str) -> PathBuf {
    shims_dir.join(format!("{command}.cmd"))
}

fn shim_ps1_path(shims_dir: &Path, command: &str) -> PathBuf {
    shims_dir.join(format!("{command}.ps1"))
}

fn shim_bat_path(shims_dir: &Path, command: &str) -> PathBuf {
    shims_dir.join(format!("{command}.bat"))
}

fn shim_posix_path(shims_dir: &Path, command: &str) -> PathBuf {
    shims_dir.join(command)
}

fn read_shim_manifest(shims_dir: &Path) -> Option<ShimManifest> {
    let path = shims_dir.join(SHIM_MANIFEST_FILE);
    let contents = fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

fn write_shim_manifest(shims_dir: &Path, commands: &[String]) -> Result<(), PyenvError> {
    let path = shims_dir.join(SHIM_MANIFEST_FILE);
    let manifest = ShimManifest {
        generated_at_epoch_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        commands: commands.to_vec(),
    };
    let payload = serde_json::to_string_pretty(&manifest).map_err(|error| {
        PyenvError::Io(format!("pyenv: failed to serialize shim manifest: {error}"))
    })?;
    fs::write(path, payload).map_err(io_error)
}

fn paths_equal(lhs: &Path, rhs: &Path) -> bool {
    if cfg!(windows) {
        lhs.to_string_lossy()
            .replace('/', "\\")
            .eq_ignore_ascii_case(&rhs.to_string_lossy().replace('/', "\\"))
    } else {
        lhs == rhs
    }
}

fn path_key(path: &Path) -> String {
    if cfg!(windows) {
        path.to_string_lossy()
            .replace('/', "\\")
            .to_ascii_lowercase()
    } else {
        path.to_string_lossy().to_string()
    }
}

fn sh_single_quote(value: &str) -> String {
    value.replace('\'', "'\\''")
}

fn acquire_rehash_lock(shims_dir: &Path) -> Result<RehashLockGuard, PyenvError> {
    let path = shims_dir.join(SHIM_LOCK_FILE);
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let payload = format!("pid={}\ncreated_at={created_at}\n", process::id());

    for _ in 0..2 {
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                use std::io::Write as _;

                file.write_all(payload.as_bytes()).map_err(io_error)?;
                file.flush().map_err(io_error)?;
                return Ok(RehashLockGuard { path });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if lock_file_is_stale(&path) {
                    let _ = fs::remove_file(&path);
                    continue;
                }
                return Err(PyenvError::Io(format!(
                    "pyenv: cannot rehash: lock {} already exists",
                    path.display()
                )));
            }
            Err(error) => return Err(io_error(error)),
        }
    }

    Err(PyenvError::Io(format!(
        "pyenv: cannot rehash: failed to acquire lock {}",
        path.display()
    )))
}

fn lock_file_is_stale(path: &Path) -> bool {
    let Ok(contents) = fs::read_to_string(path) else {
        return false;
    };
    let Some(created_at) = contents
        .lines()
        .find_map(|line| line.strip_prefix("created_at="))
        .and_then(|value| value.parse::<u64>().ok())
    else {
        return false;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    now.saturating_sub(created_at) > SHIM_LOCK_STALE_SECS
}

fn io_error(error: std::io::Error) -> PyenvError {
    PyenvError::Io(format!("pyenv: {error}"))
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::{
        SHIM_LOCK_FILE, adjusted_path, cmd_exec, cmd_rehash, make_executable, rehash_shims,
    };

    fn test_path_ext() -> Option<OsString> {
        if cfg!(windows) {
            Some(OsString::from(".exe;.cmd;.bat"))
        } else {
            None
        }
    }

    fn executable_name(name: &str) -> String {
        if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_string()
        }
    }

    fn test_context() -> (TempDir, AppContext) {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        let dir = temp.path().join("work");
        let exe_path = temp.path().join("pyenv.exe");
        fs::create_dir_all(root.join("versions")).expect("versions");
        fs::create_dir_all(&dir).expect("work");
        fs::write(&exe_path, "shim source").expect("exe");

        let ctx = AppContext {
            root,
            dir,
            exe_path,
            env_version: None,
            env_shell: None,
            path_env: None,
            path_ext: test_path_ext(),
            config: AppConfig::default(),
        };

        (temp, ctx)
    }

    #[test]
    fn rehash_generates_cmd_shims_and_manifest() {
        let (_temp, ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.6");
        if cfg!(windows) {
            fs::create_dir_all(version_dir.join("Scripts")).expect("scripts");
            fs::write(version_dir.join("python.exe"), "").expect("python");
            fs::write(version_dir.join("Scripts").join("pip.cmd"), "").expect("pip");
        } else {
            fs::create_dir_all(version_dir.join("bin")).expect("bin");
            fs::write(version_dir.join("bin").join("python"), "").expect("python");
            fs::write(version_dir.join("bin").join("pip"), "").expect("pip");
        }

        let count = rehash_shims(&ctx).expect("rehash");
        assert_eq!(count, 2);
        if cfg!(windows) {
            assert!(ctx.shims_dir().join("python.exe").is_file());
            assert!(ctx.shims_dir().join("python.cmd").is_file());
            assert!(ctx.shims_dir().join("python.bat").is_file());
            assert!(ctx.shims_dir().join("pip.cmd").is_file());
        } else {
            assert!(ctx.shims_dir().join("python").is_file());
            assert!(ctx.shims_dir().join("pip").is_file());
        }
        assert!(ctx.shims_dir().join(".pyenv-shims.json").is_file());
        assert!(!ctx.shims_dir().join(SHIM_LOCK_FILE).exists());

        let report = cmd_rehash(&ctx);
        assert_eq!(report.exit_code, 0);
    }

    #[test]
    fn rehash_removes_stale_shims() {
        let (_temp, ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.6");
        if cfg!(windows) {
            fs::create_dir_all(&version_dir).expect("version");
            fs::write(version_dir.join("python.exe"), "").expect("python");
        } else {
            fs::create_dir_all(version_dir.join("bin")).expect("version");
            fs::write(version_dir.join("bin").join("python"), "").expect("python");
        }

        rehash_shims(&ctx).expect("rehash");
        assert!(ctx.shims_dir().join(executable_name("python")).is_file());
        if cfg!(windows) {
            assert!(ctx.shims_dir().join("python.cmd").is_file());
        }

        fs::remove_dir_all(&version_dir).expect("remove version");
        rehash_shims(&ctx).expect("rehash");
        assert!(!ctx.shims_dir().join(executable_name("python")).exists());
        if cfg!(windows) {
            assert!(!ctx.shims_dir().join("python.cmd").exists());
        }
    }

    #[test]
    fn rehash_hooks_can_register_additional_commands() {
        let (_temp, ctx) = test_context();
        let hook_dir = ctx.root.join("pyenv.d").join("rehash");
        fs::create_dir_all(&hook_dir).expect("hook dir");
        if cfg!(windows) {
            fs::write(hook_dir.join("register.cmd"), "@echo extra-tool").expect("hook");
        } else {
            fs::write(
                hook_dir.join("register.sh"),
                "#!/usr/bin/env sh\necho extra-tool\n",
            )
            .expect("hook");
        }

        rehash_shims(&ctx).expect("rehash");
        if cfg!(windows) {
            assert!(ctx.shims_dir().join("extra-tool.exe").is_file());
            assert!(ctx.shims_dir().join("extra-tool.cmd").is_file());
            assert!(ctx.shims_dir().join("extra-tool.bat").is_file());
            assert!(ctx.shims_dir().join("extra-tool.ps1").is_file());
        } else {
            assert!(ctx.shims_dir().join("extra-tool").is_file());
        }
    }

    #[test]
    fn exec_hooks_can_override_target_and_set_environment() {
        let (_temp, mut ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.6");
        let hook_dir = ctx.root.join("pyenv.d").join("exec");
        let target_path = if cfg!(windows) {
            ctx.root.join("override.cmd")
        } else {
            ctx.root.join("override.sh")
        };
        let output_path = ctx.root.join("exec-output.txt");
        fs::create_dir_all(&version_dir).expect("version");
        fs::create_dir_all(&hook_dir).expect("hook dir");
        if cfg!(windows) {
            fs::write(version_dir.join("python.cmd"), "@echo base").expect("python");
            fs::write(
                &target_path,
                format!("@echo %DEMO_ENV%>{}", output_path.display()),
            )
            .expect("override");
            fs::write(
                hook_dir.join("redirect.cmd"),
                format!(
                    "@echo ENV:DEMO_ENV=from-hook\r\n@echo PYENV_COMMAND_PATH={}",
                    target_path.display()
                ),
            )
            .expect("hook");
        } else {
            fs::write(version_dir.join("python"), "#!/usr/bin/env sh\nexit 0\n").expect("python");
            fs::write(
                &target_path,
                format!(
                    "#!/usr/bin/env sh\nprintf '%s' \"$DEMO_ENV\" > '{}'\n",
                    output_path.display()
                ),
            )
            .expect("override");
            make_executable(&target_path).expect("target executable");
            fs::write(
                hook_dir.join("redirect.sh"),
                format!(
                    "#!/usr/bin/env sh\necho ENV:DEMO_ENV=from-hook\necho PYENV_COMMAND_PATH={}\n",
                    target_path.display()
                ),
            )
            .expect("hook");
        }
        ctx.env_version = Some("3.12.6".to_string());

        let report = cmd_exec(&ctx, "python", &[]);
        assert_eq!(report.exit_code, 0);
        assert_eq!(
            fs::read_to_string(output_path).expect("output").trim(),
            "from-hook"
        );
    }

    #[test]
    fn rehash_fails_when_fresh_lock_exists() {
        let (_temp, ctx) = test_context();
        fs::create_dir_all(ctx.shims_dir()).expect("shims dir");
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        fs::write(
            ctx.shims_dir().join(SHIM_LOCK_FILE),
            format!("pid=9999\ncreated_at={created_at}\n"),
        )
        .expect("lock");

        let error = rehash_shims(&ctx).expect_err("rehash should fail");
        assert!(error.to_string().contains("cannot rehash"));
    }

    #[test]
    fn rehash_replaces_stale_lock_file() {
        let (_temp, ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.6");
        fs::create_dir_all(&version_dir).expect("version");
        fs::write(version_dir.join("python.exe"), "").expect("python");
        fs::create_dir_all(ctx.shims_dir()).expect("shims dir");
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(60 * 60);
        fs::write(
            ctx.shims_dir().join(SHIM_LOCK_FILE),
            format!("pid=9999\ncreated_at={created_at}\n"),
        )
        .expect("lock");

        let count = rehash_shims(&ctx).expect("rehash");
        assert_eq!(count, 1);
        assert!(!ctx.shims_dir().join(SHIM_LOCK_FILE).exists());
    }

    #[test]
    fn adjusted_path_deduplicates_prefix_and_existing_entries() {
        let (_temp, mut ctx) = test_context();
        let first = ctx.root.join("versions").join("3.12.6").join("Scripts");
        let first = if cfg!(windows) {
            first
        } else {
            ctx.root.join("versions").join("3.12.6").join("bin")
        };
        let second = if cfg!(windows) {
            PathBuf::from("C:\\Windows\\System32")
        } else {
            PathBuf::from("/usr/bin")
        };
        ctx.path_env = Some(
            env::join_paths([
                first.clone(),
                ctx.shims_dir(),
                second.clone(),
                first.clone(),
            ])
            .expect("path env"),
        );

        let joined = adjusted_path(&ctx, &[first.clone(), second.clone(), first.clone()])
            .expect("adjusted path");
        let entries = env::split_paths(&joined).collect::<Vec<_>>();
        assert_eq!(entries, vec![first, second]);
    }
}
