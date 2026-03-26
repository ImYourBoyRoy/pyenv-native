// ./crates/pyenv-core/src/install/providers/python_build.rs
//! python-build backend discovery and definition loading helpers.

use crate::process::PyenvCommandExt;
use std::env;
use std::path::PathBuf;
use std::process::Command;

use crate::context::AppContext;
use crate::error::PyenvError;
use crate::runtime::search_path_entries;

use super::super::report::format_command_output_suffix;

pub(crate) fn load_python_build_definitions(ctx: &AppContext) -> Result<Vec<String>, PyenvError> {
    let python_build = resolve_python_build_path(ctx)?;
    let output = Command::new(&python_build)
        .headless()
        .arg("--definitions")
        .current_dir(&ctx.dir)
        .output()
        .map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to execute {} --definitions: {error}",
                python_build.display()
            ))
        })?;

    if !output.status.success() {
        return Err(PyenvError::Io(format!(
            "pyenv: python-build --definitions failed with exit code {}{}",
            output.status.code().unwrap_or(1),
            format_command_output_suffix(&output.stdout, &output.stderr)
        )));
    }

    let mut versions = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    versions.sort_by(|lhs, rhs| crate::catalog::compare_version_names(lhs, rhs).reverse());
    versions.dedup();
    Ok(versions)
}

pub(crate) fn resolve_python_build_path(ctx: &AppContext) -> Result<PathBuf, PyenvError> {
    if let Some(configured) = ctx.config.install.python_build_path.as_ref() {
        let path = if configured.is_absolute() {
            configured.clone()
        } else {
            ctx.root.join(configured)
        };
        if path.is_file() {
            return Ok(path);
        }
    }

    if let Some(path) = find_command_on_path(ctx, "python-build") {
        return Ok(path);
    }

    for candidate in repo_relative_python_build_candidates(ctx) {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(PyenvError::MissingPythonBuildBackend)
}

fn repo_relative_python_build_candidates(ctx: &AppContext) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let mut roots = Vec::new();
    if let Some(parent) = ctx.dir.parent() {
        roots.push(parent.to_path_buf());
    }
    if let Some(parent) = ctx.root.parent() {
        roots.push(parent.to_path_buf());
    }
    if let Some(parent) = ctx.exe_path.parent().and_then(|path| path.parent()) {
        roots.push(parent.to_path_buf());
    }

    for root in roots {
        candidates.push(
            root.join("pyenv")
                .join("plugins")
                .join("python-build")
                .join("bin")
                .join("python-build"),
        );
        candidates.push(
            root.join("vendor")
                .join("pyenv")
                .join("plugins")
                .join("python-build")
                .join("bin")
                .join("python-build"),
        );
        candidates.push(
            root.join("pyenv")
                .join("plugins")
                .join("python-build")
                .join("bin")
                .join("python-build.cmd"),
        );
        candidates.push(
            root.join("vendor")
                .join("pyenv")
                .join("plugins")
                .join("python-build")
                .join("bin")
                .join("python-build.cmd"),
        );
    }

    candidates
}

fn find_command_on_path(ctx: &AppContext, command: &str) -> Option<PathBuf> {
    let directories = ctx
        .path_env
        .as_ref()
        .map(env::split_paths)
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    search_path_entries(&directories, command, ctx.path_ext.as_deref())
}
