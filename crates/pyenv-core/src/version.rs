// ./crates/pyenv-core/src/version.rs
//! Version file discovery, selection, and related command implementations.

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::catalog::latest_installed_version;
use crate::command::CommandReport;
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::plugin::{parse_hook_actions, run_hook_scripts};

const LOCAL_VERSION_FILE: &str = ".python-version";
const GLOBAL_VERSION_FILE: &str = "version";

#[derive(Debug)]
struct ParsedVersionFile {
    versions: Vec<String>,
    warnings: Vec<PyenvError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionOrigin {
    Environment,
    File(PathBuf),
}

impl std::fmt::Display for VersionOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Environment => write!(f, "PYENV_VERSION environment variable"),
            Self::File(path) => write!(f, "{}", path.display()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedVersions {
    pub versions: Vec<String>,
    pub missing: Vec<String>,
    pub origin: VersionOrigin,
}

pub fn find_local_version_file(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();

    loop {
        let candidate = current.join(LOCAL_VERSION_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }

        if !current.pop() {
            return None;
        }
    }
}

pub fn version_file_path(ctx: &AppContext, target_dir: Option<&Path>) -> PathBuf {
    if let Some(target_dir) = target_dir {
        find_local_version_file(target_dir).unwrap_or_else(|| ctx.root.join(GLOBAL_VERSION_FILE))
    } else {
        find_local_version_file(&ctx.dir).unwrap_or_else(|| ctx.root.join(GLOBAL_VERSION_FILE))
    }
}

pub fn read_version_file(path: &Path) -> Result<Vec<String>, Vec<PyenvError>> {
    parse_version_file(path).map(|parsed| parsed.versions)
}

fn parse_version_file(path: &Path) -> Result<ParsedVersionFile, Vec<PyenvError>> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(_) => return Err(Vec::new()),
    };

    let mut versions = Vec::new();
    let mut errors = Vec::new();

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some(version) = trimmed.split_whitespace().next() else {
            continue;
        };

        if is_version_safe(version) {
            versions.push(version.to_string());
        } else {
            errors.push(PyenvError::InvalidVersion(
                version.to_string(),
                path.display().to_string(),
            ));
        }
    }

    if versions.is_empty() {
        Err(errors)
    } else {
        Ok(ParsedVersionFile {
            versions,
            warnings: errors,
        })
    }
}

fn is_version_safe(version: &str) -> bool {
    let path = Path::new(version);
    if path.is_absolute() {
        return false;
    }

    let mut depth = 0usize;
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(_) => depth += 1,
            Component::ParentDir => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }

    true
}

