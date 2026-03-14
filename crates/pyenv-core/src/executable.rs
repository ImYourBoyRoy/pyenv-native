// ./crates/pyenv-core/src/executable.rs
//! Executable discovery for `which` and `whence` across managed runtimes and the system path.

use std::env;
use std::path::{Path, PathBuf};

use crate::catalog::installed_version_names;
use crate::command::CommandReport;
use crate::context::AppContext;
use crate::plugin::{parse_hook_actions, run_hook_scripts};
use crate::runtime::{find_command_in_prefix, search_path_entries};
use crate::version::resolve_selected_versions;

pub fn cmd_which(
    ctx: &AppContext,
    command: &str,
    no_system: bool,
    skip_advice: bool,
) -> CommandReport {
    let selected = resolve_selected_versions(ctx, false);
    let origin = selected.origin.to_string();
    let mut searched_system = false;
    let selected_value = selected.versions.join(":");
    let mut resolved_version_name = None;
    let mut found_path = None;

    for version in &selected.versions {
        if version == "system" {
            if no_system {
                continue;
            }

            searched_system = true;
            if let Some(path) = find_system_command(ctx, command) {
                resolved_version_name = Some("system".to_string());
                found_path = Some(path);
                break;
            }
            continue;
        }

        if let Some(path) = find_command_in_version(ctx, version, command) {
            resolved_version_name = Some(version.clone());
            found_path = Some(path);
            break;
        }
    }

    if found_path.is_none() && !no_system && !searched_system {
        if let Some(path) = find_system_command(ctx, command) {
            resolved_version_name = Some("system".to_string());
            found_path = Some(path);
        }
    }

    let hook_results = match run_hook_scripts(
        ctx,
        "which",
        &[
            ("PYENV_COMMAND", command.to_string()),
            (
                "PYENV_COMMAND_PATH",
                found_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
            ),
            ("PYENV_VERSION", selected_value),
            (
                "PYENV_VERSION_RESOLVED",
                resolved_version_name
                    .clone()
                    .unwrap_or_else(|| "system".to_string()),
            ),
            ("PYENV_VERSION_ORIGIN", origin.clone()),
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
    if hook_actions.command_path.is_some() {
        found_path = hook_actions.command_path;
    }

    if let Some(path) = found_path.as_ref().filter(|path| path.is_file()) {
        return CommandReport::success_one(path.display().to_string());
    }

    let mut stderr = selected
        .missing
        .into_iter()
        .map(|version| format!("pyenv: version `{version}' is not installed (set by {origin})"))
        .collect::<Vec<_>>();
    stderr.push(format!("pyenv: {command}: command not found"));

    if !skip_advice {
        let advice = collect_whence(ctx, command, false);
        if !advice.is_empty() {
            stderr.push(String::new());
            stderr.push(format!(
                "The `{command}' command exists in these Python versions:"
            ));
            stderr.extend(advice.into_iter().map(|version| format!("  {version}")));
            stderr.push(String::new());
            stderr.push("Note: See 'pyenv help global' for tips on allowing both".to_string());
            stderr.push("      python2 and python3 to be found.".to_string());
        }
    }

    CommandReport::failure(stderr, 127)
}

pub fn cmd_whence(ctx: &AppContext, command: &str, print_paths: bool) -> CommandReport {
    let matches = collect_whence(ctx, command, print_paths);
    if matches.is_empty() {
        CommandReport::failure(Vec::new(), 1)
    } else {
        CommandReport::success(matches)
    }
}

fn collect_whence(ctx: &AppContext, command: &str, print_paths: bool) -> Vec<String> {
    let versions = match installed_version_names(ctx) {
        Ok(versions) => versions,
        Err(_) => return Vec::new(),
    };

    versions
        .into_iter()
        .filter_map(|version| {
            let path = find_command_in_version(ctx, &version, command)?;
            Some(if print_paths {
                path.display().to_string()
            } else {
                version
            })
        })
        .collect()
}

pub(crate) fn find_system_command(ctx: &AppContext, command: &str) -> Option<PathBuf> {
    let mut path_entries = ctx
        .path_env
        .as_ref()
        .map(env::split_paths)
        .into_iter()
        .flatten()
        .filter(|entry| !entry.as_os_str().is_empty())
        .collect::<Vec<_>>();

    let mut removal_targets = vec![ctx.shims_dir()];
    if let Some(extra_paths) = env::var_os(program_specific_shim_paths_env(command)) {
        removal_targets.extend(env::split_paths(&extra_paths));
    }
    path_entries.retain(|entry| {
        !removal_targets
            .iter()
            .any(|target| paths_equal(entry, target))
    });

    search_path_entries(&path_entries, command, ctx.path_ext.as_deref())
}

pub(crate) fn find_system_python_command(ctx: &AppContext) -> Option<PathBuf> {
    for command in ["python", "python3", "python2"] {
        if let Some(path) = find_system_command(ctx, command) {
            return Some(path);
        }
    }
    None
}

pub(crate) fn find_command_in_version(
    ctx: &AppContext,
    version: &str,
    command: &str,
) -> Option<PathBuf> {
    for prefix in crate::runtime::managed_search_roots_for_version(ctx, version) {
        if let Some(path) = find_command_in_prefix(&prefix, command, ctx.path_ext.as_deref()) {
            return Some(path);
        }
    }
    None
}

fn program_specific_shim_paths_env(command: &str) -> String {
    let sanitized = command
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("_PYENV_SHIM_PATHS_{sanitized}")
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

#[cfg(test)]
mod tests {
    use std::env;
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::{cmd_whence, cmd_which, find_command_in_version};

    fn test_path_ext() -> Option<OsString> {
        if cfg!(windows) {
            Some(OsString::from(".exe;.bat;.cmd"))
        } else {
            None
        }
    }

    fn command_file(name: &str) -> String {
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
        let system_bin = temp.path().join("system-bin");
        fs::create_dir_all(root.join("versions")).expect("versions dir");
        fs::create_dir_all(&dir).expect("work dir");
        fs::create_dir_all(&system_bin).expect("system bin");

        let ctx = AppContext {
            root,
            dir,
            exe_path: PathBuf::from("pyenv"),
            env_version: None,
            env_shell: None,
            path_env: Some(env::join_paths([system_bin.clone()]).expect("path env")),
            path_ext: test_path_ext(),
            config: AppConfig::default(),
        };

        (temp, ctx)
    }

    #[test]
    fn which_finds_version_root_and_scripts_commands() {
        let (_temp, mut ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.1");
        let python_path = if cfg!(windows) {
            fs::create_dir_all(version_dir.join("Scripts")).expect("scripts");
            let path = version_dir.join("python.exe");
            fs::write(&path, "").expect("python");
            fs::write(version_dir.join("Scripts").join("pip.exe"), "").expect("pip");
            path
        } else {
            fs::create_dir_all(version_dir.join("bin")).expect("bin");
            let path = version_dir.join("bin").join("python");
            fs::write(&path, "").expect("python");
            fs::write(version_dir.join("bin").join("pip"), "").expect("pip");
            path
        };
        let pip_path = if cfg!(windows) {
            version_dir.join("Scripts").join("pip.exe")
        } else {
            version_dir.join("bin").join("pip")
        };
        ctx.env_version = Some("3.12.1".to_string());

        let python_report = cmd_which(&ctx, "python", false, false);
        assert_eq!(python_report.exit_code, 0);
        assert_eq!(PathBuf::from(&python_report.stdout[0]), python_path);

        let pip_report = cmd_which(&ctx, "pip", false, false);
        assert_eq!(pip_report.exit_code, 0);
        assert_eq!(PathBuf::from(&pip_report.stdout[0]), pip_path);

        let python = find_command_in_version(&ctx, "3.12.1", "python").expect("python");
        assert_eq!(python, python_path);
    }

    #[test]
    fn which_falls_back_to_system_path_without_shims() {
        let (_temp, ctx) = test_context();
        let system_path = PathBuf::from(ctx.path_env.clone().expect("path env"));
        let ruff_path = system_path.join(command_file("ruff"));
        fs::write(&ruff_path, "").expect("ruff");

        let report = cmd_which(&ctx, "ruff", false, false);
        assert_eq!(report.exit_code, 0);
        assert_eq!(PathBuf::from(&report.stdout[0]), ruff_path);
    }

    #[test]
    fn which_can_skip_system_lookup() {
        let (_temp, ctx) = test_context();
        let system_path = PathBuf::from(ctx.path_env.clone().expect("path env"));
        fs::write(system_path.join(command_file("ruff")), "").expect("ruff");

        let report = cmd_which(&ctx, "ruff", true, false);
        assert_eq!(report.exit_code, 127);
        assert!(
            report
                .stderr
                .iter()
                .any(|line| line.contains("command not found"))
        );
    }

    #[test]
    fn whence_lists_versions_in_ascending_order() {
        let (_temp, ctx) = test_context();
        for version in ["2.7", "3.4"] {
            let version_dir = if cfg!(windows) {
                ctx.versions_dir().join(version)
            } else {
                ctx.versions_dir().join(version).join("bin")
            };
            fs::create_dir_all(&version_dir).expect("bin");
            fs::write(version_dir.join(command_file("python")), "").expect("python");
        }

        let report = cmd_whence(&ctx, "python", false);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec!["2.7".to_string(), "3.4".to_string()]);
    }

    #[test]
    fn which_reports_advice_from_other_versions() {
        let (_temp, mut ctx) = test_context();
        let version_dir = if cfg!(windows) {
            ctx.versions_dir().join("3.4")
        } else {
            ctx.versions_dir().join("3.4").join("bin")
        };
        fs::create_dir_all(&version_dir).expect("bin");
        fs::write(version_dir.join("py.test"), "").expect("py.test");
        ctx.env_version = Some("2.7".to_string());

        let report = cmd_which(&ctx, "py.test", false, false);
        assert_eq!(report.exit_code, 127);
        assert!(
            report
                .stderr
                .iter()
                .any(|line| line.contains("version `2.7' is not installed"))
        );
        assert!(report
            .stderr
            .iter()
            .any(|line| line.contains("The `py.test' command exists in these Python versions:")));
        assert!(report.stderr.iter().any(|line| line.trim() == "3.4"));
    }

    #[test]
    fn which_skip_advice_suppresses_other_version_hints() {
        let (_temp, mut ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.4").join("bin");
        fs::create_dir_all(&version_dir).expect("bin");
        fs::write(version_dir.join("py.test"), "").expect("py.test");
        ctx.env_version = Some("2.7".to_string());

        let report = cmd_which(&ctx, "py.test", false, true);
        assert_eq!(report.exit_code, 127);
        assert_eq!(
            report.stderr,
            vec![
                "pyenv: version `2.7' is not installed (set by PYENV_VERSION environment variable)"
                    .to_string(),
                "pyenv: py.test: command not found".to_string(),
            ]
        );
        assert!(
            report
                .stderr
                .iter()
                .all(|line| !line.contains("exists in these Python versions"))
        );
    }

    #[test]
    fn which_hook_can_override_command_path() {
        let (_temp, mut ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.1");
        let hook_dir = ctx.root.join("pyenv.d").join("which");
        let override_path = if cfg!(windows) {
            ctx.root.join("override.exe")
        } else {
            ctx.root.join("override")
        };
        fs::create_dir_all(&version_dir).expect("version");
        fs::create_dir_all(&hook_dir).expect("hook dir");
        if cfg!(windows) {
            fs::write(version_dir.join("python.exe"), "").expect("python");
            fs::write(
                hook_dir.join("override.cmd"),
                format!("@echo PYENV_COMMAND_PATH={}", override_path.display()),
            )
            .expect("hook");
        } else {
            fs::write(version_dir.join("python"), "").expect("python");
            fs::write(
                hook_dir.join("override.sh"),
                format!(
                    "#!/usr/bin/env sh\necho PYENV_COMMAND_PATH={}\n",
                    override_path.display()
                ),
            )
            .expect("hook");
        }
        fs::write(&override_path, "").expect("override");
        ctx.env_version = Some("3.12.1".to_string());

        let report = cmd_which(&ctx, "python", false, false);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec![override_path.display().to_string()]);
    }
}
