// ./crates/pyenv-core/src/installer/binaries.rs
//! Replacing installed executables safely, including ones that are currently running.
//!
//! POSIX allows renaming a new file over a running executable. Windows does not, but it does
//! allow renaming a running executable *out of the way*, so the old file is parked under a unique
//! `.old-*` name and swept up on a later run once the process using it has exited.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::fs_util::remove_file_with_retry;
#[cfg(not(unix))]
use crate::fs_util::rename_with_retry;

/// What `install_binary` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BinaryInstall {
    /// The destination now holds the source's bytes.
    Installed,
    /// Source and destination are the same file; nothing to do.
    AlreadyInPlace,
}

fn same_file(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    format!("{}-{nanos}", std::process::id())
}

fn sibling_with_name(path: &Path, name: String) -> PathBuf {
    path.with_file_name(name)
}

/// Install `source` as `destination`, replacing any existing (possibly running) file.
pub(crate) fn install_binary(source: &Path, destination: &Path) -> io::Result<BinaryInstall> {
    if same_file(source, destination) {
        return Ok(BinaryInstall::AlreadyInPlace);
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("pyenv")
        .to_string();
    let staged = sibling_with_name(destination, format!(".{file_name}.new-{}", unique_suffix()));

    fs::copy(source, &staged)?;
    if let Err(error) = make_executable(&staged) {
        let _ = fs::remove_file(&staged);
        return Err(error);
    }

    let result = swap_into_place(&staged, destination, &file_name);
    if result.is_err() {
        let _ = fs::remove_file(&staged);
    }
    result.map(|()| BinaryInstall::Installed)
}

#[cfg(unix)]
fn swap_into_place(staged: &Path, destination: &Path, _file_name: &str) -> io::Result<()> {
    // rename(2) atomically replaces the target even while it is executing.
    fs::rename(staged, destination)
}

#[cfg(not(unix))]
fn swap_into_place(staged: &Path, destination: &Path, file_name: &str) -> io::Result<()> {
    let parked = if destination.exists() {
        let parked = sibling_with_name(destination, format!("{file_name}.old-{}", unique_suffix()));
        rename_with_retry(destination, &parked)?;
        Some(parked)
    } else {
        None
    };
    match rename_with_retry(staged, destination) {
        Ok(()) => Ok(()),
        Err(error) => {
            if let Some(parked) = parked {
                let _ = fs::rename(&parked, destination);
            }
            Err(error)
        }
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> io::Result<()> {
    Ok(())
}

/// Remove leftovers of earlier replacements (`*.old-*` parked files, `.*.new-*` staged copies).
/// Files still held open by a running process are skipped and retried on the next run.
pub(crate) fn sweep_replaced_binaries(bin_dir: &Path) -> usize {
    let Ok(entries) = fs::read_dir(bin_dir) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let leftover = (name.contains(".old-") && name.starts_with("pyenv"))
            || (name.starts_with(".pyenv") && name.contains(".new-"));
        if leftover && remove_file_with_retry(&entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// `.cmd`/`.ps1` launchers that let `cmd.exe` and PowerShell find the Windows executables.
pub(crate) fn windows_wrapper_files(include_mcp: bool) -> Vec<(&'static str, String)> {
    let mut files = vec![
        (
            "pyenv.cmd",
            "@echo off\r\n\"%~dp0pyenv.exe\" %*\r\n".to_string(),
        ),
        (
            "pyenv.ps1",
            "& \"$PSScriptRoot\\pyenv.exe\" @args\r\nexit $LASTEXITCODE\r\n".to_string(),
        ),
        (
            "pyenv-init.cmd",
            "@echo off\r\nfor /f \"delims=\" %%i in ('\"%~dp0pyenv.exe\" init - cmd') do %%i\r\n"
                .to_string(),
        ),
    ];
    if include_mcp {
        files.push((
            "pyenv-mcp.cmd",
            "@echo off\r\n\"%~dp0pyenv-mcp.exe\" %*\r\n".to_string(),
        ));
        files.push((
            "pyenv-mcp.ps1",
            "& \"$PSScriptRoot\\pyenv-mcp.exe\" @args\r\nexit $LASTEXITCODE\r\n".to_string(),
        ));
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn installs_and_replaces_binaries() {
        let temp = TempDir::new().expect("tempdir");
        let source = temp.path().join("src").join("pyenv");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "v1").unwrap();
        let dest = temp.path().join("bin").join("pyenv");

        assert_eq!(
            install_binary(&source, &dest).unwrap(),
            BinaryInstall::Installed
        );
        assert_eq!(fs::read_to_string(&dest).unwrap(), "v1");

        fs::write(&source, "v2").unwrap();
        assert_eq!(
            install_binary(&source, &dest).unwrap(),
            BinaryInstall::Installed
        );
        assert_eq!(fs::read_to_string(&dest).unwrap(), "v2");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&dest).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0o111, "installed binary must be executable");
        }
    }

    #[test]
    fn detects_when_source_is_the_destination() {
        let temp = TempDir::new().expect("tempdir");
        let file = temp.path().join("pyenv");
        fs::write(&file, "x").unwrap();
        assert_eq!(
            install_binary(&file, &file).unwrap(),
            BinaryInstall::AlreadyInPlace
        );
        assert_eq!(fs::read_to_string(&file).unwrap(), "x");
    }

    #[test]
    fn no_staging_files_are_left_behind() {
        let temp = TempDir::new().expect("tempdir");
        let source = temp.path().join("pyenv");
        fs::write(&source, "x").unwrap();
        let bin = temp.path().join("bin");
        install_binary(&source, &bin.join("pyenv")).unwrap();
        let names: Vec<_> = fs::read_dir(&bin)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["pyenv".to_string()]);
    }

    #[test]
    fn sweeps_parked_and_staged_leftovers_only() {
        let temp = TempDir::new().expect("tempdir");
        for name in [
            "pyenv.exe",
            "pyenv.exe.old-1-2",
            ".pyenv.exe.new-3-4",
            "unrelated.old-1",
        ] {
            fs::write(temp.path().join(name), "x").unwrap();
        }
        assert_eq!(sweep_replaced_binaries(temp.path()), 2);
        assert!(temp.path().join("pyenv.exe").exists());
        assert!(temp.path().join("unrelated.old-1").exists());
    }

    #[test]
    fn windows_wrappers_use_crlf_and_optionally_include_mcp() {
        let files = windows_wrapper_files(false);
        assert_eq!(files.len(), 3);
        assert!(files.iter().all(|(_, text)| text.ends_with("\r\n")));
        assert_eq!(windows_wrapper_files(true).len(), 5);
    }
}
