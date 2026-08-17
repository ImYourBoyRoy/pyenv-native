// ./crates/pyenv-core/src/process.rs
//! Subprocess execution helpers and Windows window-suppression extensions.

use std::env;
use std::process::Command;

/// Extension trait for `std::process::Command` to handle headless execution on Windows.
pub trait PyenvCommandExt {
    /// Configures the command to run without a console window on Windows.
    /// This is essential for GUI applications to prevent terminal pop-ups.
    fn headless(&mut self) -> &mut Self;
}

impl PyenvCommandExt for Command {
    fn headless(&mut self) -> &mut Self {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NO_WINDOW = 0x08000000
            // This flag is ignored if the application already has a console.
            // If the application (like a GUI) does not have a console, this prevents
            // a new console window from being created for the child process.
            self.creation_flags(0x08000000);
        }
        self
    }
}

/// Prefer PowerShell 7 (`pwsh`) when it is on PATH; otherwise Windows PowerShell.
pub fn windows_powershell_host() -> &'static str {
    if command_on_path("pwsh") {
        "pwsh"
    } else {
        "powershell.exe"
    }
}

fn command_on_path(name: &str) -> bool {
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path)
        .any(|dir| dir.join(name).is_file() || dir.join(format!("{name}.exe")).is_file())
}
