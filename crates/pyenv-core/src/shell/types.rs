// ./crates/pyenv-core/src/shell/types.rs
//! Shared shell types for init parsing and shell-kind routing.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InitMode {
    Help,
    Print,
    Path,
    DetectShell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitCommandOptions {
    pub(super) mode: InitMode,
    pub(super) shell: ShellKind,
    pub(super) no_push_path: bool,
    pub(super) no_rehash: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShellKind {
    Pwsh,
    Cmd,
    Bash,
    Zsh,
    Fish,
    Sh,
}

impl ShellKind {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "pwsh" | "powershell" | "ps" => Some(Self::Pwsh),
            "cmd" | "cmd.exe" | "batch" => Some(Self::Cmd),
            "bash" => Some(Self::Bash),
            "zsh" => Some(Self::Zsh),
            "fish" => Some(Self::Fish),
            "sh" => Some(Self::Sh),
            _ => None,
        }
    }

    pub(super) fn canonical_name(self) -> &'static str {
        match self {
            Self::Pwsh => "pwsh",
            Self::Cmd => "cmd",
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
            Self::Sh => "sh",
        }
    }
}
