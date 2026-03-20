// ./crates/pyenv-core/src/venv_compat.rs
//! Compatibility-oriented virtual environment commands that preserve upstream pyenv terms while
//! routing all behavior through the native managed-venv model. Use this module to support commands
//! like `virtualenv`, `virtualenvs`, and `virtualenv-prefix` without making them the primary UX.

use crate::command::CommandReport;
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::venv::{
    ManagedVenvInfo, cmd_venv_create, cmd_venv_delete, cmd_venv_list, resolve_managed_venv,
};
use crate::version::resolve_selected_versions;

pub fn cmd_virtualenv(
    ctx: &AppContext,
    requested_version: Option<&str>,
    name: &str,
    force: bool,
    set_local: bool,
) -> CommandReport {
    let base_version = match requested_version {
        Some(value) => value.trim().to_string(),
        None => match current_base_runtime(ctx) {
            Ok(value) => value,
            Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
        },
    };

    cmd_venv_create(ctx, &base_version, name, force, set_local)
}

pub fn cmd_virtualenvs(ctx: &AppContext, bare: bool, json: bool) -> CommandReport {
    cmd_venv_list(ctx, bare, json)
}

pub fn cmd_virtualenv_delete(ctx: &AppContext, spec: &str, force: bool) -> CommandReport {
    cmd_venv_delete(ctx, spec, force)
}

pub fn cmd_virtualenv_prefix(ctx: &AppContext, spec: Option<&str>) -> CommandReport {
    let info = match resolve_target_managed_venv(ctx, spec) {
        Ok(info) => info,
        Err(error) => return CommandReport::failure(vec![error.to_string()], 1),
    };
    CommandReport::success_one(info.path.display().to_string())
}

fn resolve_target_managed_venv(
    ctx: &AppContext,
    spec: Option<&str>,
) -> Result<ManagedVenvInfo, PyenvError> {
    if let Some(spec) = spec.map(str::trim).filter(|value| !value.is_empty()) {
        return resolve_managed_venv(ctx, spec);
    }

    current_managed_venv(ctx)
}

fn current_base_runtime(ctx: &AppContext) -> Result<String, PyenvError> {
    let info = current_selection(ctx)?;
    match info {
        CurrentSelection::ManagedVenv(env) => Ok(env.base_version),
        CurrentSelection::Runtime(version) => Ok(version),
    }
}

fn current_managed_venv(ctx: &AppContext) -> Result<ManagedVenvInfo, PyenvError> {
    match current_selection(ctx)? {
        CurrentSelection::ManagedVenv(env) => Ok(env),
        CurrentSelection::Runtime(version) => Err(PyenvError::Io(format!(
            "pyenv: the current selection `{version}` is a runtime, not a managed venv; pass a venv name/spec or use `pyenv venv use <name>` first"
        ))),
    }
}

fn current_selection(ctx: &AppContext) -> Result<CurrentSelection, PyenvError> {
    let selected = resolve_selected_versions(ctx, false);
    if !selected.missing.is_empty() {
        return Err(PyenvError::Io(format!(
            "pyenv: the current selection is missing: {}",
            selected
                .missing
                .iter()
                .map(|value| format!("`{value}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    let Some(active) = selected.versions.first() else {
        return Err(PyenvError::Io(
            "pyenv: no current runtime is selected; set one with `pyenv global <version>` or pass an explicit runtime"
                .to_string(),
        ));
    };

    if active.eq_ignore_ascii_case("system") {
        return Err(PyenvError::Io(
            "pyenv: the current selection is `system`; pass an installed runtime explicitly, such as `pyenv virtualenv 3.13 app`"
                .to_string(),
        ));
    }

    match resolve_managed_venv(ctx, active) {
        Ok(info) => Ok(CurrentSelection::ManagedVenv(info)),
        Err(_) => Ok(CurrentSelection::Runtime(active.to_string())),
    }
}

enum CurrentSelection {
    Runtime(String),
    ManagedVenv(ManagedVenvInfo),
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;
    use crate::version::cmd_global;

    use super::{cmd_virtualenv, cmd_virtualenv_prefix};

    fn test_context() -> (TempDir, AppContext) {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        let dir = temp.path().join("work");
        fs::create_dir_all(root.join("versions")).expect("versions dir");
        fs::create_dir_all(root.join("venvs")).expect("venvs dir");
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

    fn seed_managed_venv(ctx: &AppContext, version: &str, name: &str) {
        let version_dir = ctx.root.join("versions").join(version);
        let env_dir = ctx.root.join("venvs").join(version).join(name);
        let bin_dir = if cfg!(windows) {
            env_dir.join("Scripts")
        } else {
            env_dir.join("bin")
        };

        fs::create_dir_all(&version_dir).expect("version dir");
        fs::create_dir_all(&bin_dir).expect("bin dir");
        if cfg!(windows) {
            fs::write(env_dir.join("python.exe"), "").expect("python");
        } else {
            fs::write(bin_dir.join("python"), "").expect("python");
        }
    }

    #[test]
    fn virtualenv_requires_a_selected_runtime_when_none_is_provided() {
        let (_temp, ctx) = test_context();
        let create = cmd_virtualenv(&ctx, None, "demo", false, false);
        assert_eq!(create.exit_code, 1);
        assert!(create.stderr[0].contains("current selection"));
    }

    #[test]
    fn virtualenv_prefix_uses_current_managed_env_when_not_explicitly_provided() {
        let (_temp, ctx) = test_context();
        seed_managed_venv(&ctx, "3.12.6", "demo");
        let report = cmd_global(&ctx, &[String::from("3.12.6/envs/demo")], false);
        assert_eq!(report.exit_code, 0);

        let prefix = cmd_virtualenv_prefix(&ctx, None);
        assert_eq!(prefix.exit_code, 0);
        assert!(
            prefix
                .stdout
                .first()
                .expect("prefix")
                .replace('\\', "/")
                .ends_with("/.pyenv/venvs/3.12.6/demo")
        );
    }
}
