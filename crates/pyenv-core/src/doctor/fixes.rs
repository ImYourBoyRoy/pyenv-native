// ./crates/pyenv-core/src/doctor/fixes.rs
//! Automated and manual doctor fix planning for shell, PATH, and source-build issues.

use std::collections::HashMap;
use std::env;
use std::path::Path;

use crate::context::{AppContext, is_pyenv_win_root};
use crate::error::PyenvError;
use crate::runtime::search_path_entries;
use crate::shim::rehash_shims;
use crate::version::resolve_selected_versions;

use super::helpers::{
    is_termux_environment, path_contains, path_ext_for_platform, shell_init_hint,
    user_shell_profiles_configured,
};
use super::types::{DoctorFix, DoctorFixOutcome, DoctorOptions};

fn fix_text(id: &str) -> String {
    crate::lookup_i18n(id)
}

fn fix_text_arg(id: &str, key: &str, value: impl ToString) -> String {
    let mut args = HashMap::new();
    args.insert(key.to_string(), value.to_string());
    crate::lookup_i18n_args(id, &args)
}

pub fn doctor_fix_plan(ctx: &AppContext) -> Vec<DoctorFix> {
    doctor_fix_plan_with_options(ctx, DoctorOptions::default())
}

pub fn doctor_fix_plan_with_options(ctx: &AppContext, options: DoctorOptions) -> Vec<DoctorFix> {
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
            description: fix_text_arg("doctor-fix-ensure-layout", "path", ctx.root.display()),
            command_hint: None,
        });
    }

    fixes.push(DoctorFix {
        key: "rehash-shims".to_string(),
        automated: true,
        description: fix_text_arg("doctor-fix-rehash", "path", ctx.shims_dir().display()),
        command_hint: Some("Equivalent to `pyenv rehash`".to_string()),
    });

    let skip_process_path = options.desktop_session
        && options
            .profiles_configured
            .unwrap_or_else(user_shell_profiles_configured);

    if !skip_process_path && !path_contains(ctx.path_env.as_ref(), &ctx.shims_dir()) {
        fixes.push(DoctorFix {
            key: "path-shims-manual".to_string(),
            automated: false,
            description: fix_text_arg("doctor-fix-path-shims", "path", ctx.shims_dir().display()),
            command_hint: Some(shell_init_hint(ctx, platform)),
        });
    }

    if !skip_process_path && !path_contains(ctx.path_env.as_ref(), &ctx.bin_dir()) {
        fixes.push(DoctorFix {
            key: "path-bin-manual".to_string(),
            automated: false,
            description: fix_text("doctor-fix-path-bin"),
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
                description: fix_text("doctor-fix-pyenv-win-root"),
                command_hint: Some(
                    "Delete PYENV_ROOT from your User environment variables".to_string(),
                ),
            });
        }

        if super::helpers::windows_store_alias_needs_manual_fix(ctx) {
            fixes.push(DoctorFix {
                key: "windows-store-alias-manual".to_string(),
                automated: false,
                description: fix_text("doctor-fix-store-alias"),
                command_hint: Some(
                    "Settings > Apps > Advanced app settings > App execution aliases".to_string(),
                ),
            });
        }

        if !crate::plugin::powershell_7_available(ctx) {
            let winget_available = crate::executable::find_system_command(ctx, "winget").is_some();
            fixes.push(DoctorFix {
                key: "install-powershell-7".to_string(),
                automated: winget_available,
                description: fix_text("doctor-fix-pwsh7"),
                command_hint: Some(
                    "winget install --id Microsoft.PowerShell --accept-package-agreements --accept-source-agreements"
                        .to_string(),
                ),
            });
        }
    } else {
        fixes.extend(non_windows_manual_dependency_fixes(ctx, platform));

        if platform == "macos" {
            fixes.extend(macos_toolchain_fixes());
        }

        // Add Termux automated repair if compile dependencies are missing
        if is_termux_environment() {
            use crate::preflight::{inspect_android_toolchain, termux_required_pkg_packages};

            let state = inspect_android_toolchain();
            if !state.ready {
                let packages = termux_required_pkg_packages().join(" ");
                fixes.push(DoctorFix {
                    key: "termux-compile-deps".to_string(),
                    automated: true,
                    description: fix_text_arg(
                        "doctor-fix-termux-deps",
                        "packages",
                        if state.missing.is_empty() {
                            packages.clone()
                        } else {
                            state.missing.join(", ")
                        },
                    ),
                    command_hint: Some(format!("pkg install {packages} -y")),
                });
            }
        }
    }

    fixes
}

fn macos_toolchain_fixes() -> Vec<DoctorFix> {
    use crate::preflight::inspect_macos_toolchain;

    let state = inspect_macos_toolchain();
    let mut fixes = Vec::new();
    if !state.clt_ok {
        fixes.push(DoctorFix {
            key: "macos-xcode-clt".to_string(),
            automated: true,
            description: fix_text("doctor-fix-xcode-clt"),
            command_hint: Some(
                "Prefer automated `softwareupdate` CLT install; falls back to `xcode-select --install` (may show a system dialog). Full Xcode.app still requires the App Store.".to_string(),
            ),
        });
    }
    if state.openssl_prefix.is_none() {
        fixes.push(DoctorFix {
            key: "macos-openssl-brew".to_string(),
            automated: true,
            description: fix_text("doctor-fix-openssl-brew"),
            command_hint: Some(
                if state.brew_available {
                    "brew install openssl@3 pkg-config readline sqlite3 xz zlib bzip2".to_string()
                } else {
                    "Install Homebrew from https://brew.sh then run: brew install openssl@3 pkg-config readline sqlite3 xz zlib bzip2".to_string()
                },
            ),
        });
    }
    fixes
}

