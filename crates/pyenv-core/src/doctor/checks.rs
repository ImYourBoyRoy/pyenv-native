// ./crates/pyenv-core/src/doctor/checks.rs
//! Doctor check collection for PATH health, selection health, and platform prerequisites.

use std::env;
use std::path::{Path, PathBuf};

use crate::context::{AppContext, is_pyenv_win_root};
use crate::executable::find_system_python_command;
use crate::install::resolve_python_build_path;
use crate::runtime::search_path_entries;
use crate::version::{SelectedVersions, resolve_selected_versions};

use super::helpers::{
    find_windows_store_python_alias, path_contains, path_ext_for_platform, paths_equal,
    windows_store_alias_precedes_shims,
};
use super::types::{DoctorCheck, DoctorOptions, DoctorStatus};

fn i18n(id: &str) -> String {
    crate::i18n::lookup(id)
}

fn i18n_args(id: &str, pairs: &[(&str, String)]) -> String {
    let mut args = std::collections::HashMap::new();
    for (key, value) in pairs {
        args.insert((*key).to_string(), value.clone());
    }
    crate::i18n::lookup_with_args(id, &args)
}

pub fn collect_checks(ctx: &AppContext) -> Vec<DoctorCheck> {
    collect_checks_with_options(ctx, DoctorOptions::default())
}

pub fn collect_checks_with_options(ctx: &AppContext, options: DoctorOptions) -> Vec<DoctorCheck> {
    collect_checks_for_platform(ctx, env::consts::OS, options)
}

