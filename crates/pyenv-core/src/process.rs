// ./crates/pyenv-core/src/process.rs
//! Subprocess execution helpers and Windows window-suppression extensions.

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
