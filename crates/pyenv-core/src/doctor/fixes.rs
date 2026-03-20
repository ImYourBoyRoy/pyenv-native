// ./crates/pyenv-core/src/doctor/fixes.rs
//! Automated and manual doctor fix planning for shell, PATH, and source-build issues.

use std::env;
use std::path::{Path, PathBuf};

use crate::context::{AppContext, is_pyenv_win_root};
use crate::error::PyenvError;
use crate::executable::find_system_python_command;
use crate::runtime::search_path_entries;
use crate::shim::rehash_shims;
use crate::version::resolve_selected_versions;

use super::helpers::{
    is_termux_environment, path_contains, path_ext_for_platform, shell_init_hint,
};
use super::types::{DoctorFix, DoctorFixOutcome};

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

pub fn apply_doctor_fixes(ctx: &AppContext) -> Result<DoctorFixOutcome, PyenvError> {
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
                PyenvError::Io(format!(
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
