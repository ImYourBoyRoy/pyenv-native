// ./crates/pyenv-core/src/installer/profile.rs
//! Shell profile integration: render, install, update, and remove the `pyenv-native init` block.
//!
//! Every front end (installer, updater, GUI, uninstaller) goes through this module so a profile
//! only ever carries one clearly delimited block that can be updated or removed cleanly.
//! The pure string functions (`upsert_block`, `remove_block`, `render_block`) hold the logic;
//! the file functions are thin wrappers around them.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::text::{line_ending_for, normalize_block_eol};

pub const BEGIN_MARKER: &str = "# >>> pyenv-native init >>>";
pub const END_MARKER: &str = "# <<< pyenv-native init <<<";

/// Header written by older GUI builds in front of an unmarked init snippet.
const LEGACY_GUI_HEADER: &str = "# pyenv-native shell initialization";

/// Shells pyenv-native can integrate with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShellKind {
    Bash,
    Zsh,
    Fish,
    /// POSIX `sh` (`~/.profile`).
    Sh,
    /// PowerShell 7+ and Windows PowerShell.
    Pwsh,
    /// `cmd.exe` has no profile; it is initialised through `pyenv-init.cmd`.
    Cmd,
    None,
}

impl ShellKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "bash" => Some(Self::Bash),
            "zsh" => Some(Self::Zsh),
            "fish" => Some(Self::Fish),
            "sh" | "dash" | "ash" => Some(Self::Sh),
            "pwsh" | "powershell" => Some(Self::Pwsh),
            "cmd" => Some(Self::Cmd),
            "none" | "" => Some(Self::None),
            _ => Option::None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
            Self::Sh => "sh",
            Self::Pwsh => "pwsh",
            Self::Cmd => "cmd",
            Self::None => "none",
        }
    }

    /// Detect the user's interactive shell from the environment.
    pub fn detect() -> Self {
        if cfg!(windows) {
            return Self::Pwsh;
        }
        let shell = std::env::var("SHELL").unwrap_or_default();
        let name = Path::new(shell.trim())
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        match name.as_str() {
            "bash" => Self::Bash,
            "zsh" => Self::Zsh,
            "fish" => Self::Fish,
            "" if is_termux() => Self::Bash,
            _ => Self::Sh,
        }
    }

    /// Whether this shell reads a profile file that can hold the init block.
    pub fn has_profile(self) -> bool {
        !matches!(self, Self::Cmd | Self::None)
    }
}

/// Outcome of applying the init block to a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileUpdate {
    /// The profile did not exist and was created.
    Created,
    /// An existing pyenv-native block was refreshed in place.
    Updated,
    /// A new block was appended to an existing profile.
    Appended,
    /// The block was already current; nothing was written.
    Unchanged,
    /// The profile already initialises pyenv through a snippet we did not write.
    AlreadyConfigured,
}

fn is_termux() -> bool {
    std::env::var_os("TERMUX_VERSION").is_some()
        || std::env::var("PREFIX")
            .map(|value| value.contains("com.termux"))
            .unwrap_or(false)
}

/// The current user's home directory.
pub fn home_dir() -> Option<PathBuf> {
    let key = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Profile file that `kind` reads, relative to `home`.
pub fn profile_path(kind: ShellKind, home: &Path) -> Option<PathBuf> {
    match kind {
        ShellKind::Bash => Some(home.join(".bashrc")),
        ShellKind::Zsh => Some(home.join(".zshrc")),
        ShellKind::Fish => Some(home.join(".config").join("fish").join("config.fish")),
        ShellKind::Sh if is_termux() => Some(home.join(".bashrc")),
        ShellKind::Sh => Some(home.join(".profile")),
        ShellKind::Pwsh => Some(powershell_profile_path(home, true)),
        ShellKind::Cmd | ShellKind::None => Option::None,
    }
}

/// PowerShell current-user profile. `modern` selects PowerShell 7+ over Windows PowerShell 5.1.
pub fn powershell_profile_path(home: &Path, modern: bool) -> PathBuf {
    const PROFILE: &str = "Microsoft.PowerShell_profile.ps1";
    if cfg!(windows) {
        let documents = windows_documents_dir(home);
        let folder = if modern {
            "PowerShell"
        } else {
            "WindowsPowerShell"
        };
        documents.join(folder).join(PROFILE)
    } else {
        home.join(".config").join("powershell").join(PROFILE)
    }
}

/// The user's Documents folder, honouring OneDrive/known-folder redirection on Windows.
fn windows_documents_dir(home: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        use winreg::RegKey;
        use winreg::enums::HKEY_CURRENT_USER;

        let personal = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders")
            .and_then(|key| key.get_value::<String, _>("Personal"))
            .ok()
            .map(|value| expand_percent_vars(&value))
            .filter(|value| !value.is_empty());
        if let Some(path) = personal {
            return PathBuf::from(path);
        }
    }
    home.join("Documents")
}

