// ./crates/pyenv-core/src/doctor/checks.rs
//! Doctor check collection for PATH health, selection health, and platform prerequisites.

use std::env;
use std::path::{Path, PathBuf};

use crate::context::{AppContext, is_pyenv_win_root};
use crate::executable::find_system_python_command;
use crate::install::resolve_python_build_path;
use crate::runtime::search_path_entries;
use crate::version::{SelectedVersions, resolve_selected_versions};

use super::helpers::{path_contains, path_ext_for_platform, paths_equal};
use super::types::{DoctorCheck, DoctorStatus};

pub fn collect_checks(ctx: &AppContext) -> Vec<DoctorCheck> {
    collect_checks_for_platform(ctx, env::consts::OS)
}

pub(super) fn collect_checks_for_platform(ctx: &AppContext, platform: &str) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();
    let exe_dir = ctx.bin_dir();
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
        checks.extend(termux_compile_environment_checks(ctx));
        if platform == "macos" {
            checks.extend(macos_toolchain_checks());
        }
        if platform == "android" || super::helpers::is_termux_environment() {
            checks.extend(android_termux_readiness_checks());
        }
    }

    checks.extend(selected_env_checks(&selected));
    checks.push(shim_launcher_integrity_check(ctx));
    checks.push(functional_shim_check(ctx, &selected));

    checks
}

fn macos_toolchain_checks() -> Vec<DoctorCheck> {
    use crate::preflight::inspect_macos_toolchain;

    let state = inspect_macos_toolchain();
    vec![
        DoctorCheck {
            name: "macos-xcode-clt".to_string(),
            status: if state.clt_ok {
                DoctorStatus::Ok
            } else {
                DoctorStatus::Warn
            },
            detail: state.clt_detail,
        },
        DoctorCheck {
            name: "macos-openssl".to_string(),
            status: if state.openssl_prefix.is_some() {
                DoctorStatus::Ok
            } else {
                DoctorStatus::Warn
            },
            detail: state.openssl_detail,
        },
    ]
}

fn android_termux_readiness_checks() -> Vec<DoctorCheck> {
    use crate::preflight::inspect_android_toolchain;

    let state = inspect_android_toolchain();
    vec![
        DoctorCheck {
            name: "android-termux-prefix".to_string(),
            status: if state.prefix.is_some() {
                DoctorStatus::Ok
            } else {
                DoctorStatus::Warn
            },
            detail: state
                .prefix
                .as_ref()
                .map(|path| format!("Termux PREFIX at {}", path.display()))
                .unwrap_or_else(|| state.detail.clone()),
        },
        DoctorCheck {
            name: "android-source-build-readiness".to_string(),
            status: if state.ready {
                DoctorStatus::Ok
            } else {
                DoctorStatus::Warn
            },
            detail: state.detail,
        },
    ]
}

fn termux_compile_environment_checks(ctx: &AppContext) -> Vec<DoctorCheck> {
    use super::helpers::is_termux_environment;
    if !is_termux_environment() {
        return Vec::new();
    }

    let mut checks = Vec::new();
    let termux_usr = Path::new("/data/data/com.termux/files/usr");

    // 1. Check for compile-time CLI tools
    let tools = [
        ("clang", vec!["clang", "gcc", "cc"]),
        ("make", vec!["make"]),
        ("pkg-config", vec!["pkg-config"]),
    ];

    for (name, cmds) in tools {
        let status = command_presence_check(
            ctx,
            &format!("termux-tool-{}", name),
            &cmds,
            &format!("Termux tool {} is missing", name),
            "linux",
        );
        checks.push(status);
    }

    // 2. Check for system libraries headers or shared objects
    let libraries = [
        (
            "libffi",
            vec!["include/ffi.h", "include/ffi/ffi.h", "lib/libffi.so"],
        ),
        ("openssl", vec!["include/openssl/ssl.h", "lib/libssl.so"]),
        (
            "readline",
            vec!["include/readline/readline.h", "lib/libreadline.so"],
        ),
        (
            "ncurses",
            vec![
                "include/ncurses.h",
                "include/ncursesw/ncurses.h",
                "lib/libncurses.so",
            ],
        ),
        ("sqlite", vec!["include/sqlite3.h", "lib/libsqlite3.so"]),
        ("zlib", vec!["include/zlib.h", "lib/libz.so"]),
        ("bzip2", vec!["include/bzlib.h", "lib/libbz2.so"]),
        ("xz", vec!["include/lzma.h", "lib/liblzma.so"]),
    ];

    for (lib, paths) in libraries {
        let found = paths.iter().any(|rel| termux_usr.join(rel).exists());
        checks.push(DoctorCheck {
            name: format!("termux-lib-{}", lib),
            status: if found {
                DoctorStatus::Ok
            } else {
                DoctorStatus::Warn
            },
            detail: if found {
                format!("Termux library {} is installed", lib)
            } else {
                format!(
                    "Termux library package {} is missing; please install it",
                    lib
                )
            },
        });
    }

    checks
}