pub fn apply_doctor_fixes(ctx: &AppContext) -> Result<DoctorFixOutcome, PyenvError> {
    apply_doctor_fixes_with_options(ctx, DoctorOptions::default())
}

pub fn apply_doctor_fixes_with_options(
    ctx: &AppContext,
    options: DoctorOptions,
) -> Result<DoctorFixOutcome, PyenvError> {
    let plan = doctor_fix_plan_with_options(ctx, options);
    let has_termux_fix = plan.iter().any(|f| f.key == "termux-compile-deps");
    let has_macos_clt_fix = plan.iter().any(|f| f.key == "macos-xcode-clt");
    let has_macos_openssl_fix = plan.iter().any(|f| f.key == "macos-openssl-brew");
    let has_powershell_7_fix = plan
        .iter()
        .any(|f| f.key == "install-powershell-7" && f.automated);
    let manual = plan
        .into_iter()
        .filter(|item| !item.automated)
        .collect::<Vec<_>>();
    let mut applied = Vec::new();

    if has_powershell_7_fix {
        let message = install_powershell_7()?;
        applied.push(message);
    }

    if has_macos_clt_fix {
        match crate::preflight::try_install_or_update_macos_clt() {
            Ok(message) => applied.push(message),
            Err(error) => {
                return Err(PyenvError::Io(format!(
                    "pyenv: failed to install/update Xcode Command Line Tools: {error}"
                )));
            }
        }
    }

    if has_macos_openssl_fix {
        let brew = std::process::Command::new("brew")
            .args([
                "install",
                "openssl@3",
                "pkg-config",
                "readline",
                "sqlite3",
                "xz",
                "zlib",
                "bzip2",
            ])
            .output();
        match brew {
            Ok(out) if out.status.success() => {
                applied.push(
                    "Installed Homebrew OpenSSL and related CPython build libraries.".to_string(),
                );
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                return Err(PyenvError::Io(format!(
                    "pyenv: failed to install Homebrew OpenSSL deps (exit {}): {stderr}",
                    out.status
                )));
            }
            Err(error) => {
                return Err(PyenvError::Io(format!(
                    "pyenv: Homebrew is required for automated OpenSSL setup on macOS: {error}"
                )));
            }
        }
    }

    // Execute Termux automated repair if planned
    if has_termux_fix {
        use crate::preflight::termux_required_pkg_packages;

        let mut args = vec!["install".to_string()];
        args.extend(
            termux_required_pkg_packages()
                .iter()
                .map(|package| (*package).to_string()),
        );
        args.push("-y".to_string());
        let output = std::process::Command::new("pkg").args(&args).output();
        match output {
            Ok(out) if out.status.success() => {
                applied.push("Successfully auto-installed missing Termux compilation tools and system libraries via pkg.".to_string());
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                return Err(PyenvError::Io(format!(
                    "pyenv: failed to auto-install Termux dependencies (exit code {}): {}",
                    out.status, stderr
                )));
            }
            Err(e) => {
                return Err(PyenvError::Io(format!(
                    "pyenv: failed to execute pkg install command: {}",
                    e
                )));
            }
        }
    }

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

pub fn install_powershell_7() -> Result<String, PyenvError> {
    if cfg!(not(windows)) {
        return Err(PyenvError::Io(
            "pyenv: winget PowerShell 7 install is only available on Windows".to_string(),
        ));
    }

    let output = std::process::Command::new("winget")
        .args([
            "install",
            "--id",
            "Microsoft.PowerShell",
            "-e",
            "--accept-package-agreements",
            "--accept-source-agreements",
            "--disable-interactivity",
        ])
        .output()
        .map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: winget is required to install PowerShell 7: {error}"
            ))
        })?;
    if output.status.success() {
        Ok("Installed PowerShell 7 (pwsh) via winget (Microsoft.PowerShell). Restart the terminal to refresh PATH.".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(PyenvError::Io(format!(
            "pyenv: winget failed to install PowerShell 7 (exit {}): {stderr}",
            output.status
        )))
    }
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
        "pkg install clang make pkg-config libffi openssl readline ncurses sqlite zlib bzip2 xz -y"
            .to_string()
    } else if platform == "macos" {
        "pyenv doctor --fix  # installs/updates CLT when possible, then: brew install openssl@3 pkg-config readline sqlite3 xz zlib bzip2"
            .to_string()
    } else {
        "Install a POSIX shell, make, compiler toolchain, and development headers for OpenSSL/readline/sqlite/zlib".to_string()
    };

    vec![DoctorFix {
        key: format!("{platform}-source-build-deps-manual"),
        automated: false,
        description: fix_text_arg("doctor-fix-source-deps", "platform", platform),
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
            description: fix_text_arg("doctor-fix-missing-venv", "value", &value),
            command_hint: Some(format!(
                "Use `pyenv venv list`, `pyenv venv info {value}`, or update `.python-version` with `pyenv venv use <name>`"
            )),
        })
        .collect()
}
