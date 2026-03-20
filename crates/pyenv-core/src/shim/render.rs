// ./crates/pyenv-core/src/shim/render.rs
//! Shim script rendering and filesystem write helpers.

use std::fs;
use std::path::Path;

use crate::PyenvError;
use crate::context::AppContext;

use super::paths::{
    shim_bat_path, shim_cmd_path, shim_native_path, shim_posix_path, shim_ps1_path,
};

pub(super) fn write_shim_artifacts(
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

    if destination.exists() && files_match(source, destination)? {
        return Ok(());
    }

    if destination.exists() {
        fs::remove_file(destination).map_err(io_error)?;
    }

    fs::copy(source, destination).map_err(io_error)?;
    Ok(())
}

fn files_match(lhs: &Path, rhs: &Path) -> Result<bool, PyenvError> {
    let lhs_meta = fs::metadata(lhs).map_err(io_error)?;
    let rhs_meta = fs::metadata(rhs).map_err(io_error)?;
    if lhs_meta.len() != rhs_meta.len() {
        return Ok(false);
    }

    let lhs_file = fs::File::open(lhs).map_err(io_error)?;
    let rhs_file = fs::File::open(rhs).map_err(io_error)?;
    let mut lhs_reader = std::io::BufReader::new(lhs_file);
    let mut rhs_reader = std::io::BufReader::new(rhs_file);
    let mut lhs_buffer = [0u8; 8192];
    let mut rhs_buffer = [0u8; 8192];

    loop {
        use std::io::Read as _;

        let lhs_read = lhs_reader.read(&mut lhs_buffer).map_err(io_error)?;
        let rhs_read = rhs_reader.read(&mut rhs_buffer).map_err(io_error)?;
        if lhs_read != rhs_read {
            return Ok(false);
        }
        if lhs_read == 0 {
            return Ok(true);
        }
        if lhs_buffer[..lhs_read] != rhs_buffer[..rhs_read] {
            return Ok(false);
        }
    }
}

pub(super) fn make_executable(path: &Path) -> Result<(), PyenvError> {
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

fn sh_single_quote(value: &str) -> String {
    value.replace('\'', "'\\''")
}

fn io_error(error: std::io::Error) -> PyenvError {
    PyenvError::Io(format!("pyenv: {error}"))
}
