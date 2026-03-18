// ./crates/pyenv-core/src/manage.rs
//! Management commands for prefixes, installed-version listings, and uninstall operations.

use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::catalog::{compare_version_names, installed_version_names};
use crate::command::CommandReport;
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::executable::find_system_python_command;
use crate::plugin::run_hook_scripts;
use crate::runtime::{
    collect_shim_names_from_prefix, inventory_roots_for_version, managed_search_roots_for_version,
};
use crate::shim::rehash_shims;
use crate::version::{
    VersionOrigin, find_local_version_file, installed_version_dir, resolve_selected_versions,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VersionsCommandOptions {
    pub bare: bool,
    pub skip_aliases: bool,
    pub skip_envs: bool,
    pub executables: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VersionEntry {
    name: String,
    link_target: Option<PathBuf>,
}

pub fn cmd_prefix(ctx: &AppContext, versions: &[String]) -> CommandReport {
    let requested = if versions.is_empty() {
        let selected = resolve_selected_versions(ctx, false);
        if !selected.missing.is_empty() {
            let origin = selected.origin.to_string();
            let stderr = selected
                .missing
                .into_iter()
                .map(|version| PyenvError::VersionNotInstalled(version, origin.clone()).to_string())
                .collect();
            return CommandReport::failure(stderr, 1);
        }
        selected.versions
    } else {
        versions.to_vec()
    };

    let mut prefixes = Vec::new();
    for requested_version in requested {
        match resolve_prefix_path(ctx, &requested_version) {
            Ok(path) => prefixes.push(path),
            Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
        }
    }

    CommandReport::success_one(join_prefixes(&prefixes))
}

pub fn cmd_versions(ctx: &AppContext, options: &VersionsCommandOptions) -> CommandReport {
    if options.executables {
        return cmd_versions_executables(ctx);
    }

    let entries = match list_version_entries(ctx, options.skip_aliases, options.skip_envs) {
        Ok(entries) => entries,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };

    if options.bare {
        return CommandReport::success(entries.into_iter().map(|entry| entry.name).collect());
    }

    let origin = current_version_origin(ctx);
    let current = resolve_selected_versions(ctx, false)
        .versions
        .into_iter()
        .map(|value| value.to_ascii_lowercase())
        .collect::<HashSet<_>>();

    let mut stdout = Vec::new();
    let mut num_versions = 0usize;

    if system_prefix(ctx).is_some() {
        stdout.push(render_version_line(
            "system",
            None,
            current.contains("system"),
            &origin,
        ));
        num_versions += 1;
    }

    for entry in entries {
        stdout.push(render_version_line(
            &entry.name,
            entry.link_target.as_ref(),
            current.contains(&entry.name.to_ascii_lowercase()),
            &origin,
        ));
        num_versions += 1;
    }

    if num_versions == 0 {
        return CommandReport::failure(
            vec!["Warning: no Python detected on the system".to_string()],
            1,
        );
    }

    CommandReport::success(stdout)
}

pub fn cmd_uninstall(ctx: &AppContext, versions: &[String], force: bool) -> CommandReport {
    if versions.is_empty() {
        return CommandReport::failure(
            vec!["pyenv: uninstall operation requires at least one version argument".to_string()],
            1,
        );
    }

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut removed_any = false;

    for version in versions {
        if version.trim().is_empty() || version.starts_with('-') {
            stderr.push(format!("pyenv: invalid version argument `{version}`"));
            continue;
        }

        let version_dir = installed_version_dir(ctx, version);
        if !version_dir.exists() {
            if !force {
                stderr.push(format!("pyenv: version `{version}` not installed"));
            }
            continue;
        }

        if !force && !confirm_uninstall(&version_dir) {
            stderr.push(format!("pyenv: uninstall cancelled for `{version}`"));
            continue;
        }

        if let Err(error) = run_hook_scripts(
            ctx,
            "uninstall",
            &[
                ("PYENV_VERSION_NAME", version.to_string()),
                ("PYENV_VERSION", version.to_string()),
                ("PYENV_PREFIX", version_dir.display().to_string()),
                ("PYENV_HOOK_STAGE", "before".to_string()),
            ],
        ) {
            stderr.push(error.to_string());
            continue;
        }

        match fs::remove_dir_all(&version_dir) {
            Ok(_) => {
                removed_any = true;
                stdout.push(format!("pyenv: {version} uninstalled"));
                if let Err(error) = run_hook_scripts(
                    ctx,
                    "uninstall",
                    &[
                        ("PYENV_VERSION_NAME", version.to_string()),
                        ("PYENV_VERSION", version.to_string()),
                        ("PYENV_PREFIX", version_dir.display().to_string()),
                        ("PYENV_HOOK_STAGE", "after".to_string()),
                    ],
                ) {
                    stderr.push(error.to_string());
                }
            }
            Err(error) => stderr.push(format!(
                "pyenv: failed to remove {}: {error}",
                version_dir.display()
            )),
        }
    }

    if removed_any && let Err(error) = rehash_shims(ctx) {
        stderr.push(error.to_string());
    }

    let exit_code = if stderr.is_empty() { 0 } else { 1 };

    CommandReport {
        stdout,
        stderr,
        exit_code,
    }
}

fn cmd_versions_executables(ctx: &AppContext) -> CommandReport {
    let versions = match installed_version_names(ctx) {
        Ok(versions) => versions,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };

    let mut names = HashSet::new();
    for version in versions {
        for prefix in inventory_roots_for_version(ctx, &version) {
            match collect_shim_names_from_prefix(&prefix, ctx.path_ext.as_deref()) {
                Ok(found) => names.extend(found),
                Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
            }
        }
    }

    let mut stdout = names.into_iter().collect::<Vec<_>>();
    stdout.sort_by_key(|value| value.to_ascii_lowercase());
    CommandReport::success(stdout)
}

fn resolve_prefix_path(ctx: &AppContext, version: &str) -> Result<PathBuf, PyenvError> {
    if version == "system" {
        return system_prefix(ctx)
            .ok_or_else(|| PyenvError::Io("pyenv: system version not found in PATH".to_string()));
    }

    for prefix in managed_search_roots_for_version(ctx, version) {
        if prefix.is_dir() {
            return Ok(prefix);
        }
    }

    if let Some(resolved) = crate::catalog::latest_installed_version(ctx, version) {
        for prefix in managed_search_roots_for_version(ctx, &resolved) {
            if prefix.is_dir() {
                return Ok(prefix);
            }
        }
        return Err(PyenvError::Io(format!(
            "pyenv: version `{resolved}` not installed"
        )));
    }

    Err(PyenvError::Io(format!(
        "pyenv: version `{version}` not installed"
    )))
}

fn join_prefixes(prefixes: &[PathBuf]) -> String {
    let separator = if cfg!(windows) { ";" } else { ":" };
    prefixes
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(separator)
}

fn system_prefix(ctx: &AppContext) -> Option<PathBuf> {
    let python_path = find_system_python_command(ctx)?;
    system_prefix_from_python(&python_path)
}

fn system_prefix_from_python(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let parent_name = parent.file_name()?.to_string_lossy().to_ascii_lowercase();
    if matches!(parent_name.as_str(), "bin" | "sbin" | "scripts") {
        let prefix = parent
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(parent));
        if prefix.as_os_str().is_empty() {
            Some(PathBuf::from(std::path::MAIN_SEPARATOR.to_string()))
        } else {
            Some(prefix)
        }
    } else {
        Some(parent.to_path_buf())
    }
}

fn current_version_origin(ctx: &AppContext) -> String {
    if ctx.env_version.is_some() {
        VersionOrigin::Environment.to_string()
    } else if let Some(local_file) = find_local_version_file(&ctx.dir) {
        VersionOrigin::File(local_file).to_string()
    } else {
        VersionOrigin::File(ctx.root.join("version")).to_string()
    }
}

fn render_version_line(
    name: &str,
    link_target: Option<&PathBuf>,
    current: bool,
    origin: &str,
) -> String {
    let repr = link_target
        .map(|target| format!("{name} --> {}", target.display()))
        .unwrap_or_else(|| name.to_string());

    if current {
        format!("* {repr} (set by {origin})")
    } else {
        format!("  {repr}")
    }
}

fn list_version_entries(
    ctx: &AppContext,
    skip_aliases: bool,
    skip_envs: bool,
) -> Result<Vec<VersionEntry>, PyenvError> {
    let versions_dir = ctx.versions_dir();
    if !versions_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(&versions_dir)
        .map_err(io_error)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            path.is_dir()
                .then(|| entry.file_name().to_string_lossy().to_string())
        })
        .collect::<Vec<_>>();
    entries.sort_by(|lhs, rhs| compare_version_names(lhs, rhs));

    let versions_dir_canonical = fs::canonicalize(&versions_dir).unwrap_or(versions_dir.clone());
    let mut results = Vec::new();

    for version in entries {
        let path = versions_dir.join(&version);
        let metadata = fs::symlink_metadata(&path).map_err(io_error)?;
        let link_target = if metadata.file_type().is_symlink() {
            fs::read_link(&path).ok()
        } else {
            None
        };

        if skip_aliases
            && metadata.file_type().is_symlink()
            && fs::canonicalize(&path)
                .ok()
                .is_some_and(|target| target.starts_with(&versions_dir_canonical))
        {
            continue;
        }

        results.push(VersionEntry {
            name: version.clone(),
            link_target,
        });

        if skip_envs {
            continue;
        }

        let envs_dir = path.join("envs");
        if !envs_dir.is_dir() {
            continue;
        }

        let mut env_entries = fs::read_dir(&envs_dir)
            .map_err(io_error)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let env_path = entry.path();
                env_path
                    .is_dir()
                    .then(|| entry.file_name().to_string_lossy().to_string())
            })
            .collect::<Vec<_>>();
        env_entries.sort();

        for env_name in env_entries {
            let env_path = envs_dir.join(&env_name);
            let env_metadata = fs::symlink_metadata(&env_path).map_err(io_error)?;
            let env_link_target = if env_metadata.file_type().is_symlink() {
                fs::read_link(&env_path).ok()
            } else {
                None
            };
            results.push(VersionEntry {
                name: format!("{version}/envs/{env_name}"),
                link_target: env_link_target,
            });
        }
    }

    results.sort_by(|lhs, rhs| compare_version_names(&lhs.name, &rhs.name));
    Ok(results)
}

