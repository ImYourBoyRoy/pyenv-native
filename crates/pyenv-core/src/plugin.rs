// ./crates/pyenv-core/src/plugin.rs
//! Plugin command discovery, completion, and hook execution for pyenv-compatible extension points.

use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::runtime::{candidate_file_names, search_path_entries};

pub const DEFAULT_HOOK_COMMANDS: &[&str] = &[
    "exec",
    "install",
    "rehash",
    "uninstall",
    "version-name",
    "version-origin",
    "which",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookResult {
    pub path: PathBuf,
    pub stdout: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HookActions {
    pub command_path: Option<PathBuf>,
    pub prepend_paths: Vec<PathBuf>,
    pub env_pairs: Vec<(String, String)>,
    pub passthrough_lines: Vec<String>,
}

pub fn cmd_hooks(ctx: &AppContext, hook: &str) -> CommandReport {
    if hook == "--complete" {
        return CommandReport::success(
            DEFAULT_HOOK_COMMANDS
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
        );
    }

    match discover_hook_scripts(ctx, hook) {
        Ok(scripts) => CommandReport::success(
            scripts
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
        ),
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

pub fn cmd_external(ctx: &AppContext, args: &[String]) -> CommandReport {
    let Some((command, rest)) = args.split_first() else {
        return CommandReport::failure(vec!["pyenv: no external command specified".to_string()], 1);
    };

    let Some(command_path) = find_plugin_command(ctx, command) else {
        return CommandReport::failure(vec![format!("pyenv: no such command `{command}`")], 1);
    };

    match run_process(&command_path, rest, ctx, &[], false) {
        Ok((exit_code, _, _)) => CommandReport {
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit_code,
        },
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

pub fn discover_plugin_commands(ctx: &AppContext) -> Vec<String> {
    let mut commands = HashSet::new();

    for bin_dir in plugin_search_dirs(ctx) {
        let entries = std::fs::read_dir(&bin_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();

        for path in entries {
            let Some(stem) = path
                .file_stem()
                .map(|value| value.to_string_lossy().to_string())
            else {
                continue;
            };
            let Some(command) = stem.strip_prefix("pyenv-") else {
                continue;
            };
            if !command.trim().is_empty() {
                commands.insert(command.to_string());
            }
        }
    }

    let mut values = commands.into_iter().collect::<Vec<_>>();
    values.sort_by_key(|value| value.to_ascii_lowercase());
    values
}

pub fn complete_plugin_command(
    ctx: &AppContext,
    command: &str,
    args: &[String],
) -> Result<Option<Vec<String>>, PyenvError> {
    let Some(command_path) = find_plugin_command(ctx, command) else {
        return Ok(None);
    };

    let mut completion_args = vec!["--complete".to_string()];
    completion_args.extend(args.iter().cloned());
    let (exit_code, stdout, stderr) = run_process(&command_path, &completion_args, ctx, &[], true)?;
    if exit_code != 0 {
        let detail = if !stderr.is_empty() {
            stderr.join("\n")
        } else {
            format!("exit code {exit_code}")
        };
        return Err(PyenvError::Io(format!(
            "pyenv: completion failed for {}: {detail}",
            command_path.display()
        )));
    }

    Ok(Some(
        stdout
            .into_iter()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect(),
    ))
}

pub fn discover_hook_scripts(ctx: &AppContext, hook: &str) -> Result<Vec<PathBuf>, PyenvError> {
    if hook.trim().is_empty() {
        return Err(PyenvError::Io("Usage: pyenv hooks <command>".to_string()));
    }

    let mut scripts = Vec::new();
    for hook_root in hook_search_roots(ctx) {
        let hook_dir = hook_root.join(hook);
        if !hook_dir.is_dir() {
            continue;
        }

        let mut entries = std::fs::read_dir(&hook_dir)
            .map_err(io_error)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter(|path| is_supported_hook_script(path))
            .collect::<Vec<_>>();
        entries.sort_by_key(|path| {
            path.file_name()
                .map(|value| value.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default()
        });
        scripts.extend(entries.into_iter().map(resolve_hook_script_path));
    }

    Ok(scripts)
}

pub fn run_hook_scripts(
    ctx: &AppContext,
    hook: &str,
    extra_env: &[(&str, String)],
) -> Result<Vec<HookResult>, PyenvError> {
    let mut results = Vec::new();

    for script in discover_hook_scripts(ctx, hook)? {
        let mut env_pairs = vec![
            ("PYENV_HOOK", hook.to_string()),
            ("PYENV_COMMAND", hook.to_string()),
        ];
        env_pairs.extend(extra_env.iter().map(|(key, value)| (*key, value.clone())));

        let (exit_code, stdout, stderr) = run_process(&script, &[], ctx, &env_pairs, true)?;
        if exit_code != 0 {
            let detail = if !stderr.is_empty() {
                stderr.join("\n")
            } else {
                format!("exit code {exit_code}")
            };
            return Err(PyenvError::Io(format!(
                "pyenv: hook `{}` failed for {}: {detail}",
                script.display(),
                hook
            )));
        }

        results.push(HookResult {
            path: script,
            stdout,
        });
    }

    Ok(results)
}

pub fn collect_rehash_hook_names(
    ctx: &AppContext,
    extra_env: &[(&str, String)],
) -> Result<Vec<String>, PyenvError> {
    let mut names = HashSet::new();
    for result in run_hook_scripts(ctx, "rehash", extra_env)? {
        for line in parse_hook_actions(&result.stdout).passthrough_lines {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                names.insert(trimmed.to_string());
            }
        }
    }
    let mut values = names.into_iter().collect::<Vec<_>>();
    values.sort_by_key(|value| value.to_ascii_lowercase());
    Ok(values)
}

pub fn parse_hook_actions(lines: &[String]) -> HookActions {
    let mut actions = HookActions::default();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("PYENV_COMMAND_PATH=") {
            if !value.trim().is_empty() {
                actions.command_path = Some(PathBuf::from(value.trim()));
            }
            continue;
        }

        if let Some(value) = trimmed
            .strip_prefix("PATH+=")
            .or_else(|| trimmed.strip_prefix("PYENV_PATH+="))
        {
            if !value.trim().is_empty() {
                actions.prepend_paths.push(PathBuf::from(value.trim()));
            }
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("ENV:")
            && let Some((key, rest)) = value.split_once('=') {
                let key = key.trim();
                if !key.is_empty() {
                    actions
                        .env_pairs
                        .push((key.to_string(), rest.trim().to_string()));
                    continue;
                }
            }

        if let Some((key, value)) = parse_shell_assignment(trimmed) {
            if key.eq_ignore_ascii_case("PYENV_COMMAND_PATH") {
                if !value.trim().is_empty() {
                    actions.command_path = Some(PathBuf::from(value.trim()));
                }
                continue;
            }

            if key.eq_ignore_ascii_case("PATH") {
                actions.env_pairs.push((key.to_string(), value.to_string()));
                continue;
            }

            if key.starts_with("PYENV_") {
                actions.env_pairs.push((key.to_string(), value.to_string()));
                continue;
            }
        }

        actions.passthrough_lines.push(trimmed.to_string());
    }

    actions
}

pub fn find_plugin_command(ctx: &AppContext, command: &str) -> Option<PathBuf> {
    let plugin_name = format!("pyenv-{command}");
    for bin_dir in plugin_search_dirs(ctx) {
        if let Some(path) = search_path_entries(
            std::slice::from_ref(&bin_dir),
            &plugin_name,
            ctx.path_ext.as_deref(),
        ) {
            return Some(path);
        }

        for candidate in
            candidate_file_names(&plugin_name, Some(std::ffi::OsStr::new(".ps1;.sh;.bash")))
        {
            let path = bin_dir.join(candidate);
            if path.is_file() {
                return Some(path);
            }
        }
    }

    None
}

fn plugin_search_dirs(ctx: &AppContext) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(extra) = env::var_os("PYENV_PLUGIN_PATH") {
        roots.extend(env::split_paths(&extra));
    }

    roots.extend(default_plugin_bin_dirs(ctx));
    if let Some(path_env) = &ctx.path_env {
        roots.extend(env::split_paths(path_env));
    }
    dedup_paths(roots)
}

fn hook_search_roots(ctx: &AppContext) -> Vec<PathBuf> {
    hook_search_roots_with_extra(ctx, env::var_os("PYENV_HOOK_PATH"))
}

fn hook_search_roots_with_extra(ctx: &AppContext, extra: Option<OsString>) -> Vec<PathBuf> {
    let mut roots = extra
        .map(|extra| {
            env::split_paths(&extra)
                .map(resolve_relative_path)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    roots.extend(default_hook_roots(ctx));
    dedup_paths(roots)
}

fn default_plugin_bin_dirs(ctx: &AppContext) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    dirs.extend(collect_plugin_bins_under(&ctx.root.join("plugins")));

    for ancestor in exe_ancestor_roots(ctx) {
        dirs.extend(collect_plugin_bins_under(&ancestor.join("plugins")));
    }

    dirs
}

fn default_hook_roots(ctx: &AppContext) -> Vec<PathBuf> {
    let mut roots = vec![ctx.root.join("pyenv.d")];
    roots.extend(collect_plugin_hook_roots_under(&ctx.root.join("plugins")));
    roots.extend(sibling_hook_roots_for_bin_dirs(&plugin_search_dirs(ctx)));
    roots.extend(system_hook_roots());

    for ancestor in exe_ancestor_roots(ctx) {
        roots.push(ancestor.join("pyenv.d"));
        roots.extend(collect_plugin_hook_roots_under(&ancestor.join("plugins")));
    }

    roots
}

fn exe_ancestor_roots(ctx: &AppContext) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut current = ctx.exe_path.parent().map(Path::to_path_buf);
    for _ in 0..4 {
        let Some(path) = current.take() else {
            break;
        };
        roots.push(path.clone());
        current = path.parent().map(Path::to_path_buf);
    }
    roots
}

fn collect_plugin_bins_under(plugins_dir: &Path) -> Vec<PathBuf> {
    if !plugins_dir.is_dir() {
        return Vec::new();
    }

    std::fs::read_dir(plugins_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join("bin"))
        .filter(|path| path.is_dir())
        .collect()
}

fn collect_plugin_hook_roots_under(plugins_dir: &Path) -> Vec<PathBuf> {
    if !plugins_dir.is_dir() {
        return Vec::new();
    }

    std::fs::read_dir(plugins_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join("etc").join("pyenv.d"))
        .filter(|path| path.is_dir())
        .collect()
}

fn sibling_hook_roots_for_bin_dirs(bin_dirs: &[PathBuf]) -> Vec<PathBuf> {
    bin_dirs
        .iter()
        .filter_map(|bin_dir| {
            let plugin_root = bin_dir.parent()?;
            let hook_root = plugin_root.join("etc").join("pyenv.d");
            hook_root.is_dir().then_some(hook_root)
        })
        .collect()
}

fn resolve_hook_script_path(path: PathBuf) -> PathBuf {
    if cfg!(windows) {
        path
    } else {
        fs::canonicalize(&path).unwrap_or(path)
    }
}

fn dedup_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for path in paths {
        let normalized = normalize_path_key(&path);
        if seen.insert(normalized) {
            deduped.push(path);
        }
    }

    deduped
}

fn normalize_path_key(path: &Path) -> String {
    let resolved = resolve_relative_path(path.to_path_buf());
    if cfg!(windows) {
        resolved
            .to_string_lossy()
            .replace('/', "\\")
            .to_ascii_lowercase()
    } else {
        resolved.to_string_lossy().to_string()
    }
}

fn resolve_relative_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        env::current_dir()
            .map(|cwd| cwd.join(&path))
            .unwrap_or(path)
    }
}

fn system_hook_roots() -> Vec<PathBuf> {
    if cfg!(windows) {
        return Vec::new();
    }

    vec![
        PathBuf::from("/usr/etc/pyenv.d"),
        PathBuf::from("/usr/local/etc/pyenv.d"),
        PathBuf::from("/etc/pyenv.d"),
        PathBuf::from("/usr/lib/pyenv/hooks"),
    ]
}

fn parse_shell_assignment(line: &str) -> Option<(&str, &str)> {
    let candidate = line
        .strip_prefix("export ")
        .or_else(|| line.strip_prefix("setenv "))
        .unwrap_or(line)
        .trim();
    let (key, raw_value) = candidate.split_once('=')?;
    let key = key.trim();
    if key.is_empty()
        || !key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    Some((key, strip_assignment_quotes(raw_value.trim())))
}

fn strip_assignment_quotes(value: &str) -> &str {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let first = bytes.first().copied();
        let last = bytes.last().copied();
        if matches!(
            (first, last),
            (Some(b'"'), Some(b'"')) | (Some(b'\''), Some(b'\''))
        ) {
            return &value[1..value.len() - 1];
        }
    }
    value
}

fn is_supported_hook_script(path: &Path) -> bool {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("ps1" | "cmd" | "bat" | "exe" | "sh" | "bash") => true,
        None => true,
        _ => false,
    }
}

fn run_process(
    path: &Path,
    args: &[String],
    ctx: &AppContext,
    extra_env: &[(&str, String)],
    capture_output: bool,
) -> Result<(i32, Vec<String>, Vec<String>), PyenvError> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    let mut command = match extension.as_deref() {
        Some("ps1") => {
            let mut command = Command::new("powershell");
            command.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"]);
            command.arg(path);
            command
        }
        Some("cmd" | "bat") => {
            let mut command = Command::new("cmd");
            command.arg("/C");
            command.arg(path);
            command
        }
        Some("sh" | "bash") => {
            let mut command = Command::new(if extension.as_deref() == Some("bash") {
                "bash"
            } else {
                "sh"
            });
            command.arg(path);
            command
        }
        _ => Command::new(path),
    };

    command.args(args);
    command.current_dir(&ctx.dir);
    command.env("PYENV_ROOT", &ctx.root);
    command.env("PYENV_DIR", &ctx.dir);
    command.env("PYENV_EXE", &ctx.exe_path);
    if let Some(version) = &ctx.env_version {
        command.env("PYENV_VERSION", version);
    }
    if let Some(shell) = &ctx.env_shell {
        command.env("PYENV_SHELL", shell);
    }

    for (key, value) in extra_env {
        command.env(key, value);
    }

    if capture_output {
        let output = command.output().map_err(|error| {
            PyenvError::Io(format!("pyenv: failed to run {}: {error}", path.display()))
        })?;
        Ok((
            output.status.code().unwrap_or(1),
            split_lines(&String::from_utf8_lossy(&output.stdout)),
            split_lines(&String::from_utf8_lossy(&output.stderr)),
        ))
    } else {
        let status = command
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|error| {
                PyenvError::Io(format!("pyenv: failed to run {}: {error}", path.display()))
            })?;
        Ok((status.code().unwrap_or(1), Vec::new(), Vec::new()))
    }
}