pub(super) fn collect_checks_for_platform(
    ctx: &AppContext,
    platform: &str,
    options: DoctorOptions,
) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();
    let exe_dir = ctx.bin_dir();
    let shims_dir = ctx.shims_dir();
    let versions_dir = ctx.versions_dir();
    let desktop_profiles_ok = options.desktop_session
        && options
            .profiles_configured
            .unwrap_or_else(super::helpers::user_shell_profiles_configured);

    checks.push(DoctorCheck {
        name: "root-directory".to_string(),
        status: if ctx.root.exists() {
            DoctorStatus::Ok
        } else {
            DoctorStatus::Warn
        },
        detail: path_detail(
            ctx.root.exists(),
            "doctor-root-ok",
            "doctor-root-missing",
            &ctx.root,
        ),
    });

    checks.push(path_membership_check(
        "pyenv-bin-on-path",
        path_contains(ctx.path_env.as_ref(), &exe_dir),
        desktop_profiles_ok,
        &exe_dir,
        "doctor-bin-on-path",
        "doctor-bin-on-path-desktop",
    ));

    checks.push(path_membership_check(
        "shims-on-path",
        path_contains(ctx.path_env.as_ref(), &shims_dir),
        desktop_profiles_ok,
        &shims_dir,
        "doctor-shims-on-path",
        "doctor-shims-on-path-desktop",
    ));

    checks.push(DoctorCheck {
        name: "versions-directory".to_string(),
        status: if versions_dir.is_dir() {
            DoctorStatus::Ok
        } else {
            DoctorStatus::Info
        },
        detail: {
            let mut args = std::collections::HashMap::new();
            args.insert("path".to_string(), versions_dir.display().to_string());
            crate::i18n::lookup_with_args("doctor-versions-dir", &args)
        },
    });

    let selected = resolve_selected_versions(ctx, false);
    let selected_detail = if selected.versions.is_empty() {
        i18n("doctor-no-selected-versions")
    } else {
        i18n_args(
            "doctor-selected-versions",
            &[
                ("versions", selected.versions.join(" ")),
                ("origin", selected.origin.to_string()),
            ],
        )
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
        checks.push(windows_powershell_7_check(ctx));
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
    checks.push(plugin_path_search_check(ctx));

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
                i18n_args("doctor-termux-lib-ok", &[("lib", lib.to_string())])
            } else {
                i18n_args("doctor-termux-lib-missing", &[("lib", lib.to_string())])
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
            detail: i18n_args("doctor-venv-missing", &[("value", value.clone())]),
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
            detail: i18n_args("doctor-cli-missing", &[("path", cli.display().to_string())]),
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
                detail: i18n("doctor-shim-gui-copy"),
            };
        }

        if shim.is_file() && !files_are_identical(&shim, &cli).unwrap_or(false) {
            return DoctorCheck {
                name: "shim-launcher-integrity".to_string(),
                status: DoctorStatus::Warn,
                detail: i18n_args(
                    "doctor-shim-mismatch",
                    &[
                        ("shim", shim.display().to_string()),
                        ("cli", cli.display().to_string()),
                    ],
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
                detail: i18n("doctor-shim-gui-ref"),
            };
        }

        if shim.is_file() && !contents_embed_cli_launcher(&shim, &cli).unwrap_or(true) {
            return DoctorCheck {
                name: "shim-launcher-integrity".to_string(),
                status: DoctorStatus::Warn,
                detail: i18n_args(
                    "doctor-shim-mismatch",
                    &[
                        ("shim", shim.display().to_string()),
                        ("cli", cli.display().to_string()),
                    ],
                ),
            };
        }
    }

    DoctorCheck {
        name: "shim-launcher-integrity".to_string(),
        status: DoctorStatus::Ok,
        detail: i18n_args(
            "doctor-shim-launchers-ok",
            &[("path", cli.display().to_string())],
        ),
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

fn python_shim_path(shims_dir: &Path) -> PathBuf {
    if cfg!(windows) {
        for name in ["python.exe", "python.cmd", "python.bat"] {
            let candidate = shims_dir.join(name);
            if candidate.exists() {
                return candidate;
            }
        }
        shims_dir.join("python.exe")
    } else {
        shims_dir.join("python")
    }
}

fn plugin_path_search_check(ctx: &AppContext) -> DoctorCheck {
    if ctx.config.plugins.search_path {
        DoctorCheck {
            name: "plugin-path-search".to_string(),
            status: DoctorStatus::Info,
            detail: crate::i18n::lookup("doctor-plugin-path-search-on"),
        }
    } else {
        DoctorCheck {
            name: "plugin-path-search".to_string(),
            status: DoctorStatus::Ok,
            detail: crate::i18n::lookup("doctor-plugin-path-search-off"),
        }
    }
}

fn path_detail(ok: bool, ok_id: &str, missing_id: &str, path: &Path) -> String {
    let mut args = std::collections::HashMap::new();
    args.insert("path".to_string(), path.display().to_string());
    crate::i18n::lookup_with_args(if ok { ok_id } else { missing_id }, &args)
}

fn path_membership_check(
    name: &str,
    on_path: bool,
    desktop_profiles_ok: bool,
    path: &Path,
    warn_id: &str,
    desktop_id: &str,
) -> DoctorCheck {
    let mut args = std::collections::HashMap::new();
    args.insert("path".to_string(), path.display().to_string());
    let (status, id) = if on_path {
        (DoctorStatus::Ok, warn_id)
    } else if desktop_profiles_ok {
        (DoctorStatus::Info, desktop_id)
    } else {
        (DoctorStatus::Warn, warn_id)
    };
    DoctorCheck {
        name: name.to_string(),
        status,
        detail: crate::i18n::lookup_with_args(id, &args),
    }
}

fn functional_shim_check(ctx: &AppContext, selected: &SelectedVersions) -> DoctorCheck {
    if selected.versions.is_empty() {
        return DoctorCheck {
            name: "functional-shim-check".to_string(),
            status: DoctorStatus::Info,
            detail: i18n("doctor-func-skip-no-version"),
        };
    }

    if !selected.missing.is_empty() {
        return DoctorCheck {
            name: "functional-shim-check".to_string(),
            status: DoctorStatus::Info,
            detail: i18n("doctor-func-skip-missing"),
        };
    }

    let shims_dir = ctx.shims_dir();
    let python_shim = python_shim_path(&shims_dir);

    if !python_shim.exists() {
        return DoctorCheck {
            name: "functional-shim-check".to_string(),
            status: DoctorStatus::Warn,
            detail: i18n("doctor-func-shim-missing"),
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
                detail: i18n_args("doctor-func-ok", &[("version", version_str)]),
            }
        }
        Ok(out) => {
            let error = String::from_utf8_lossy(&out.stderr).trim().to_string();
            DoctorCheck {
                name: "functional-shim-check".to_string(),
                status: DoctorStatus::Warn,
                detail: i18n_args(
                    "doctor-func-failed",
                    &[("status", out.status.to_string()), ("error", error)],
                ),
            }
        }
        Err(e) => DoctorCheck {
            name: "functional-shim-check".to_string(),
            status: DoctorStatus::Warn,
            detail: i18n_args(
                "doctor-func-launch-failed",
                &[
                    ("path", python_shim.display().to_string()),
                    ("error", e.to_string()),
                ],
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
            detail: i18n_args("doctor-pyenv-win-root", &[("path", env_root)]),
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
                    detail: i18n_args(
                        "doctor-pyenv-win-path",
                        &[("path", entries[pw_pos].display().to_string())],
                    ),
                });
            }
        }
    }

    checks
}

