// ./crates/pyenv-core/src/doctor.rs
//! Health and diagnostics reporting for common pyenv-native configuration issues.

use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::catalog::installed_version_names;
use crate::command::CommandReport;
use crate::context::{AppContext, is_pyenv_win_root};
use crate::executable::find_system_python_command;
use crate::install::resolve_python_build_path;
use crate::runtime::search_path_entries;
use crate::shim::rehash_shims;
use crate::version::resolve_selected_versions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum DoctorStatus {
    Ok,
    Warn,
    Info,
}

impl DoctorStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Warn => "WARN",
            Self::Info => "INFO",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DoctorCheck {
    name: String,
    status: DoctorStatus,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DoctorReport {
    root: String,
    platform: String,
    installed_versions: usize,
    checks: Vec<DoctorCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorFix {
    pub key: String,
    pub automated: bool,
    pub description: String,
    pub command_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorFixOutcome {
    pub applied: Vec<String>,
    pub manual: Vec<DoctorFix>,
}

pub fn cmd_doctor(ctx: &AppContext, json: bool) -> CommandReport {
    let checks = collect_checks(ctx);
    let installed_versions = installed_version_names(ctx)
        .map(|items| items.len())
        .unwrap_or(0);
    let report = DoctorReport {
        root: ctx.root.display().to_string(),
        platform: env::consts::OS.to_string(),
        installed_versions,
        checks,
    };

    if json {
        return match serde_json::to_string_pretty(&report) {
            Ok(payload) => CommandReport::success(payload.lines().map(ToOwned::to_owned).collect()),
            Err(error) => CommandReport::failure(
                vec![format!("pyenv: failed to serialize doctor output: {error}")],
                1,
            ),
        };
    }

    let mut stdout = vec![
        format!("pyenv root: {}", report.root),
        format!("platform: {}", report.platform),
        format!("installed versions: {}", report.installed_versions),
        String::new(),
    ];
    stdout.extend(report.checks.into_iter().map(|check| {
        format!(
            "[{}] {}: {}",
            check.status.label(),
            check.name,
            check.detail
        )
    }));
    CommandReport::success(stdout)
}

pub fn doctor_fix_plan(ctx: &AppContext) -> Vec<DoctorFix> {
    let mut fixes = Vec::new();
    let platform = env::consts::OS;

    if !ctx.root.exists()
        || !ctx.versions_dir().is_dir()
        || !ctx.shims_dir().is_dir()
        || !ctx.cache_dir().is_dir()
    {
        fixes.push(DoctorFix {
            key: "ensure-managed-layout".to_string(),
            automated: true,
            description: format!(
                "Create missing managed directories under {}",
                ctx.root.display()
            ),
            command_hint: None,
        });
    }

    fixes.push(DoctorFix {
        key: "rehash-shims".to_string(),
        automated: true,
        description: format!("Refresh shim launchers under {}", ctx.shims_dir().display()),
        command_hint: Some("Equivalent to `pyenv rehash`".to_string()),
    });

    if !path_contains(ctx.path_env.as_ref(), &ctx.shims_dir()) {
        fixes.push(DoctorFix {
            key: "path-shims-manual".to_string(),
            automated: false,
            description: format!(
                "Add {} to your shell PATH so python/pip resolve through pyenv shims",
                ctx.shims_dir().display()
            ),
            command_hint: Some(shell_init_hint(ctx, platform)),
        });
    }

    if !path_contains(
        ctx.path_env.as_ref(),
        &ctx.exe_path
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| ctx.root.join("bin")),
    ) {
        fixes.push(DoctorFix {
            key: "path-bin-manual".to_string(),
            automated: false,
            description: "Add the pyenv bin directory to your shell PATH".to_string(),
            command_hint: Some(match platform {
                "windows" => "Re-run the Windows installer or prepend PYENV_ROOT\\bin to your User PATH".to_string(),
                _ => "Install pyenv with the web installer or add $PYENV_ROOT/bin in your shell profile before evaluating `pyenv init`".to_string(),
            }),
        });
    }

    fixes.extend(selection_manual_fixes(ctx));

    if platform == "windows" {
        if let Ok(env_root) = env::var("PYENV_ROOT")
            && is_pyenv_win_root(Path::new(&env_root))
        {
            fixes.push(DoctorFix {
                key: "pyenv-win-root-manual".to_string(),
                automated: false,
                description: "Remove the stale pyenv-win PYENV_ROOT environment variable"
                    .to_string(),
                command_hint: Some(
                    "Delete PYENV_ROOT from your User environment variables".to_string(),
                ),
            });
        }

        if let Some(path) = find_system_python_command(ctx)
            && path.to_string_lossy().contains("WindowsApps")
        {
            fixes.push(DoctorFix {
                key: "windows-store-alias-manual".to_string(),
                automated: false,
                description:
                    "Disable the Microsoft Store Python App Execution Alias to avoid PATH interception"
                        .to_string(),
                command_hint: Some(
                    "Settings > Apps > Advanced app settings > App execution aliases"
                        .to_string(),
                ),
            });
        }
    } else {
        fixes.extend(non_windows_manual_dependency_fixes(ctx, platform));
    }

    fixes
}

pub fn apply_doctor_fixes(ctx: &AppContext) -> Result<DoctorFixOutcome, crate::error::PyenvError> {
    let manual = doctor_fix_plan(ctx)
        .into_iter()
        .filter(|item| !item.automated)
        .collect::<Vec<_>>();
    let mut applied = Vec::new();

    for path in [
        ctx.root.clone(),
        ctx.root.join("bin"),
        ctx.shims_dir(),
        ctx.versions_dir(),
        ctx.cache_dir(),
    ] {
        if !path.exists() {
            std::fs::create_dir_all(&path).map_err(|error| {
                crate::error::PyenvError::Io(format!(
                    "pyenv: failed to create {}: {error}",
                    path.display()
                ))
            })?;
        }
    }
    applied.push(format!(
        "Ensured the managed directory layout exists under {}",
        ctx.root.display()
    ));

    let count = rehash_shims(ctx)?;
    applied.push(format!(
        "Refreshed {count} shim command(s) under {}",
        ctx.shims_dir().display()
    ));

    Ok(DoctorFixOutcome { applied, manual })
}

fn collect_checks(ctx: &AppContext) -> Vec<DoctorCheck> {
    collect_checks_for_platform(ctx, env::consts::OS)
}

fn collect_checks_for_platform(ctx: &AppContext, platform: &str) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();
    let exe_dir = ctx
        .exe_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| ctx.root.join("bin"));
    let shims_dir = ctx.shims_dir();
    let versions_dir = ctx.versions_dir();

    checks.push(DoctorCheck {
        name: "root-directory".to_string(),
        status: if ctx.root.exists() {
            DoctorStatus::Ok
        } else {
            DoctorStatus::Warn
        },
        detail: if ctx.root.exists() {
            format!("root exists at {}", ctx.root.display())
        } else {
            format!("root does not exist yet at {}", ctx.root.display())
        },
    });

    checks.push(DoctorCheck {
        name: "pyenv-bin-on-path".to_string(),
        status: if path_contains(ctx.path_env.as_ref(), &exe_dir) {
            DoctorStatus::Ok
        } else {
            DoctorStatus::Warn
        },
        detail: format!("expected {} on PATH", exe_dir.display()),
    });

    checks.push(DoctorCheck {
        name: "shims-on-path".to_string(),
        status: if path_contains(ctx.path_env.as_ref(), &shims_dir) {
            DoctorStatus::Ok
        } else {
            DoctorStatus::Warn
        },
        detail: format!("expected {} on PATH", shims_dir.display()),
    });

    checks.push(DoctorCheck {
        name: "versions-directory".to_string(),
        status: if versions_dir.is_dir() {
            DoctorStatus::Ok
        } else {
            DoctorStatus::Info
        },
        detail: format!("managed runtimes live under {}", versions_dir.display()),
    });

    let selected = resolve_selected_versions(ctx, false);
    let selected_detail = if selected.versions.is_empty() {
        "no selected versions".to_string()
    } else {
        format!("{} (from {})", selected.versions.join(" "), selected.origin)
    };
    checks.push(DoctorCheck {
        name: "selected-version".to_string(),
        status: if selected.missing.is_empty() {
            DoctorStatus::Info
        } else {
            DoctorStatus::Warn
        },
        detail: selected_detail,
    });

    if platform == "windows" {
        checks.extend(pyenv_win_conflict_checks(ctx));
        checks.push(windows_store_alias_check(ctx));
    } else {
        checks.extend(non_windows_source_build_checks(ctx, platform));
    }

    checks.extend(selected_env_checks(&selected));

    checks
}

