// ./crates/pyenv-core/src/shell/helpers.rs
//! Validation and resolution helpers used by shell-facing commands.

use std::path::{Path, PathBuf};

use crate::context::AppContext;
use crate::error::PyenvError;
use crate::manage::cmd_prefix;
use crate::venv::{ManagedVenvInfo, resolve_managed_venv};
use crate::version::resolve_selected_versions;

pub(super) fn validate_shell_versions(
    ctx: &AppContext,
    versions: &[String],
) -> Result<(), PyenvError> {
    for version in versions {
        #[allow(clippy::cloned_ref_to_slice_refs)]
        let report = cmd_prefix(ctx, &[version.clone()]);
        if report.exit_code != 0 {
            let message = report
                .stderr
                .first()
                .cloned()
                .unwrap_or_else(|| format!("pyenv: version `{version}` not installed"));
            return Err(PyenvError::Io(message));
        }
    }

    Ok(())
}

pub(super) fn resolve_activation_target(
    ctx: &AppContext,
    requested: Option<&str>,
) -> Result<ManagedVenvInfo, PyenvError> {
    if let Some(spec) = requested.map(str::trim).filter(|value| !value.is_empty()) {
        return resolve_managed_venv(ctx, spec);
    }

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
            "pyenv: no managed venv is selected; pass a venv name/spec such as `pyenv activate api`"
                .to_string(),
        ));
    };

    resolve_managed_venv(ctx, active).map_err(|_| {
        PyenvError::Io(format!(
            "pyenv: the current selection `{active}` is not a managed venv; pass a venv name/spec such as `pyenv activate api`"
        ))
    })
}

pub(super) fn virtualenv_bin_dir(prefix: &Path) -> Option<PathBuf> {
    let candidates = if cfg!(windows) {
        [prefix.join("Scripts"), prefix.join("bin")]
    } else {
        [prefix.join("bin"), prefix.join("Scripts")]
    };
    candidates.into_iter().find(|path| path.is_dir())
}

pub(super) fn io_error(error: std::io::Error) -> PyenvError {
    PyenvError::Io(format!("pyenv: {error}"))
}