fn windows_store_alias_check(ctx: &AppContext) -> DoctorCheck {
    if let Some(alias) = find_windows_store_python_alias(ctx) {
        let intercepts = windows_store_alias_precedes_shims(ctx);
        let (status, detail) = if intercepts {
            (
                DoctorStatus::Warn,
                i18n_args(
                    "doctor-store-alias",
                    &[("path", alias.display().to_string())],
                ),
            )
        } else {
            (
                DoctorStatus::Info,
                i18n_args(
                    "doctor-store-alias-shadowed",
                    &[("path", alias.display().to_string())],
                ),
            )
        };
        return DoctorCheck {
            name: "system-python".to_string(),
            status,
            detail,
        };
    }

    let (status, detail) = match find_system_python_command(ctx) {
        Some(path) => (
            DoctorStatus::Info,
            i18n_args(
                "doctor-system-python",
                &[("path", path.display().to_string())],
            ),
        ),
        None => (DoctorStatus::Info, i18n("doctor-no-system-python")),
    };
    DoctorCheck {
        name: "system-python".to_string(),
        status,
        detail,
    }
}

fn windows_powershell_7_check(ctx: &AppContext) -> DoctorCheck {
    if crate::plugin::powershell_7_available(ctx) {
        DoctorCheck {
            name: "powershell-7".to_string(),
            status: DoctorStatus::Ok,
            detail: i18n("doctor-pwsh-ok"),
        }
    } else {
        DoctorCheck {
            name: "powershell-7".to_string(),
            status: DoctorStatus::Warn,
            detail: i18n("doctor-pwsh-missing"),
        }
    }
}

fn non_windows_python_build_check(ctx: &AppContext) -> DoctorCheck {
    match resolve_python_build_path(ctx) {
        Ok(path) => DoctorCheck {
            name: "python-build-backend".to_string(),
            status: DoctorStatus::Ok,
            detail: i18n_args(
                "doctor-python-build-ok",
                &[("path", path.display().to_string())],
            ),
        },
        Err(error) => DoctorCheck {
            name: "python-build-backend".to_string(),
            status: DoctorStatus::Info,
            detail: i18n_args(
                "doctor-python-build-optional",
                &[("error", error.to_string())],
            ),
        },
    }
}

fn non_windows_source_build_checks(ctx: &AppContext, platform: &str) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();

    checks.push(command_presence_check(
        ctx,
        "source-build-shell",
        &["sh", "bash"],
        "doctor-reason-shell",
        platform,
    ));
    checks.push(command_presence_check(
        ctx,
        "source-build-make",
        &["make", "gmake"],
        "doctor-reason-make",
        platform,
    ));
    checks.push(command_presence_check(
        ctx,
        "source-build-compiler",
        &["cc", "clang", "gcc"],
        "doctor-reason-compiler",
        platform,
    ));

    let pkg_config_status = command_presence_check(
        ctx,
        "source-build-pkg-config",
        &["pkg-config"],
        "doctor-reason-pkg-config",
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
            i18n_args(
                "doctor-source-may-fail",
                &[("platform", platform.to_string())],
            )
        } else {
            i18n_args("doctor-source-ok", &[("platform", platform.to_string())])
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
                detail: i18n_args(
                    "doctor-command-ok",
                    &[
                        ("command", command.to_string()),
                        ("path", path.display().to_string()),
                    ],
                ),
            };
        }
    }

    DoctorCheck {
        name: name.to_string(),
        status: DoctorStatus::Warn,
        detail: i18n_args(
            "doctor-command-missing",
            &[
                ("reason", i18n(missing_detail)),
                ("commands", commands.join(", ")),
            ],
        ),
    }
}

#[cfg(test)]
mod python_shim_path_tests {
    use super::python_shim_path;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn prefers_python_exe_over_bat_on_windows() {
        let temp = TempDir::new().expect("tempdir");
        fs::write(temp.path().join("python.bat"), b"bat").expect("bat");
        fs::write(temp.path().join("python.exe"), b"exe").expect("exe");
        let name = python_shim_path(temp.path())
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned();
        if cfg!(windows) {
            assert_eq!(name, "python.exe");
        } else {
            assert_eq!(name, "python");
        }
    }
}