fn selected_env_checks(selected: &crate::version::SelectedVersions) -> Vec<DoctorCheck> {
    selected
        .missing
        .iter()
        .filter(|value| value.contains("/envs/") || value.contains("\\envs\\"))
        .map(|value| DoctorCheck {
            name: "managed-venv-selection".to_string(),
            status: DoctorStatus::Warn,
            detail: format!(
                "selected managed venv `{value}` is missing; run `pyenv venv list` to inspect available envs or `pyenv venv create <runtime> <name>` to recreate it"
            ),
        })
        .collect()
}

fn pyenv_win_conflict_checks(ctx: &AppContext) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();

    // Check if PYENV_ROOT env var still points to pyenv-win
    if let Ok(env_root) = env::var("PYENV_ROOT")
        && is_pyenv_win_root(Path::new(&env_root))
    {
        checks.push(DoctorCheck {
            name: "pyenv-win-root-conflict".to_string(),
            status: DoctorStatus::Warn,
            detail: format!(
                "PYENV_ROOT is set to `{}` which looks like a pyenv-win path; \
                     pyenv-native overrides this at runtime, but removing the env var \
                     is recommended: remove PYENV_ROOT from your User environment variables",
                env_root
            ),
        });
    }

    // Check if pyenv-win bin/shims appear on PATH before the native ones
    let exe_dir = ctx
        .exe_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| ctx.root.join("bin"));
    let shims_dir = ctx.shims_dir();

    if let Some(path_env) = ctx.path_env.as_ref() {
        let entries: Vec<PathBuf> = env::split_paths(path_env).collect();
        let native_bin_pos = entries
            .iter()
            .position(|entry| paths_equal(entry, &exe_dir));
        let native_shims_pos = entries
            .iter()
            .position(|entry| paths_equal(entry, &shims_dir));

        let pyenv_win_pos = entries.iter().position(|entry| {
            let s = entry.to_string_lossy().to_ascii_lowercase();
            s.contains("pyenv-win")
                && (s.ends_with("bin")
                    || s.ends_with("bin\\")
                    || s.ends_with("bin/")
                    || s.ends_with("shims")
                    || s.ends_with("shims\\")
                    || s.ends_with("shims/"))
        });

        if let Some(pw_pos) = pyenv_win_pos {
            let shadowed = native_bin_pos.is_none_or(|nb| pw_pos < nb)
                || native_shims_pos.is_none_or(|ns| pw_pos < ns);
            if shadowed {
                checks.push(DoctorCheck {
                    name: "pyenv-win-path-conflict".to_string(),
                    status: DoctorStatus::Warn,
                    detail: format!(
                        "pyenv-win PATH entries appear before pyenv-native in PATH; \
                         this can cause pyenv-win to intercept commands. \
                         Remove pyenv-win entries from your User PATH: {}",
                        entries[pw_pos].display()
                    ),
                });
            }
        }
    }

    checks
}

