// ./crates/pyenv-core/src/command.rs
//! Common command result types shared across the native pyenv core.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandReport {
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
    pub exit_code: i32,
}

impl CommandReport {
    pub fn success(stdout: Vec<String>) -> Self {
        Self {
            stdout,
            stderr: Vec::new(),
            exit_code: 0,
        }
    }

    pub fn empty_success() -> Self {
        Self::success(Vec::new())
    }

    pub fn success_one(line: impl Into<String>) -> Self {
        Self::success(vec![line.into()])
    }

    pub fn failure(stderr: Vec<String>, exit_code: i32) -> Self {
        Self {
            stdout: Vec::new(),
            stderr,
            exit_code,
        }
    }
}