fn parse_env_versions(value: &str) -> Vec<String> {
    value
        .split(':')
        .flat_map(|segment| segment.split_whitespace())
        .filter(|segment| !segment.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn version_origin(ctx: &AppContext) -> VersionOrigin {
    if ctx.env_version.is_some() {
        VersionOrigin::Environment
    } else {
        VersionOrigin::File(version_file_path(ctx, None))
    }
}

pub fn installed_version_dir(ctx: &AppContext, version: &str) -> PathBuf {
    ctx.versions_dir().join(version)
}

fn version_exists(ctx: &AppContext, version: &str) -> bool {
    version == "system" || installed_version_dir(ctx, version).is_dir()
}

fn normalize_version_name(version: &str) -> String {
    version
        .strip_prefix("python-")
        .unwrap_or(version)
        .to_string()
}

pub fn resolve_selected_versions(ctx: &AppContext, force: bool) -> SelectedVersions {
    let raw_versions = if let Some(env_version) = &ctx.env_version {
        parse_env_versions(env_version)
    } else {
        let version_file = version_file_path(ctx, None);
        match read_version_file(&version_file) {
            Ok(versions) => versions,
            Err(_) => vec!["system".to_string()],
        }
    };

    if raw_versions.is_empty() {
        return SelectedVersions {
            versions: vec!["system".to_string()],
            missing: Vec::new(),
            origin: version_origin(ctx),
        };
    }

    let origin = version_origin(ctx);
    let mut versions = Vec::new();
    let mut missing = Vec::new();

    for raw_version in raw_versions {
        let normalized = normalize_version_name(&raw_version);
        if version_exists(ctx, &raw_version) {
            versions.push(raw_version);
        } else if version_exists(ctx, &normalized) {
            versions.push(normalized);
        } else if let Some(resolved) = latest_installed_version(ctx, &raw_version) {
            versions.push(resolved);
        } else if let Some(resolved) = latest_installed_version(ctx, &normalized) {
            versions.push(resolved);
        } else if force {
            versions.push(normalized);
        } else {
            missing.push(raw_version);
        }
    }

    if versions.is_empty() && missing.is_empty() {
        versions.push("system".to_string());
    }

    SelectedVersions {
        versions,
        missing,
        origin,
    }
}

fn ensure_versions_exist(
    ctx: &AppContext,
    versions: &[String],
    force: bool,
    origin: &str,
) -> Result<(), PyenvError> {
    for version in versions {
        let normalized = normalize_version_name(version);
        if force
            || version_exists(ctx, version)
            || version_exists(ctx, &normalized)
            || latest_installed_version(ctx, version).is_some()
            || latest_installed_version(ctx, &normalized).is_some()
        {
            continue;
        }

        return Err(PyenvError::VersionNotInstalled(
            version.clone(),
            origin.to_string(),
        ));
    }

    Ok(())
}

fn write_version_file(path: &Path, versions: &[String]) -> Result<(), PyenvError> {
    if versions.is_empty() {
        return Err(PyenvError::Io("pyenv: no versions specified".to_string()));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| PyenvError::Io(format!("pyenv: {error}")))?;
    }

    let mut contents = versions.join("\n");
    contents.push('\n');
    fs::write(path, contents).map_err(|error| PyenvError::Io(format!("pyenv: {error}")))?;
    Ok(())
}

pub fn cmd_root(ctx: &AppContext) -> CommandReport {
    CommandReport::success_one(ctx.root.display().to_string())
}

pub fn cmd_version_file(ctx: &AppContext, target_dir: Option<&Path>) -> CommandReport {
    CommandReport::success_one(version_file_path(ctx, target_dir).display().to_string())
}

pub fn cmd_version_file_write(
    ctx: &AppContext,
    path: &Path,
    versions: &[String],
    force: bool,
) -> CommandReport {
    if versions.is_empty() {
        return CommandReport::failure(
            vec!["Usage: pyenv version-file-write [-f|--force] <file> <version> [...]".to_string()],
            1,
        );
    }

    match ensure_versions_exist(ctx, versions, force, &path.display().to_string())
        .and_then(|_| write_version_file(path, versions))
    {
        Ok(_) => CommandReport::empty_success(),
        Err(error) => CommandReport::failure(vec![error.to_string()], 1),
    }
}

pub fn cmd_version_file_read(path: &Path) -> CommandReport {
    match parse_version_file(path) {
        Ok(parsed) => CommandReport {
            stdout: vec![parsed.versions.join(":")],
            stderr: parsed
                .warnings
                .into_iter()
                .filter_map(|error| {
                    let message = error.to_string();
                    if message.is_empty() {
                        None
                    } else {
                        Some(message)
                    }
                })
                .collect(),
            exit_code: 0,
        },
        Err(errors) => {
            let stderr = errors
                .into_iter()
                .filter_map(|error| {
                    let message = error.to_string();
                    if message.is_empty() {
                        None
                    } else {
                        Some(message)
                    }
                })
                .collect::<Vec<_>>();

            CommandReport {
                stdout: Vec::new(),
                stderr,
                exit_code: 1,
            }
        }
    }
}

pub fn cmd_version_origin(ctx: &AppContext) -> CommandReport {
    let default_origin = version_origin(ctx).to_string();
    let hook_results = match run_hook_scripts(
        ctx,
        "version-origin",
        &[("PYENV_VERSION_ORIGIN", default_origin.clone())],
    ) {
        Ok(results) => results,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };
    let actions = parse_hook_actions(
        &hook_results
            .into_iter()
            .flat_map(|result| result.stdout)
            .collect::<Vec<_>>(),
    );
    let origin = actions
        .env_pairs
        .into_iter()
        .find_map(|(key, value)| {
            key.eq_ignore_ascii_case("PYENV_VERSION_ORIGIN")
                .then_some(value)
        })
        .or_else(|| {
            actions
                .passthrough_lines
                .into_iter()
                .find(|line| !line.is_empty())
        })
        .unwrap_or(default_origin);
    CommandReport::success_one(origin)
}

pub fn cmd_version_name(ctx: &AppContext, force: bool) -> CommandReport {
    let mut selected = resolve_selected_versions(ctx, force);
    let origin = selected.origin.to_string();
    let selected_value = selected.versions.join(":");
    let hook_results = match run_hook_scripts(
        ctx,
        "version-name",
        &[
            ("PYENV_VERSION", selected_value.clone()),
            ("PYENV_VERSION_ORIGIN", origin.clone()),
        ],
    ) {
        Ok(results) => results,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };
    let actions = parse_hook_actions(
        &hook_results
            .into_iter()
            .flat_map(|result| result.stdout)
            .collect::<Vec<_>>(),
    );
    if let Some(overridden) = actions
        .env_pairs
        .into_iter()
        .find_map(|(key, value)| key.eq_ignore_ascii_case("PYENV_VERSION").then_some(value))
        .or_else(|| {
            actions
                .passthrough_lines
                .into_iter()
                .find(|line| !line.is_empty())
        })
    {
        let raw = parse_env_versions(&overridden);
        if !raw.is_empty() {
            selected.versions = raw;
            selected.missing.clear();
        }
    }

    let stderr = selected
        .missing
        .iter()
        .map(|version| PyenvError::VersionNotInstalled(version.clone(), origin.clone()).to_string())
        .collect::<Vec<_>>();

    CommandReport {
        stdout: vec![selected.versions.join(":")],
        stderr,
        exit_code: if selected.missing.is_empty() { 0 } else { 1 },
    }
}

pub fn cmd_version(ctx: &AppContext, bare: bool) -> CommandReport {
    let selected = resolve_selected_versions(ctx, false);
    let origin = selected.origin.to_string();
    let stderr = selected
        .missing
        .iter()
        .map(|version| PyenvError::VersionNotInstalled(version.clone(), origin.clone()).to_string())
        .collect::<Vec<_>>();

    let stdout = if bare {
        selected.versions.clone()
    } else {
        selected
            .versions
            .iter()
            .map(|version| format!("{version} (set by {origin})"))
            .collect()
    };

    CommandReport {
        stdout,
        stderr,
        exit_code: if selected.missing.is_empty() { 0 } else { 1 },
    }
}

pub fn cmd_global(ctx: &AppContext, versions: &[String], unset: bool) -> CommandReport {
    let path = ctx.root.join(GLOBAL_VERSION_FILE);

    if unset {
        match fs::remove_file(&path) {
            Ok(_) => CommandReport::empty_success(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                CommandReport::empty_success()
            }
            Err(error) => CommandReport::failure(vec![format!("pyenv: {error}")], 1),
        }
    } else if versions.is_empty() {
        for fallback in ["version", "global", "default"] {
            let candidate = ctx.root.join(fallback);
            if let Ok(found_versions) = read_version_file(&candidate) {
                return CommandReport::success(found_versions);
            }
        }

        CommandReport::success(vec!["system".to_string()])
    } else {
        match ensure_versions_exist(ctx, versions, false, &path.display().to_string())
            .and_then(|_| write_version_file(&path, versions))
        {
            Ok(_) => CommandReport::empty_success(),
            Err(error) => CommandReport::failure(vec![error.to_string()], 1),
        }
    }
}

pub fn cmd_local(ctx: &AppContext, versions: &[String], unset: bool, force: bool) -> CommandReport {
    let path = ctx.dir.join(LOCAL_VERSION_FILE);

    if unset {
        match fs::remove_file(&path) {
            Ok(_) => CommandReport::empty_success(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                CommandReport::empty_success()
            }
            Err(error) => CommandReport::failure(vec![format!("pyenv: {error}")], 1),
        }
    } else if versions.is_empty() {
        if let Some(local_path) = find_local_version_file(&ctx.dir) {
            cmd_version_file_read(&local_path)
        } else {
            CommandReport::failure(vec![PyenvError::NoLocalVersion.to_string()], 1)
        }
    } else {
        match ensure_versions_exist(ctx, versions, force, &path.display().to_string())
            .and_then(|_| write_version_file(&path, versions))
        {
            Ok(_) => CommandReport::empty_success(),
            Err(error) => CommandReport::failure(vec![error.to_string()], 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::{
        LOCAL_VERSION_FILE, cmd_global, cmd_local, cmd_version, cmd_version_file_read,
        cmd_version_file_write, cmd_version_name, cmd_version_origin, find_local_version_file,
        installed_version_dir, read_version_file, version_file_path,
    };

    fn test_context() -> (TempDir, AppContext) {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        let dir = temp.path().join("work");
        fs::create_dir_all(root.join("versions")).expect("versions dir");
        fs::create_dir_all(&dir).expect("work dir");

        let ctx = AppContext {
            root,
            dir,
            exe_path: std::path::PathBuf::from("pyenv"),
            env_version: None,
            env_shell: None,
            path_env: None,
            path_ext: None,
            config: AppConfig::default(),
        };

        (temp, ctx)
    }

    #[test]
    fn finds_local_version_file_in_parent_chain() {
        let (_temp, ctx) = test_context();
        let project = ctx.dir.join("project").join("nested");
        fs::create_dir_all(&project).expect("project");
        let local_file = ctx.dir.join("project").join(LOCAL_VERSION_FILE);
        fs::write(&local_file, "3.12.1\n").expect("version file");

        assert_eq!(find_local_version_file(&project), Some(local_file));
    }

    #[test]
    fn version_name_prefers_environment() {
        let (_temp, mut ctx) = test_context();
        fs::create_dir_all(installed_version_dir(&ctx, "3.12.1")).expect("installed version");
        ctx.env_version = Some("3.12.1".to_string());

        let report = cmd_version_name(&ctx, false);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec!["3.12.1"]);
    }

    #[test]
    fn global_command_writes_version_file() {
        let (_temp, ctx) = test_context();
        fs::create_dir_all(installed_version_dir(&ctx, "3.11.9")).expect("installed version");

        let report = cmd_global(&ctx, &[String::from("3.11.9")], false);
        assert_eq!(report.exit_code, 0);
        assert_eq!(
            fs::read_to_string(
                version_file_path(&ctx, None)
                    .parent()
                    .expect("parent")
                    .join("version")
            )
            .expect("global file"),
            "3.11.9\n"
        );
    }

    #[test]
    fn local_command_can_force_uninstalled_version() {
        let (_temp, ctx) = test_context();

        let report = cmd_local(&ctx, &[String::from("3.99.0")], false, true);
        assert_eq!(report.exit_code, 0);
        assert_eq!(
            fs::read_to_string(ctx.dir.join(LOCAL_VERSION_FILE)).expect("local file"),
            "3.99.0\n"
        );
    }

    #[test]
    fn version_defaults_to_system_when_unconfigured() {
        let (_temp, ctx) = test_context();
        let report = cmd_version_name(&ctx, false);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec!["system"]);
    }

    #[test]
    fn version_name_falls_back_to_latest_prefix() {
        let (_temp, mut ctx) = test_context();
        fs::create_dir_all(installed_version_dir(&ctx, "3.12.6")).expect("installed version");
        ctx.env_version = Some("python-3.12".to_string());

        let report = cmd_version_name(&ctx, false);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec!["3.12.6"]);
    }

    #[test]
    fn version_bare_emits_each_selected_version_on_its_own_line() {
        let (_temp, mut ctx) = test_context();
        for version in ["3.12.6", "3.11.9"] {
            fs::create_dir_all(installed_version_dir(&ctx, version)).expect("installed version");
        }
        ctx.env_version = Some("3.12:3.11".to_string());

        let report = cmd_version(&ctx, true);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec!["3.12.6", "3.11.9"]);
    }

    #[test]
    fn version_file_read_joins_versions_and_reports_safe_warnings() {
        let (_temp, ctx) = test_context();
        let path = ctx.dir.join("my-version");
        fs::write(&path, "3.9.3\n../*\n3.8.9\n# ignored\n").expect("version file");

        let report = cmd_version_file_read(&path);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec!["3.9.3:3.8.9"]);
        assert_eq!(report.stderr.len(), 1);
        assert!(report.stderr[0].contains("invalid version"));
        assert!(report.stderr[0].contains("../*"));
    }

    #[test]
    fn version_file_read_allows_internal_parent_components_within_version_tree() {
        let (_temp, ctx) = test_context();
        let path = ctx.dir.join("my-version");
        fs::write(&path, "3.10.3/envs/../test\n").expect("version file");

        let versions = read_version_file(&path).expect("versions");
        assert_eq!(versions, vec!["3.10.3/envs/../test"]);
    }

    #[test]
    fn version_name_reports_missing_origin_for_environment_version() {
        let (_temp, mut ctx) = test_context();
        ctx.env_version = Some("1.2".to_string());

        let report = cmd_version_name(&ctx, false);
        assert_eq!(report.exit_code, 1);
        assert_eq!(report.stdout, vec![""]);
        assert!(report.stderr[0].contains("set by PYENV_VERSION environment variable"));
    }

    #[test]
    fn version_file_write_persists_versions() {
        let (_temp, ctx) = test_context();
        fs::create_dir_all(installed_version_dir(&ctx, "3.12.6")).expect("installed version");
        let output = ctx.dir.join("custom-version");

        let report = cmd_version_file_write(&ctx, &output, &[String::from("3.12.6")], false);
        assert_eq!(report.exit_code, 0);
        assert_eq!(
            fs::read_to_string(output).expect("version file"),
            "3.12.6\n"
        );
    }

    #[test]
    fn version_name_hook_can_override_selected_value() {
        let (_temp, mut ctx) = test_context();
        let hook_dir = ctx.root.join("pyenv.d").join("version-name");
        fs::create_dir_all(installed_version_dir(&ctx, "3.12.6")).expect("installed version");
        fs::create_dir_all(&hook_dir).expect("hook dir");
        if cfg!(windows) {
            fs::write(
                hook_dir.join("override.cmd"),
                "@echo ENV:PYENV_VERSION=3.12.6",
            )
            .expect("hook");
        } else {
            fs::write(
                hook_dir.join("override.sh"),
                "#!/usr/bin/env sh\necho ENV:PYENV_VERSION=3.12.6\n",
            )
            .expect("hook");
        }
        ctx.env_version = Some("3.12".to_string());

        let report = cmd_version_name(&ctx, false);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec!["3.12.6"]);
    }

    #[test]
    fn version_origin_hook_can_override_origin_text() {
        let (_temp, ctx) = test_context();
        let hook_dir = ctx.root.join("pyenv.d").join("version-origin");
        fs::create_dir_all(&hook_dir).expect("hook dir");
        if cfg!(windows) {
            fs::write(
                hook_dir.join("override.cmd"),
                "@echo ENV:PYENV_VERSION_ORIGIN=hooked-origin",
            )
            .expect("hook");
        } else {
            fs::write(
                hook_dir.join("override.sh"),
                "#!/usr/bin/env sh\necho ENV:PYENV_VERSION_ORIGIN=hooked-origin\n",
            )
            .expect("hook");
        }

        let report = cmd_version_origin(&ctx);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout, vec!["hooked-origin"]);
    }
}