fn confirm_uninstall(prefix: &Path) -> bool {
    let _ = write!(io::stdout(), "pyenv: remove {}? (y/N) ", prefix.display());
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

#[cfg(test)]
mod tests {
    use std::env;
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::{VersionsCommandOptions, cmd_prefix, cmd_uninstall, cmd_versions};

    fn test_path_ext() -> Option<OsString> {
        if cfg!(windows) {
            Some(OsString::from(".exe;.cmd;.bat"))
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
        let system_bin = temp.path().join("system");
        fs::create_dir_all(root.join("versions")).expect("versions");
        fs::create_dir_all(&dir).expect("dir");
        fs::create_dir_all(&system_bin).expect("system");

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
    fn prefix_resolves_selected_version_directory() {
        let (_temp, mut ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.6");
        fs::create_dir_all(&version_dir).expect("version");
        ctx.env_version = Some("3.12".to_string());

        let report = cmd_prefix(&ctx, &[]);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec![version_dir.display().to_string()]);
    }

    #[test]
    fn versions_marks_current_and_lists_envs() {
        let (_temp, mut ctx) = test_context();
        fs::write(
            PathBuf::from(ctx.path_env.clone().expect("path env")).join(command_file("python")),
            "",
        )
        .expect("system python");
        fs::create_dir_all(ctx.versions_dir().join("3.12.6").join("envs").join("demo"))
            .expect("env");
        fs::create_dir_all(ctx.versions_dir().join("3.13.2")).expect("version");
        ctx.env_version = Some("3.12.6".to_string());

        let report = cmd_versions(&ctx, &VersionsCommandOptions::default());
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout[0], "  system");
        assert!(
            report
                .stdout
                .iter()
                .any(|line| line.starts_with("* 3.12.6 "))
        );
        assert!(
            report
                .stdout
                .iter()
                .any(|line| line.trim() == "3.12.6/envs/demo")
        );
    }

    #[test]
    fn versions_executables_are_deduplicated() {
        let (_temp, ctx) = test_context();
        if cfg!(windows) {
            fs::create_dir_all(ctx.versions_dir().join("3.12.6").join("Scripts")).expect("scripts");
            fs::create_dir_all(ctx.versions_dir().join("3.13.2").join("Scripts")).expect("scripts");
            fs::write(ctx.versions_dir().join("3.12.6").join("python.exe"), "").expect("python");
            fs::write(
                ctx.versions_dir()
                    .join("3.12.6")
                    .join("Scripts")
                    .join("pip.cmd"),
                "",
            )
            .expect("pip");
            fs::write(ctx.versions_dir().join("3.13.2").join("python.exe"), "").expect("python");
        } else {
            fs::create_dir_all(ctx.versions_dir().join("3.12.6").join("bin")).expect("bin");
            fs::create_dir_all(ctx.versions_dir().join("3.13.2").join("bin")).expect("bin");
            fs::write(
                ctx.versions_dir().join("3.12.6").join("bin").join("python"),
                "",
            )
            .expect("python");
            fs::write(
                ctx.versions_dir().join("3.12.6").join("bin").join("pip"),
                "",
            )
            .expect("pip");
            fs::write(
                ctx.versions_dir().join("3.13.2").join("bin").join("python"),
                "",
            )
            .expect("python");
        }

        let report = cmd_versions(
            &ctx,
            &VersionsCommandOptions {
                executables: true,
                ..VersionsCommandOptions::default()
            },
        );

        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec!["pip".to_string(), "python".to_string()]);
    }

    #[test]
    fn uninstall_force_removes_version_directory() {
        let (_temp, ctx) = test_context();
        let version_dir = ctx.versions_dir().join("3.12.6");
        if cfg!(windows) {
            fs::create_dir_all(version_dir.join("Scripts")).expect("version");
            fs::write(version_dir.join("python.exe"), "").expect("python");
        } else {
            fs::create_dir_all(version_dir.join("bin")).expect("version");
            fs::write(version_dir.join("bin").join("python"), "").expect("python");
        }

        let report = cmd_uninstall(&ctx, &[String::from("3.12.6")], true);
        assert_eq!(report.exit_code, 0);
        assert!(
            report
                .stdout
                .iter()
                .any(|line| line.contains("3.12.6 uninstalled"))
        );
        assert!(!version_dir.exists());
    }
}