fn windows_store_alias_check(ctx: &AppContext) -> DoctorCheck {
    let detail = match find_system_python_command(ctx) {
        Some(path) if path.to_string_lossy().contains("WindowsApps") => format!(
            "system python resolves to WindowsApps alias at {}; \
             this 'trap' can intercept commands and should be disabled in \
             'Settings > Apps > App Execution Aliases'",
            path.display()
        ),
        Some(path) => format!("system python resolves to {}", path.display()),
        None => "no system python found on PATH".to_string(),
    };
    let status = if detail.contains("WindowsApps") {
        DoctorStatus::Warn
    } else {
        DoctorStatus::Info
    };
    DoctorCheck {
        name: "system-python".to_string(),
        status,
        detail,
    }
}

fn non_windows_python_build_check(ctx: &AppContext) -> DoctorCheck {
    match resolve_python_build_path(ctx) {
        Ok(path) => DoctorCheck {
            name: "python-build-backend".to_string(),
            status: DoctorStatus::Ok,
            detail: format!("python-build available at {}", path.display()),
        },
        Err(error) => DoctorCheck {
            name: "python-build-backend".to_string(),
            status: DoctorStatus::Info,
            detail: format!("{error} (native CPython source builds do not require it)"),
        },
    }
}

fn non_windows_source_build_checks(ctx: &AppContext, platform: &str) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();

    checks.push(command_presence_check(
        ctx,
        "source-build-shell",
        &["sh", "bash"],
        "required for configure-script execution",
        platform,
    ));
    checks.push(command_presence_check(
        ctx,
        "source-build-make",
        &["make", "gmake"],
        "required for native CPython source builds",
        platform,
    ));
    checks.push(command_presence_check(
        ctx,
        "source-build-compiler",
        &["cc", "clang", "gcc"],
        "required for native CPython source builds",
        platform,
    ));

    let pkg_config_status = command_presence_check(
        ctx,
        "source-build-pkg-config",
        &["pkg-config"],
        "recommended for locating native library dependencies",
        platform,
    );
    checks.push(DoctorCheck {
        status: match pkg_config_status.status {
            DoctorStatus::Warn => DoctorStatus::Info,
            status => status,
        },
        ..pkg_config_status
    });

    let toolchain_missing = checks
        .iter()
        .any(|check| check.status == DoctorStatus::Warn);
    checks.push(DoctorCheck {
        name: "source-build-readiness".to_string(),
        status: if toolchain_missing {
            DoctorStatus::Warn
        } else {
            DoctorStatus::Ok
        },
        detail: if toolchain_missing {
            format!(
                "{platform} source builds may fail until the required shell, make, and compiler tooling are available"
            )
        } else {
            format!("{platform} source-build prerequisites look available on PATH")
        },
    });

    checks.push(non_windows_python_build_check(ctx));
    checks
}