fn split_lines(value: &str) -> Vec<String> {
    value.lines().map(ToOwned::to_owned).collect()
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

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::{
        cmd_hooks, complete_plugin_command, discover_hook_scripts, discover_plugin_commands,
        find_plugin_command, hook_search_roots_with_extra, parse_hook_actions, run_hook_scripts,
        system_hook_roots,
    };

    fn test_path_ext() -> Option<OsString> {
        if cfg!(windows) {
            Some(OsString::from(".exe;.cmd;.bat"))
        } else {
            None
        }
    }

    fn write_plugin_script(path: &PathBuf, body: &str) {
        fs::write(path, body).expect("plugin");
    }

    fn test_context() -> (TempDir, AppContext) {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        let dir = temp.path().join("work");
        let system_bin = temp.path().join("system-bin");
        fs::create_dir_all(root.join("plugins")).expect("plugins");
        fs::create_dir_all(&dir).expect("work");
        fs::create_dir_all(&system_bin).expect("system bin");

        let ctx = AppContext {
            root,
            dir,
            exe_path: std::path::PathBuf::from("pyenv"),
            env_version: None,
            env_shell: None,
            path_env: Some(env::join_paths([system_bin]).expect("path env")),
            path_ext: test_path_ext(),
            config: AppConfig::default(),
        };

        (temp, ctx)
    }

    #[test]
    fn finds_plugin_command_under_root_plugins() {
        let (_temp, ctx) = test_context();
        let plugin_bin = ctx.root.join("plugins").join("demo").join("bin");
        fs::create_dir_all(&plugin_bin).expect("plugin bin");
        let plugin_path = if cfg!(windows) {
            plugin_bin.join("pyenv-hello.cmd")
        } else {
            plugin_bin.join("pyenv-hello.sh")
        };
        write_plugin_script(
            &plugin_path,
            if cfg!(windows) {
                "@echo off\r\n"
            } else {
                "#!/usr/bin/env sh\n"
            },
        );

        let path = find_plugin_command(&ctx, "hello").expect("plugin path");
        assert_eq!(path, plugin_path);
    }

    #[test]
    fn plugin_commands_are_discovered_and_sorted() {
        let (_temp, ctx) = test_context();
        let plugin_bin = ctx.root.join("plugins").join("demo").join("bin");
        fs::create_dir_all(&plugin_bin).expect("plugin bin");
        if cfg!(windows) {
            fs::write(plugin_bin.join("pyenv-zeta.cmd"), "@echo off").expect("plugin");
            fs::write(plugin_bin.join("pyenv-alpha.ps1"), "Write-Output alpha").expect("plugin");
        } else {
            fs::write(plugin_bin.join("pyenv-zeta"), "#!/usr/bin/env sh\n").expect("plugin");
            fs::write(plugin_bin.join("pyenv-alpha.sh"), "#!/usr/bin/env sh\n").expect("plugin");
        }

        let commands = discover_plugin_commands(&ctx);
        assert_eq!(commands, vec!["alpha".to_string(), "zeta".to_string()]);
    }

    #[test]
    fn finds_plugin_commands_on_path_in_directories_with_spaces() {
        let (_temp, mut ctx) = test_context();
        let path_dir = ctx.root.join("path plugins");
        fs::create_dir_all(&path_dir).expect("path dir");
        let plugin_path = if cfg!(windows) {
            path_dir.join("pyenv-sh-hello.cmd")
        } else {
            path_dir.join("pyenv-sh-hello.sh")
        };
        write_plugin_script(
            &plugin_path,
            if cfg!(windows) {
                "@echo off\r\n"
            } else {
                "#!/usr/bin/env sh\n"
            },
        );
        let existing_path = ctx.path_env.clone().expect("path env");
        let mut joined = env::split_paths(&existing_path).collect::<Vec<_>>();
        joined.insert(0, path_dir.clone());
        ctx.path_env = Some(env::join_paths(joined).expect("join path"));

        let commands = discover_plugin_commands(&ctx);
        assert!(commands.iter().any(|command| command == "sh-hello"));

        let resolved = find_plugin_command(&ctx, "sh-hello").expect("plugin path");
        assert_eq!(resolved, plugin_path);
    }

    #[test]
    fn hooks_lists_sorted_supported_scripts() {
        let (_temp, ctx) = test_context();
        let hook_dir = ctx.root.join("pyenv.d").join("rehash");
        fs::create_dir_all(&hook_dir).expect("hook dir");
        fs::write(hook_dir.join("zeta.cmd"), "@echo off").expect("hook");
        fs::write(hook_dir.join("alpha.ps1"), "Write-Output alpha").expect("hook");
        fs::write(hook_dir.join("skip.txt"), "").expect("skip");

        let hooks = discover_hook_scripts(&ctx, "rehash").expect("hooks");
        assert_eq!(hooks.len(), 2);
        assert!(hooks[0].ends_with("alpha.ps1"));

        let report = cmd_hooks(&ctx, "rehash");
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout.len(), 2);

        let completion = cmd_hooks(&ctx, "--complete");
        assert_eq!(completion.exit_code, 0);
        assert!(completion.stdout.iter().any(|line| line == "rehash"));
    }

    #[test]
    fn hook_actions_parse_supported_directives() {
        let actions = parse_hook_actions(&[
            "PATH+=C:\\tools".to_string(),
            "ENV:DEMO=value".to_string(),
            "PYENV_COMMAND_PATH=C:\\demo\\python.exe".to_string(),
            "python".to_string(),
        ]);

        assert_eq!(
            actions.command_path,
            Some(PathBuf::from("C:\\demo\\python.exe"))
        );
        assert_eq!(actions.prepend_paths, vec![PathBuf::from("C:\\tools")]);
        assert_eq!(
            actions.env_pairs,
            vec![("DEMO".to_string(), "value".to_string())]
        );
        assert_eq!(actions.passthrough_lines, vec!["python".to_string()]);
    }

    #[test]
    fn hook_actions_parse_shell_style_assignments() {
        let actions = parse_hook_actions(&[
            "export PYENV_VERSION=3.12.6".to_string(),
            "PYENV_VERSION_ORIGIN=\"hook-origin\"".to_string(),
            "PATH=/tmp/demo".to_string(),
        ]);

        assert_eq!(
            actions.env_pairs,
            vec![
                ("PYENV_VERSION".to_string(), "3.12.6".to_string()),
                (
                    "PYENV_VERSION_ORIGIN".to_string(),
                    "hook-origin".to_string()
                ),
                ("PATH".to_string(), "/tmp/demo".to_string()),
            ]
        );
    }

    #[test]
    fn run_hook_scripts_executes_cmd_and_collects_output() {
        let (_temp, ctx) = test_context();
        let hook_dir = ctx.root.join("pyenv.d").join("rehash");
        fs::create_dir_all(&hook_dir).expect("hook dir");
        let hook_name = if cfg!(windows) {
            let path = hook_dir.join("alpha.cmd");
            fs::write(&path, "@echo one\r\n@echo two").expect("hook");
            path
        } else {
            let path = hook_dir.join("alpha.sh");
            fs::write(&path, "#!/usr/bin/env sh\necho one\necho two\n").expect("hook");
            path
        };

        let results = run_hook_scripts(&ctx, "rehash", &[]).expect("results");
        assert_eq!(results.len(), 1);
        let expected_path = if cfg!(windows) {
            hook_name
        } else {
            fs::canonicalize(hook_name).expect("canonical hook")
        };
        assert_eq!(results[0].path, expected_path);
        assert_eq!(
            results[0].stdout,
            vec!["one".to_string(), "two".to_string()]
        );
    }

    #[test]
    fn plugin_completion_runs_complete_mode() {
        let (_temp, ctx) = test_context();
        let plugin_bin = ctx.root.join("plugins").join("demo").join("bin");
        fs::create_dir_all(&plugin_bin).expect("plugin bin");
        if cfg!(windows) {
            fs::write(
                plugin_bin.join("pyenv-hello.cmd"),
                "@if \"%~1\"==\"--complete\" (\r\n@echo world\r\n@echo friend\r\n@exit /b 0\r\n)\r\n@exit /b 0\r\n",
            )
            .expect("plugin");
        } else {
            fs::write(
                plugin_bin.join("pyenv-hello.sh"),
                "#!/usr/bin/env sh\nif [ \"$1\" = \"--complete\" ]; then\n  echo world\n  echo friend\n  exit 0\nfi\nexit 0\n",
            )
            .expect("plugin");
        }

        let completions = complete_plugin_command(&ctx, "hello", &[String::from("he")])
            .expect("completion")
            .expect("plugin completions");
        assert_eq!(completions, vec!["world".to_string(), "friend".to_string()]);
    }

    #[test]
    fn hook_search_path_keeps_custom_and_default_roots() {
        let (_temp, ctx) = test_context();
        let default_hook_dir = ctx.root.join("pyenv.d").join("rehash");
        let custom_root = ctx.root.join("custom-hooks");
        let custom_hook_dir = custom_root.join("rehash");
        fs::create_dir_all(&default_hook_dir).expect("default hook dir");
        fs::create_dir_all(&custom_hook_dir).expect("custom hook dir");
        fs::write(default_hook_dir.join("beta.cmd"), "@echo beta").expect("default hook");
        fs::write(custom_hook_dir.join("alpha.cmd"), "@echo alpha").expect("custom hook");

        let roots = hook_search_roots_with_extra(&ctx, Some(custom_root.clone().into_os_string()));
        assert_eq!(roots[0], custom_root);
        assert!(roots.iter().any(|path| path == &ctx.root.join("pyenv.d")));
    }

    #[test]
    fn system_hook_roots_match_upstream_posix_locations() {
        if cfg!(windows) {
            assert!(system_hook_roots().is_empty());
        } else {
            let roots = system_hook_roots();
            assert!(
                roots
                    .iter()
                    .any(|path| path == &PathBuf::from("/etc/pyenv.d"))
            );
            assert!(
                roots
                    .iter()
                    .any(|path| path == &PathBuf::from("/usr/lib/pyenv/hooks"))
            );
        }
    }
}
