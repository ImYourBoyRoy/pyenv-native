// ./crates/pyenv-core/src/installer/user_path.rs
//! Persistent per-user `PATH` management on Windows (HKCU\Environment).
//!
//! The list manipulation is pure and unit-tested on every platform. The registry access is
//! Windows-only; elsewhere the public functions are no-ops because POSIX hosts integrate through
//! shell profiles instead (see `profile`). The `Path` value keeps its `REG_EXPAND_SZ` type so
//! entries such as `%USERPROFILE%\bin` written by other tools keep expanding.

use std::io;
use std::path::{Path, PathBuf};

const SEPARATOR: char = ';';

fn normalized(segment: &str) -> String {
    segment
        .trim()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

/// Put `entries` at the front of `current`, removing any earlier copies (case-insensitively).
pub(crate) fn with_entries_first(current: &str, entries: &[String]) -> String {
    let wanted: Vec<String> = entries
        .iter()
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect();
    let remaining = current.split(SEPARATOR).filter(|segment| {
        let key = normalized(segment);
        !key.is_empty() && !wanted.iter().any(|entry| normalized(entry) == key)
    });
    wanted
        .iter()
        .map(String::as_str)
        .chain(remaining)
        .collect::<Vec<_>>()
        .join(&SEPARATOR.to_string())
}

/// Drop every segment equal to, or located under, `root`.
pub(crate) fn without_entries_under(current: &str, root: &str) -> String {
    let root = normalized(root);
    let prefix = format!("{root}\\");
    current
        .split(SEPARATOR)
        .filter(|segment| {
            let key = normalized(segment);
            !key.is_empty() && key != root && !key.starts_with(&prefix)
        })
        .collect::<Vec<_>>()
        .join(&SEPARATOR.to_string())
}

/// Prepend `entries` to the user's persistent `PATH`. Returns whether the value changed.
pub fn add_user_path_entries(entries: &[PathBuf]) -> io::Result<bool> {
    let entries: Vec<String> = entries
        .iter()
        .map(|entry| entry.to_string_lossy().into_owned())
        .collect();
    platform::update_user_path(|current| with_entries_first(current, &entries))
}

/// Remove every entry under `root` from the user's persistent `PATH`.
pub fn remove_user_path_entries_under(root: &Path) -> io::Result<bool> {
    let root = root.to_string_lossy().into_owned();
    platform::update_user_path(|current| without_entries_under(current, &root))
}

/// Delete a per-user environment variable, but only when its value is `expected` (compared like a
/// path). This keeps an unrelated `PYENV_ROOT` that points at another install untouched.
/// Returns whether the variable was removed.
pub fn clear_user_env_var_if(name: &str, expected: &Path) -> io::Result<bool> {
    platform::clear_env_var_if(name, &normalized(&expected.to_string_lossy()))
}

#[cfg(windows)]
mod platform {
    use std::borrow::Cow;
    use std::io;

    use winreg::RegKey;
    use winreg::RegValue;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, RegType};

    const ENVIRONMENT_KEY: &str = "Environment";

    #[link(name = "user32")]
    unsafe extern "system" {
        fn SendMessageTimeoutW(
            hwnd: isize,
            msg: u32,
            wparam: usize,
            lparam: isize,
            flags: u32,
            timeout: u32,
            result: *mut usize,
        ) -> isize;
    }

    /// Tell running programs (Explorer, new terminals) that the environment changed.
    fn broadcast_environment_change() {
        const HWND_BROADCAST: isize = 0xffff;
        const WM_SETTINGCHANGE: u32 = 0x001A;
        const SMTO_ABORTIFHUNG: u32 = 0x0002;
        let payload: Vec<u16> = "Environment\0".encode_utf16().collect();
        let mut result = 0usize;
        // SAFETY: `payload` is a NUL-terminated UTF-16 string that outlives the call.
        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                payload.as_ptr() as isize,
                SMTO_ABORTIFHUNG,
                5000,
                &mut result,
            );
        }
    }

    fn open_environment() -> io::Result<RegKey> {
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(ENVIRONMENT_KEY, KEY_READ | KEY_WRITE)
    }

    pub(super) fn update_user_path(edit: impl FnOnce(&str) -> String) -> io::Result<bool> {
        let key = open_environment()?;
        let (current, vtype) = match key.get_raw_value("Path") {
            Ok(raw) => {
                let text = key.get_value::<String, _>("Path").unwrap_or_default();
                (text, raw.vtype)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                (String::new(), RegType::REG_EXPAND_SZ)
            }
            Err(error) => return Err(error),
        };
        let updated = edit(&current);
        if updated == current {
            return Ok(false);
        }
        let mut bytes: Vec<u8> = updated
            .encode_utf16()
            .chain(std::iter::once(0))
            .flat_map(u16::to_le_bytes)
            .collect();
        bytes.shrink_to_fit();
        let value = RegValue {
            bytes: Cow::Owned(bytes),
            vtype: if vtype == RegType::REG_SZ {
                RegType::REG_SZ
            } else {
                RegType::REG_EXPAND_SZ
            },
        };
        key.set_raw_value("Path", &value)?;
        broadcast_environment_change();
        Ok(true)
    }

    pub(super) fn clear_env_var_if(name: &str, expected_normalized: &str) -> io::Result<bool> {
        let key = open_environment()?;
        match key.get_value::<String, _>(name) {
            Ok(value) if super::normalized(&value) == expected_normalized => {}
            Ok(_) => return Ok(false),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error),
        }
        match key.delete_value(name) {
            Ok(()) => {
                broadcast_environment_change();
                Ok(true)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use std::io;

    pub(super) fn update_user_path(_edit: impl FnOnce(&str) -> String) -> io::Result<bool> {
        Ok(false)
    }

    pub(super) fn clear_env_var_if(_name: &str, _expected: &str) -> io::Result<bool> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_move_to_the_front_without_duplicates() {
        let current = r"C:\Tools;C:\Users\Roy\.pyenv\shims;C:\Windows";
        let updated = with_entries_first(
            current,
            &[
                r"C:\Users\Roy\.pyenv\bin".to_string(),
                r"c:\users\roy\.pyenv\SHIMS\".to_string(),
            ],
        );
        assert_eq!(
            updated,
            r"C:\Users\Roy\.pyenv\bin;c:\users\roy\.pyenv\SHIMS\;C:\Tools;C:\Windows"
        );
    }

    #[test]
    fn adding_to_an_empty_path_yields_just_the_entries() {
        assert_eq!(with_entries_first("", &["C:\\a".to_string()]), "C:\\a");
        assert_eq!(with_entries_first(";;", &[]), "");
    }

    #[test]
    fn adding_is_idempotent() {
        let entries = [r"C:\r\bin".to_string(), r"C:\r\shims".to_string()];
        let once = with_entries_first(r"C:\x", &entries);
        assert_eq!(with_entries_first(&once, &entries), once);
    }

    #[test]
    fn removal_drops_the_root_and_everything_under_it() {
        let current = r"C:\r\bin;C:\other;C:\R\shims\;C:\r;C:\rx\bin";
        assert_eq!(
            without_entries_under(current, r"C:\r"),
            r"C:\other;C:\rx\bin"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_hosts_are_a_no_op() {
        assert!(!add_user_path_entries(&[PathBuf::from("/x")]).unwrap());
        assert!(!remove_user_path_entries_under(Path::new("/x")).unwrap());
        assert!(!clear_user_env_var_if("PYENV_ROOT", Path::new("/x")).unwrap());
    }
}
