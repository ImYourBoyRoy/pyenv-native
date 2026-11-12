// ./crates/pyenv-core/src/fs_util.rs
//! Small filesystem helpers shared by the venv, installer, and updater code.
//!
//! Windows briefly holds directory and file handles (antivirus, the search indexer, editors,
//! language servers), so deletes and renames there can fail transiently with "access denied".
//! These helpers retry with a short back-off on Windows and make a single attempt elsewhere.

use std::fs;
use std::io;
use std::path::Path;
use std::thread;
use std::time::Duration;

/// Total attempts for retried filesystem operations (one attempt on non-Windows hosts).
const RETRY_ATTEMPTS: u32 = if cfg!(windows) { 6 } else { 1 };

/// Run `op`, retrying with a linear back-off while it keeps failing.
pub(crate) fn retry_io<T>(mut op: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let mut attempt = 1;
    loop {
        match op() {
            Ok(value) => return Ok(value),
            Err(error) if attempt >= RETRY_ATTEMPTS => return Err(error),
            Err(_) => {
                thread::sleep(Duration::from_millis(250 * u64::from(attempt)));
                attempt += 1;
            }
        }
    }
}

/// Recursively remove a directory; a missing directory counts as success.
pub(crate) fn remove_dir_all_with_retry(path: &Path) -> io::Result<()> {
    retry_io(|| match fs::remove_dir_all(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    })
}

/// Remove a single file; a missing file counts as success.
pub(crate) fn remove_file_with_retry(path: &Path) -> io::Result<()> {
    retry_io(|| match fs::remove_file(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    })
}

/// Rename a file or directory.
pub(crate) fn rename_with_retry(from: &Path, to: &Path) -> io::Result<()> {
    retry_io(|| fs::rename(from, to))
}

/// Returns whether a process with this id currently exists.
#[cfg(unix)]
pub(crate) fn process_exists(pid: u32) -> bool {
    unsafe extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    // SAFETY: signal 0 only probes for existence; it delivers nothing.
    unsafe { kill(pid as i32, 0) == 0 }
}

/// Returns whether a process with this id is still running.
///
/// A terminated process can stay openable for as long as anything holds a handle to it, so
/// opening it is not enough: the exit code must still read `STILL_ACTIVE`.
#[cfg(windows)]
pub(crate) fn process_exists(pid: u32) -> bool {
    unsafe extern "system" {
        fn OpenProcess(
            dwDesiredAccess: u32,
            bInheritHandle: i32,
            dwProcessId: u32,
        ) -> *mut std::ffi::c_void;
        fn GetExitCodeProcess(hProcess: *mut std::ffi::c_void, lpExitCode: *mut u32) -> i32;
        fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
    }

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;

    // SAFETY: plain Win32 calls; the handle is closed before returning.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let mut code = 0u32;
        let queried = GetExitCodeProcess(handle, &mut code) != 0;
        CloseHandle(handle);
        // If the status cannot be read, assume the process is alive rather than racing it.
        !queried || code == STILL_ACTIVE
    }
}

/// Returns whether a process with this id currently exists.
#[cfg(not(any(windows, unix)))]
pub(crate) fn process_exists(_pid: u32) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn missing_paths_count_as_removed() {
        let temp = TempDir::new().expect("tempdir");
        assert!(remove_dir_all_with_retry(&temp.path().join("nope")).is_ok());
        assert!(remove_file_with_retry(&temp.path().join("nope.txt")).is_ok());
    }

    #[test]
    fn removes_nested_directories_and_files() {
        let temp = TempDir::new().expect("tempdir");
        let nested = temp.path().join("a").join("b");
        fs::create_dir_all(&nested).expect("nested");
        fs::write(nested.join("f"), "x").expect("file");
        remove_dir_all_with_retry(&temp.path().join("a")).expect("remove");
        assert!(!temp.path().join("a").exists());
    }

    #[test]
    fn current_process_exists() {
        assert!(process_exists(std::process::id()));
    }
}