fn non_windows_manual_dependency_fixes(ctx: &AppContext, platform: &str) -> Vec<DoctorFix> {
    let directories = ctx
        .path_env
        .as_ref()
        .map(env::split_paths)
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let missing_shell =
        search_path_entries(&directories, "sh", path_ext_for_platform(ctx, platform)).is_none()
            && search_path_entries(&directories, "bash", path_ext_for_platform(ctx, platform))
                .is_none();
    let missing_make =
        search_path_entries(&directories, "make", path_ext_for_platform(ctx, platform)).is_none()
            && search_path_entries(&directories, "gmake", path_ext_for_platform(ctx, platform))
                .is_none();
    let missing_compiler =
        search_path_entries(&directories, "cc", path_ext_for_platform(ctx, platform)).is_none()
            && search_path_entries(&directories, "clang", path_ext_for_platform(ctx, platform))
                .is_none()
            && search_path_entries(&directories, "gcc", path_ext_for_platform(ctx, platform))
                .is_none();

    if !(missing_shell || missing_make || missing_compiler) {
        return Vec::new();
    }

    let command_hint = if is_termux_environment() {
        "pkg install clang make pkg-config libffi openssl readline sqlite zlib bzip2 xz".to_string()
    } else if platform == "macos" {
        "xcode-select --install && brew install pkg-config openssl readline sqlite3 xz zlib"
            .to_string()
    } else {
        "Install a POSIX shell, make, compiler toolchain, and development headers for OpenSSL/readline/sqlite/zlib".to_string()
    };

    vec![DoctorFix {
        key: format!("{platform}-source-build-deps-manual"),
        automated: false,
        description: format!(
            "Install the native source-build prerequisites required on {platform}"
        ),
        command_hint: Some(command_hint),
    }]
}

fn selection_manual_fixes(ctx: &AppContext) -> Vec<DoctorFix> {
    let selected = resolve_selected_versions(ctx, false);
    selected
        .missing
        .into_iter()
        .filter(|value| value.contains("/envs/") || value.contains("\\envs\\"))
        .map(|value| DoctorFix {
            key: format!("missing-managed-venv-{value}"),
            automated: false,
            description: format!(
                "Recreate or repoint the missing managed venv selection `{value}`"
            ),
            command_hint: Some(format!(
                "Use `pyenv venv list`, `pyenv venv info {value}`, or update `.python-version` with `pyenv venv use <name>`"
            )),
        })
        .collect()
}

fn shell_init_hint(ctx: &AppContext, platform: &str) -> String {
    match platform {
        "windows" => match ctx.env_shell.as_deref() {
            Some("cmd") => {
                "Add `for /f \"delims=\" %i in ('pyenv init - cmd') do %i` to your shell startup or rerun the Windows installer".to_string()
            }
            _ => "Add `iex ((pyenv init - pwsh) -join \"`n\")` to your PowerShell profile or rerun the Windows installer".to_string(),
        },
        _ => match ctx.env_shell.as_deref() {
            Some("zsh") => "Add `eval \"$(pyenv init - zsh)\"` to ~/.zshrc".to_string(),
            Some("fish") => "Add `pyenv init - fish | source` to your Fish config".to_string(),
            Some("sh") => "Add `eval \"$(pyenv init - sh)\"` to your shell profile".to_string(),
            _ => "Add `eval \"$(pyenv init - bash)\"` to ~/.bashrc (or the equivalent profile for your shell)".to_string(),
        },
    }
}

fn is_termux_environment() -> bool {
    env::var_os("TERMUX_VERSION").is_some()
        || env::var_os("PREFIX")
            .map(|value| value.to_string_lossy().contains("/data/data/com.termux"))
            .unwrap_or(false)
}