/// Expand `%NAME%` references using the process environment (unknown names are left as-is).
#[cfg_attr(not(windows), allow(dead_code))]
fn expand_percent_vars(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if end > 0 => {
                let name = &after[..end];
                match std::env::var(name) {
                    Ok(resolved) => out.push_str(&resolved),
                    Err(_) => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Every profile file pyenv-native may have written, for uninstall and status checks.
pub fn known_profile_paths(home: &Path) -> Vec<(ShellKind, PathBuf)> {
    let mut paths = vec![
        (ShellKind::Bash, home.join(".bashrc")),
        (ShellKind::Zsh, home.join(".zshrc")),
        (ShellKind::Sh, home.join(".profile")),
        (
            ShellKind::Fish,
            home.join(".config").join("fish").join("config.fish"),
        ),
        (ShellKind::Pwsh, powershell_profile_path(home, true)),
    ];
    if cfg!(windows) {
        paths.push((ShellKind::Pwsh, powershell_profile_path(home, false)));
    }
    paths
}

/// Whether `content` already initialises pyenv, through our block or any other snippet.
pub fn has_pyenv_init(content: &str) -> bool {
    content.contains(BEGIN_MARKER) || content.contains("pyenv init")
}

fn posix_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn fish_quote(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn pwsh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// Render the delimited init block for `kind`, or `None` for shells without a profile.
///
/// `exe` is the installed `pyenv` executable and `bin_dir` its directory; the block puts the
/// latter on `PATH` (once) and evaluates `pyenv init`.
pub fn render_block(kind: ShellKind, exe: &Path, bin_dir: &Path) -> Option<String> {
    let exe = exe.to_string_lossy();
    let bin = bin_dir.to_string_lossy();
    let body = match kind {
        ShellKind::Bash | ShellKind::Zsh | ShellKind::Sh => {
            let (exe_q, bin_q) = (posix_quote(&exe), posix_quote(&bin));
            let shell = kind.as_str();
            format!(
                "if [ -x {exe_q} ]; then\n  case \":${{PATH}}:\" in\n    *:{bin_q}:*) ;;\n    *) export PATH={bin_q}:\"${{PATH}}\" ;;\n  esac\n  eval \"$({exe_q} init - {shell})\"\nfi"
            )
        }
        ShellKind::Fish => {
            let (exe_q, bin_q) = (fish_quote(&exe), fish_quote(&bin));
            format!(
                "if test -x {exe_q}\n  if not contains -- {bin_q} $PATH\n    set -gx PATH {bin_q} $PATH\n  end\n  {exe_q} init - fish | source\nend"
            )
        }
        ShellKind::Pwsh => {
            let exe_q = pwsh_quote(&exe);
            format!(
                "if (Test-Path {exe_q}) {{\n  $__pyenv_init = (& {exe_q} init - pwsh) -join \"`n\"\n  if ($__pyenv_init) {{ Invoke-Expression $__pyenv_init }}\n}}"
            )
        }
        ShellKind::Cmd | ShellKind::None => return Option::None,
    };
    Some(format!("{BEGIN_MARKER}\n{body}\n{END_MARKER}"))
}

/// Byte range of the marked block (from the begin marker through the end of the end marker line).
fn marked_block_range(content: &str) -> Option<(usize, usize)> {
    let start = content.find(BEGIN_MARKER)?;
    let end_marker = content[start..].find(END_MARKER)? + start;
    let mut end = end_marker + END_MARKER.len();
    if content[end..].starts_with("\r\n") {
        end += 2;
    } else if content[end..].starts_with('\n') {
        end += 1;
    }
    Some((start, end))
}

/// Insert or refresh `block` inside `existing`, preserving the file's line endings.
pub fn upsert_block(existing: &str, block: &str) -> (String, ProfileUpdate) {
    let eol = line_ending_for(existing);
    let block = normalize_block_eol(block, eol);

    if let Some((start, end)) = marked_block_range(existing) {
        let trailing_newline = existing[start..end].ends_with('\n');
        let mut replacement = block.clone();
        if trailing_newline {
            replacement.push_str(eol);
        }
        let mut updated = String::with_capacity(existing.len() + block.len());
        updated.push_str(&existing[..start]);
        updated.push_str(&replacement);
        updated.push_str(&existing[end..]);
        let outcome = if updated == existing {
            ProfileUpdate::Unchanged
        } else {
            ProfileUpdate::Updated
        };
        return (updated, outcome);
    }

    if existing.contains("pyenv init") {
        return (existing.to_string(), ProfileUpdate::AlreadyConfigured);
    }

    if existing.trim().is_empty() {
        return (format!("{block}{eol}"), ProfileUpdate::Created);
    }

    let mut updated = existing.trim_end().to_string();
    updated.push_str(eol);
    updated.push_str(eol);
    updated.push_str(&block);
    updated.push_str(eol);
    (updated, ProfileUpdate::Appended)
}

/// Remove the marked block (and any legacy GUI snippet). Returns the new content and whether
/// anything was removed.
pub fn remove_block(existing: &str) -> (String, bool) {
    let mut content = existing.to_string();
    let mut removed = false;

    while let Some((start, end)) = marked_block_range(&content) {
        // Swallow the blank separator line the installer put in front of the block.
        let mut cut_start = start;
        let before = &content[..start];
        for separator in ["\r\n\r\n", "\n\n"] {
            if before.ends_with(separator) {
                cut_start = start - separator.len() / 2;
                break;
            }
        }
        content.replace_range(cut_start..end, "");
        removed = true;
    }

    let (without_legacy, legacy_removed) = remove_legacy_gui_block(&content);
    if legacy_removed {
        content = without_legacy;
        removed = true;
    }

    (content, removed)
}

/// Drop the unmarked snippet older GUI builds appended (header line plus its init lines).
fn remove_legacy_gui_block(content: &str) -> (String, bool) {
    let eol = line_ending_for(content);
    let mut kept: Vec<&str> = Vec::new();
    let mut removed = false;
    let mut lines = content.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim() == LEGACY_GUI_HEADER {
            removed = true;
            while let Some(next) = lines.peek() {
                let trimmed = next.trim();
                if trimmed.contains("pyenv init") || trimmed.starts_with("if ($__pyenv_init)") {
                    lines.next();
                } else {
                    break;
                }
            }
            continue;
        }
        kept.push(line);
    }
    if !removed {
        return (content.to_string(), false);
    }
    let mut out = kept.join(eol);
    if content.ends_with('\n') && !out.is_empty() {
        out.push_str(eol);
    }
    (out, true)
}

/// Install or refresh the init block for `kind` in `path`.
pub fn apply_profile_block(
    kind: ShellKind,
    path: &Path,
    exe: &Path,
    bin_dir: &Path,
) -> io::Result<ProfileUpdate> {
    let Some(block) = render_block(kind, exe, bin_dir) else {
        return Ok(ProfileUpdate::Unchanged);
    };
    let existing = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    let existed = path.exists();
    let (mut updated, mut outcome) = upsert_block(&existing, &block);
    if matches!(outcome, ProfileUpdate::Created) && existed {
        outcome = ProfileUpdate::Appended;
    }
    if matches!(
        outcome,
        ProfileUpdate::Unchanged | ProfileUpdate::AlreadyConfigured
    ) {
        return Ok(outcome);
    }
    if cfg!(windows) && kind == ShellKind::Pwsh && !existed {
        updated = normalize_block_eol(&updated, "\r\n");
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, updated)?;
    Ok(outcome)
}

/// Remove the init block from `path`. Returns whether the file changed. A profile left empty by
/// the removal is deleted, since pyenv-native was the only thing that wrote to it.
pub fn remove_profile_block(path: &Path) -> io::Result<bool> {
    let existing = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    let (updated, removed) = remove_block(&existing);
    if !removed {
        return Ok(false);
    }
    if updated.trim().is_empty() {
        fs::remove_file(path)?;
    } else {
        fs::write(path, updated)?;
    }
    Ok(true)
}

/// Shell snippet that activates pyenv in the *current* shell without reopening the terminal.
pub fn current_shell_hint(kind: ShellKind, exe: &Path, bin_dir: &Path) -> Option<String> {
    let exe = exe.to_string_lossy();
    let bin = bin_dir.to_string_lossy();
    match kind {
        ShellKind::Bash | ShellKind::Zsh | ShellKind::Sh => {
            let (exe_q, bin_q) = (posix_quote(&exe), posix_quote(&bin));
            let shell = kind.as_str();
            Some(format!(
                "case \":${{PATH}}:\" in\n  *:{bin_q}:*) ;;\n  *) export PATH={bin_q}:\"${{PATH}}\" ;;\nesac\neval \"$({exe_q} init - {shell})\""
            ))
        }
        ShellKind::Fish => {
            let (exe_q, bin_q) = (fish_quote(&exe), fish_quote(&bin));
            Some(format!(
                "if not contains -- {bin_q} $PATH\n  set -gx PATH {bin_q} $PATH\nend\n{exe_q} init - fish | source"
            ))
        }
        ShellKind::Pwsh | ShellKind::Cmd | ShellKind::None => Option::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn block(kind: ShellKind) -> String {
        render_block(
            kind,
            Path::new("/home/u/.pyenv/bin/pyenv"),
            Path::new("/home/u/.pyenv/bin"),
        )
        .expect("block")
    }

    #[test]
    fn parses_shell_names() {
        assert_eq!(ShellKind::parse("BASH"), Some(ShellKind::Bash));
        assert_eq!(ShellKind::parse("powershell"), Some(ShellKind::Pwsh));
        assert_eq!(ShellKind::parse("none"), Some(ShellKind::None));
        assert_eq!(ShellKind::parse("tcsh"), Option::None);
        assert!(ShellKind::Zsh.has_profile());
        assert!(!ShellKind::Cmd.has_profile());
    }

    #[test]
    fn blocks_are_delimited_and_name_the_shell() {
        for kind in [
            ShellKind::Bash,
            ShellKind::Zsh,
            ShellKind::Sh,
            ShellKind::Fish,
            ShellKind::Pwsh,
        ] {
            let text = block(kind);
            assert!(text.starts_with(BEGIN_MARKER), "{kind:?}");
            assert!(text.ends_with(END_MARKER), "{kind:?}");
            assert!(
                text.contains(&format!("init - {}", kind.as_str())),
                "{kind:?}"
            );
        }
        assert!(render_block(ShellKind::Cmd, Path::new("x"), Path::new("y")).is_none());
    }

    #[test]
    fn quoting_survives_single_quotes_in_paths() {
        let text = render_block(
            ShellKind::Bash,
            Path::new("/home/o'brien/.pyenv/bin/pyenv"),
            Path::new("/home/o'brien/.pyenv/bin"),
        )
        .unwrap();
        assert!(text.contains("'/home/o'\\''brien/.pyenv/bin/pyenv'"));
        let text = render_block(
            ShellKind::Pwsh,
            Path::new("C:\\Users\\o'brien\\.pyenv\\bin\\pyenv.exe"),
            Path::new("C:\\Users\\o'brien\\.pyenv\\bin"),
        )
        .unwrap();
        assert!(text.contains("o''brien"));
    }

    #[test]
    fn upsert_creates_appends_and_is_idempotent() {
        let b = block(ShellKind::Bash);
        let (created, outcome) = upsert_block("", &b);
        assert_eq!(outcome, ProfileUpdate::Created);
        assert_eq!(created, format!("{b}\n"));

        let (appended, outcome) = upsert_block("export FOO=1\n", &b);
        assert_eq!(outcome, ProfileUpdate::Appended);
        assert_eq!(appended, format!("export FOO=1\n\n{b}\n"));

        let (again, outcome) = upsert_block(&appended, &b);
        assert_eq!(outcome, ProfileUpdate::Unchanged);
        assert_eq!(again, appended);
    }

    #[test]
    fn upsert_refreshes_a_stale_block_in_place() {
        let old = format!("a\n\n{BEGIN_MARKER}\nold\n{END_MARKER}\nz\n");
        let b = block(ShellKind::Bash);
        let (updated, outcome) = upsert_block(&old, &b);
        assert_eq!(outcome, ProfileUpdate::Updated);
        assert!(updated.starts_with("a\n\n"));
        assert!(updated.ends_with("\nz\n"));
        assert!(!updated.contains("old"));
        assert_eq!(updated.matches(BEGIN_MARKER).count(), 1);
    }

    #[test]
    fn upsert_respects_foreign_init_snippets() {
        let existing = "eval \"$(pyenv init - bash)\"\n";
        let (updated, outcome) = upsert_block(existing, &block(ShellKind::Bash));
        assert_eq!(outcome, ProfileUpdate::AlreadyConfigured);
        assert_eq!(updated, existing);
    }

    #[test]
    fn upsert_preserves_crlf_files() {
        let existing = "line1\r\nline2\r\n";
        let (updated, _) = upsert_block(existing, &block(ShellKind::Pwsh));
        assert!(updated.contains("\r\n"));
        assert!(!updated.replace("\r\n", "").contains('\n'));
    }

    #[test]
    fn unterminated_block_is_never_swallowed() {
        let existing = format!("keep me\n{BEGIN_MARKER}\nno end marker\nmore user content\n");
        let (updated, outcome) = upsert_block(&existing, &block(ShellKind::Bash));
        assert_eq!(outcome, ProfileUpdate::Appended);
        assert!(updated.contains("more user content"));
        let (removed, changed) = remove_block(&existing);
        assert!(!changed);
        assert_eq!(removed, existing);
    }

    #[test]
    fn remove_block_restores_the_original_profile() {
        let original = "export FOO=1\nalias ll='ls -l'\n";
        let (with_block, _) = upsert_block(original, &block(ShellKind::Bash));
        let (restored, removed) = remove_block(&with_block);
        assert!(removed);
        assert_eq!(restored.trim_end(), original.trim_end());
    }

    #[test]
    fn remove_block_strips_legacy_gui_snippets() {
        let bash =
            "export A=1\n\n# pyenv-native shell initialization\neval \"$(pyenv init - bash)\"\n";
        let (restored, removed) = remove_block(bash);
        assert!(removed);
        assert!(!restored.contains("pyenv"));
        assert!(restored.contains("export A=1"));

        let pwsh = "$x = 1\n\n# pyenv-native shell initialization\n$__pyenv_init = (pyenv init - pwsh) -join \"`n\"\nif ($__pyenv_init) { Invoke-Expression $__pyenv_init }\n";
        let (restored, removed) = remove_block(pwsh);
        assert!(removed);
        assert!(!restored.contains("pyenv"));
        assert!(restored.contains("$x = 1"));
    }

    #[test]
    fn file_round_trip_creates_and_deletes_profiles() {
        let temp = TempDir::new().expect("tempdir");
        let path = temp.path().join("nested").join(".bashrc");
        let exe = Path::new("/r/bin/pyenv");
        let bin = Path::new("/r/bin");

        let outcome = apply_profile_block(ShellKind::Bash, &path, exe, bin).expect("apply");
        assert_eq!(outcome, ProfileUpdate::Created);
        assert!(has_pyenv_init(&fs::read_to_string(&path).unwrap()));
        assert_eq!(
            apply_profile_block(ShellKind::Bash, &path, exe, bin).unwrap(),
            ProfileUpdate::Unchanged
        );

        assert!(remove_profile_block(&path).expect("remove"));
        assert!(
            !path.exists(),
            "a profile that only held our block is deleted"
        );
        assert!(!remove_profile_block(&path).expect("remove again"));
    }

    #[test]
    fn removal_keeps_profiles_with_user_content() {
        let temp = TempDir::new().expect("tempdir");
        let path = temp.path().join(".zshrc");
        fs::write(&path, "export KEEP=1\n").unwrap();
        apply_profile_block(
            ShellKind::Zsh,
            &path,
            Path::new("/r/bin/pyenv"),
            Path::new("/r/bin"),
        )
        .unwrap();
        assert!(remove_profile_block(&path).unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap().trim(), "export KEEP=1");
    }

    #[test]
    fn expands_percent_variables() {
        // SAFETY: tests in this module do not read this variable concurrently.
        unsafe { std::env::set_var("PYENV_TEST_EXPAND", "C:\\Users\\Roy") };
        assert_eq!(
            expand_percent_vars("%PYENV_TEST_EXPAND%\\Documents"),
            "C:\\Users\\Roy\\Documents"
        );
        assert_eq!(
            expand_percent_vars("%NOPE_NOT_SET%\\x"),
            "%NOPE_NOT_SET%\\x"
        );
        assert_eq!(expand_percent_vars("100%"), "100%");
    }
}