fn selected_env_checks(selected: &SelectedVersions) -> Vec<DoctorCheck> {
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

fn shim_launcher_integrity_check(ctx: &AppContext) -> DoctorCheck {
    let cli = ctx.cli_exe_path();
    let shims_dir = ctx.shims_dir();

    if !cli.is_file() {
        return DoctorCheck {
            name: "shim-launcher-integrity".to_string(),
            status: DoctorStatus::Warn,
            detail: format!(
                "pyenv CLI launcher not found at {}; reinstall pyenv-native or run the installer",
                cli.display()
            ),
        };
    }

    #[cfg(windows)]
    {
        let companion = ctx.root.join("bin").join("pyenv-gui.exe");
        let shim = shims_dir.join("python.exe");
        if shim.is_file()
            && companion.is_file()
            && files_are_identical(&shim, &companion).unwrap_or(false)
        {
            return DoctorCheck {
                name: "shim-launcher-integrity".to_string(),
                status: DoctorStatus::Warn,
                detail: "python shim is a copy of pyenv-gui.exe and will open the GUI instead of running Python; run `pyenv rehash` to repair".to_string(),
            };
        }

        if shim.is_file() && !files_are_identical(&shim, &cli).unwrap_or(false) {
            return DoctorCheck {
                name: "shim-launcher-integrity".to_string(),
                status: DoctorStatus::Warn,
                detail: format!(
                    "python shim at {} does not match the pyenv CLI launcher at {}; run `pyenv rehash` to refresh shims",
                    shim.display(),
                    cli.display()
                ),
            };
        }
    }

    #[cfg(not(windows))]
    {
        let shim = shims_dir.join("python");
        if let Ok(contents) = std::fs::read_to_string(&shim)
            && contents.contains("pyenv-gui")
        {
            return DoctorCheck {
                name: "shim-launcher-integrity".to_string(),
                status: DoctorStatus::Warn,
                detail: "python shim references pyenv-gui and will open the GUI instead of running Python; run `pyenv rehash` to repair".to_string(),
            };
        }

        if shim.is_file() && !contents_embed_cli_launcher(&shim, &cli).unwrap_or(true) {
            return DoctorCheck {
                name: "shim-launcher-integrity".to_string(),
                status: DoctorStatus::Warn,
                detail: format!(
                    "python shim at {} does not reference the pyenv CLI launcher at {}; run `pyenv rehash` to refresh shims",
                    shim.display(),
                    cli.display()
                ),
            };
        }
    }

    DoctorCheck {
        name: "shim-launcher-integrity".to_string(),
        status: DoctorStatus::Ok,
        detail: format!("shim launchers reference {}", cli.display()),
    }
}

#[cfg(windows)]
fn files_are_identical(lhs: &Path, rhs: &Path) -> Result<bool, std::io::Error> {
    let lhs_meta = lhs.metadata()?;
    let rhs_meta = rhs.metadata()?;
    if lhs_meta.len() != rhs_meta.len() {
        return Ok(false);
    }
    Ok(std::fs::read(lhs)? == std::fs::read(rhs)?)
}

#[cfg(not(windows))]
fn contents_embed_cli_launcher(shim: &Path, cli: &Path) -> Result<bool, std::io::Error> {
    let contents = std::fs::read_to_string(shim)?;
    Ok(contents.contains(&cli.display().to_string()))
}

fn functional_shim_check(ctx: &AppContext, selected: &SelectedVersions) -> DoctorCheck {
    if selected.versions.is_empty() {
        return DoctorCheck {
            name: "functional-shim-check".to_string(),
            status: DoctorStatus::Info,
            detail: "skipped functional test; no python version selected".to_string(),
        };
    }

    if !selected.missing.is_empty() {
        return DoctorCheck {
            name: "functional-shim-check".to_string(),
            status: DoctorStatus::Info,
            detail: "skipped functional test; selected versions are missing".to_string(),
        };
    }

    let shims_dir = ctx.shims_dir();
    let python_shim = if cfg!(windows) {
        shims_dir.join("python.bat")
    } else {
        shims_dir.join("python")
    };

    if !python_shim.exists() {
        return DoctorCheck {
            name: "functional-shim-check".to_string(),
            status: DoctorStatus::Warn,
            detail: "python shim not found; run `pyenv rehash` to generate it".to_string(),
        };
    }

    use crate::process::PyenvCommandExt;
    let output = std::process::Command::new(&python_shim)
        .headless()
        .arg("--version")
        .env("PYENV_ROOT", &ctx.root)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let mut version_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if version_str.is_empty() {
                version_str = String::from_utf8_lossy(&out.stderr).trim().to_string();
            }
            let version_str = version_str.lines().next().unwrap_or("unknown").to_string();
            DoctorCheck {
                name: "functional-shim-check".to_string(),
                status: DoctorStatus::Ok,
                detail: format!("shim functional; successfully invoked {version_str}"),
            }
        }
        Ok(out) => {
            let error = String::from_utf8_lossy(&out.stderr).trim().to_string();
            DoctorCheck {
                name: "functional-shim-check".to_string(),
                status: DoctorStatus::Warn,
                detail: format!(
                    "shim invocation failed (exit status {}): {}",
                    out.status, error
                ),
            }
        }
        Err(e) => DoctorCheck {
            name: "functional-shim-check".to_string(),
            status: DoctorStatus::Warn,
            detail: format!(
                "failed to launch python shim at {}: {}",
                python_shim.display(),
                e
            ),
        },
    }
}

fn pyenv_win_conflict_checks(ctx: &AppContext) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();

    if let Ok(env_root) = env::var("PYENV_ROOT")
        && is_pyenv_win_root(Path::new(&env_root))
    {
        checks.push(DoctorCheck {
            name: "pyenv-win-root-conflict".to_string(),
            status: DoctorStatus::Warn,
            detail: format!(
                "PYENV_ROOT is set to `{}` which looks like a pyenv-win path; pyenv-native overrides this at runtime, but removing the env var is recommended: remove PYENV_ROOT from your User environment variables",
                env_root
            ),
        });
    }

    let exe_dir = ctx.bin_dir();
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
                        "pyenv-win PATH entries appear before pyenv-native in PATH; this can cause pyenv-win to intercept commands. Remove pyenv-win entries from your User PATH: {}",
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
            "system python resolves to WindowsApps alias at {}; this 'trap' can intercept commands and should be disabled in 'Settings > Apps > App Execution Aliases'",
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