fn command_presence_check(
    ctx: &AppContext,
    name: &str,
    commands: &[&str],
    missing_detail: &str,
    platform: &str,
) -> DoctorCheck {
    let directories = ctx
        .path_env
        .as_ref()
        .map(env::split_paths)
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    for command in commands {
        if let Some(path) =
            search_path_entries(&directories, command, path_ext_for_platform(ctx, platform))
        {
            return DoctorCheck {
                name: name.to_string(),
                status: DoctorStatus::Ok,
                detail: format!("{} available at {}", command, path.display()),
            };
        }
    }

    DoctorCheck {
        name: name.to_string(),
        status: DoctorStatus::Warn,
        detail: format!("{}; searched for {}", missing_detail, commands.join(", ")),
    }
}

fn path_ext_for_platform<'a>(ctx: &'a AppContext, platform: &str) -> Option<&'a OsStr> {
    if platform == "windows" {
        ctx.path_ext.as_deref()
    } else {
        None
    }
}

fn path_contains(path_env: Option<&std::ffi::OsString>, target: &Path) -> bool {
    path_env
        .map(env::split_paths)
        .into_iter()
        .flatten()
        .any(|entry| paths_equal(&entry, target))
}

fn paths_equal(lhs: &Path, rhs: &Path) -> bool {
    if cfg!(windows) {
        lhs.to_string_lossy()
            .replace('/', "\\")
            .eq_ignore_ascii_case(&rhs.to_string_lossy().replace('/', "\\"))
    } else {
        lhs == rhs
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::ffi::OsString;
    use std::fs;

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::{cmd_doctor, collect_checks_for_platform};

    fn test_path_ext() -> Option<OsString> {
        if cfg!(windows) {
            Some(OsString::from(".exe;.cmd"))
        } else {
            None
        }
    }

    fn test_context() -> (TempDir, AppContext) {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        let dir = temp.path().join("work");
        fs::create_dir_all(root.join("versions")).expect("versions dir");
        fs::create_dir_all(root.join("shims")).expect("shims dir");
        fs::create_dir_all(root.join("bin")).expect("bin dir");
        fs::create_dir_all(&dir).expect("work dir");

        let ctx = AppContext {
            root: root.clone(),
            dir,
            exe_path: root
                .join("bin")
                .join(if cfg!(windows) { "pyenv.exe" } else { "pyenv" }),
            env_version: Some("3.12.10".to_string()),
            env_shell: None,
            path_env: Some(
                env::join_paths([root.join("bin"), root.join("shims")]).expect("path env"),
            ),
            path_ext: test_path_ext(),
            config: AppConfig::default(),
        };

        (temp, ctx)
    }

    #[test]
    fn doctor_reports_ok_for_bin_and_shims_on_path() {
        let (_temp, ctx) = test_context();
        let report = cmd_doctor(&ctx, false);
        assert_eq!(report.exit_code, 0);
        assert!(
            report
                .stdout
                .iter()
                .any(|line| line.contains("[OK] pyenv-bin-on-path"))
        );
        assert!(
            report
                .stdout
                .iter()
                .any(|line| line.contains("[OK] shims-on-path"))
        );
    }

    #[test]
    fn doctor_json_includes_checks() {
        let (_temp, mut ctx) = test_context();
        ctx.path_env = Some(OsString::from(String::new()));
        let report = cmd_doctor(&ctx, true);
        assert_eq!(report.exit_code, 0);
        let payload = report.stdout.join("\n");
        assert!(payload.contains("\"checks\""));
        assert!(payload.contains("\"pyenv-bin-on-path\""));
    }

    #[test]
    fn non_windows_doctor_reports_source_build_readiness() {
        let (_temp, ctx) = test_context();
        let checks = collect_checks_for_platform(&ctx, "linux");
        assert!(
            checks
                .iter()
                .any(|check| check.name == "source-build-shell")
        );
        assert!(checks.iter().any(|check| check.name == "source-build-make"));
        assert!(
            checks
                .iter()
                .any(|check| check.name == "source-build-compiler")
        );
        assert!(
            checks
                .iter()
                .any(|check| check.name == "source-build-readiness")
        );
    }

    #[test]
    fn non_windows_doctor_treats_missing_python_build_as_info() {
        let (_temp, mut ctx) = test_context();
        ctx.path_env = Some(OsString::from(String::new()));
        let checks = collect_checks_for_platform(&ctx, "macos");
        let python_build = checks
            .iter()
            .find(|check| check.name == "python-build-backend")
            .expect("python-build check");
        assert_eq!(python_build.status, super::DoctorStatus::Info);
    }
}
