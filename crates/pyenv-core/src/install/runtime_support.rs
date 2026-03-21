// ./crates/pyenv-core/src/install/runtime_support.rs
//! Shared subprocess, alias, and pip-bootstrap helpers for runtime installation flows.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(any(target_os = "android", test))]
use std::env;

use crate::context::AppContext;
use crate::error::PyenvError;

use super::archive::run_python;
use super::report::{format_command_output_suffix, io_error};
use super::types::InstallPlan;

pub(super) fn run_python_build_install(
    ctx: &AppContext,
    python_build: &Path,
    version: &str,
    prefix: &Path,
) -> Result<(), PyenvError> {
    let cache_dir = ctx.cache_dir().join("python-build");
    fs::create_dir_all(&cache_dir).map_err(io_error)?;

    let output = Command::new(python_build)
        .arg(version)
        .arg(prefix)
        .current_dir(&ctx.dir)
        .env("PYENV_ROOT", &ctx.root)
        .env("PYTHON_BUILD_CACHE_PATH", cache_dir)
        .output()
        .map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to execute {}: {error}",
                python_build.display()
            ))
        })?;

    if output.status.success() {
        return Ok(());
    }

    Err(PyenvError::Io(format!(
        "pyenv: python-build failed for `{version}` with exit code {}{}",
        output.status.code().unwrap_or(1),
        format_command_output_suffix(&output.stdout, &output.stderr)
    )))
}

pub(super) fn build_cpython_source_install(
    plan: &InstallPlan,
    source_dir: &Path,
    build_dir: &Path,
) -> Result<(), PyenvError> {
    let configure_script = source_dir.join("configure");
    if !configure_script.is_file() {
        return Err(PyenvError::Io(format!(
            "pyenv: extracted source tree is missing {}",
            configure_script.display()
        )));
    }

    let prefix_arg = format!("--prefix={}", plan.install_dir.display());
    let mut configure = Command::new("sh");
    configure
        .current_dir(build_dir)
        .arg(configure_script)
        .arg(prefix_arg)
        .arg("--with-ensurepip=install");
    if plan.free_threaded {
        configure.arg("--disable-gil");
    }
    apply_android_source_build_env(&mut configure);
    run_checked_process(
        configure,
        format!("configure source build for `{}`", plan.resolved_version),
    )?;

    let jobs = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1);
    let mut make = Command::new("make");
    make.current_dir(build_dir).arg(format!("-j{jobs}"));
    apply_android_source_build_env(&mut make);
    run_checked_process(make, format!("build `{}`", plan.resolved_version))?;

    let mut install = Command::new("make");
    install.current_dir(build_dir).arg("install");
    apply_android_source_build_env(&mut install);
    run_checked_process(
        install,
        format!(
            "install `{}` into {}",
            plan.resolved_version,
            plan.install_dir.display()
        ),
    )
}

fn run_checked_process(mut command: Command, description: String) -> Result<(), PyenvError> {
    let output = command
        .output()
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to {description}: {error}")))?;

    if output.status.success() {
        return Ok(());
    }

    Err(PyenvError::Io(format!(
        "pyenv: failed to {description} with exit code {}{}",
        output.status.code().unwrap_or(1),
        format_command_output_suffix(&output.stdout, &output.stderr)
    )))
}

fn apply_android_source_build_env(command: &mut Command) {
    #[cfg(target_os = "android")]
    {
        let prefix = env::var_os("PREFIX").map(PathBuf::from);
        for (key, value) in android_source_build_env(prefix.as_deref(), detect_android_api_level())
        {
            command.env(key, value);
        }
    }

    #[cfg(not(target_os = "android"))]
    {
        let _ = command;
    }
}

#[cfg(any(target_os = "android", test))]
fn android_source_build_env(
    prefix: Option<&Path>,
    api_level: Option<u32>,
) -> Vec<(String, String)> {
    let mut env_pairs = Vec::new();

    if let Some(prefix) = prefix {
        let include_dir = prefix.join("include");
        let lib_dir = prefix.join("lib");
        env_pairs.push((
            "CPPFLAGS".to_string(),
            append_shell_flag(
                env::var("CPPFLAGS").ok().as_deref(),
                &format!("-I{}", include_dir.display()),
            ),
        ));
        env_pairs.push((
            "LDFLAGS".to_string(),
            append_shell_flag(
                env::var("LDFLAGS").ok().as_deref(),
                &format!("-L{} -Wl,-rpath,{}", lib_dir.display(), lib_dir.display()),
            ),
        ));
        env_pairs.push(("LIBCRYPT_LIBS".to_string(), "-lcrypt".to_string()));
    }

    if let Some(api_level) = api_level {
        if api_level < 28 {
            env_pairs.push(("ac_cv_func_fexecve".to_string(), "no".to_string()));
            env_pairs.push(("ac_cv_func_getlogin_r".to_string(), "no".to_string()));
        }
        if api_level < 29 {
            env_pairs.push(("ac_cv_func_getloadavg".to_string(), "no".to_string()));
        }
        if api_level < 30 {
            env_pairs.push(("ac_cv_func_sem_clockwait".to_string(), "no".to_string()));
        }
        if api_level < 33 {
            env_pairs.push(("ac_cv_func_preadv2".to_string(), "no".to_string()));
            env_pairs.push(("ac_cv_func_pwritev2".to_string(), "no".to_string()));
        }
        if api_level < 34 {
            env_pairs.push(("ac_cv_func_close_range".to_string(), "no".to_string()));
            env_pairs.push(("ac_cv_func_copy_file_range".to_string(), "no".to_string()));
        }
    }

    env_pairs
}

#[cfg(any(target_os = "android", test))]
fn append_shell_flag(existing: Option<&str>, addition: &str) -> String {
    match existing.map(str::trim).filter(|value| !value.is_empty()) {
        Some(existing) => format!("{existing} {addition}"),
        None => addition.to_string(),
    }
}

#[cfg(any(target_os = "android", test))]
#[cfg_attr(test, allow(dead_code))]
fn detect_android_api_level() -> Option<u32> {
    for key in [
        "TERMUX_PKG_API_LEVEL",
        "ANDROID_API_LEVEL",
        "TERMUX_API_LEVEL",
    ] {
        if let Ok(value) = env::var(key)
            && let Ok(parsed) = value.trim().parse::<u32>()
        {
            return Some(parsed);
        }
    }

    let output = Command::new("getprop")
        .arg("ro.build.version.sdk")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

pub(super) fn ensure_unix_runtime_aliases(
    prefix: &Path,
    runtime_version: &str,
) -> Result<(), PyenvError> {
    let bin_dir = prefix.join("bin");
    if !bin_dir.is_dir() {
        return Ok(());
    }

    let parts = runtime_version.split('.').collect::<Vec<_>>();
    let major = parts.first().copied().unwrap_or("3");
    let major_minor = parts.iter().take(2).copied().collect::<Vec<_>>().join(".");

    let python_candidates = [
        bin_dir.join("python"),
        bin_dir.join("python3"),
        bin_dir.join(format!("python{major}")),
        bin_dir.join(format!("python{major_minor}")),
    ];
    if let Some(source) = first_existing_file(&python_candidates) {
        ensure_path_alias(&source, &bin_dir.join("python3"))?;
        ensure_path_alias(&source, &bin_dir.join("python"))?;
    }

    let pip_candidates = [
        bin_dir.join("pip"),
        bin_dir.join("pip3"),
        bin_dir.join(format!("pip{major}")),
        bin_dir.join(format!("pip{major_minor}")),
    ];
    if let Some(source) = first_existing_file(&pip_candidates) {
        ensure_path_alias(&source, &bin_dir.join("pip3"))?;
        ensure_path_alias(&source, &bin_dir.join("pip"))?;
    }

    Ok(())
}

fn first_existing_file(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|path| path.is_file()).cloned()
}

fn ensure_path_alias(source: &Path, destination: &Path) -> Result<(), PyenvError> {
    if source == destination || destination.exists() {
        return Ok(());
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let link_target = if source.parent() == destination.parent() {
            PathBuf::from(
                source
                    .file_name()
                    .ok_or_else(|| PyenvError::Io("pyenv: invalid alias source".to_string()))?,
            )
        } else {
            source.to_path_buf()
        };

        match symlink(&link_target, destination) {
            Ok(_) => Ok(()),
            Err(error) => {
                fs::copy(source, destination).map_err(|copy_error| {
                    PyenvError::Io(format!(
                        "pyenv: failed to create alias {} -> {}: {error}; copy fallback also failed: {copy_error}",
                        destination.display(),
                        source.display()
                    ))
                })?;
                Ok(())
            }
        }
    }

    #[cfg(not(unix))]
    {
        fs::copy(source, destination).map_err(io_error)?;
        Ok(())
    }
}

pub(super) fn ensure_pip_available(python_executable: &Path) -> Result<bool, PyenvError> {
    if run_python(python_executable, &["-m", "pip", "--version"]).is_ok() {
        return Ok(true);
    }

    run_python(python_executable, &["-m", "ensurepip", "--default-pip"])?;
    run_python(python_executable, &["-m", "pip", "--version"])?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::android_source_build_env;

    #[test]
    fn android_source_build_env_includes_termux_prefix_flags() {
        let env_pairs =
            android_source_build_env(Some(Path::new("/data/data/com.termux/files/usr")), Some(34));
        let cppflags = env_pairs
            .iter()
            .find(|(key, _)| key == "CPPFLAGS")
            .map(|(_, value)| value.replace('\\', "/"))
            .expect("CPPFLAGS");
        let ldflags = env_pairs
            .iter()
            .find(|(key, _)| key == "LDFLAGS")
            .map(|(_, value)| value.replace('\\', "/"))
            .expect("LDFLAGS");

        assert!(cppflags.contains("/data/data/com.termux/files/usr/include"));
        assert!(ldflags.contains("/data/data/com.termux/files/usr/lib"));
        assert!(
            env_pairs
                .iter()
                .any(|(key, value)| key == "LIBCRYPT_LIBS" && value == "-lcrypt")
        );
    }

    #[test]
    fn android_source_build_env_disables_api_gated_functions() {
        let env_pairs = android_source_build_env(None, Some(32));
        assert!(
            env_pairs
                .iter()
                .any(|(key, value)| key == "ac_cv_func_close_range" && value == "no")
        );
        assert!(
            env_pairs
                .iter()
                .any(|(key, value)| key == "ac_cv_func_copy_file_range" && value == "no")
        );
        assert!(
            env_pairs
                .iter()
                .any(|(key, value)| key == "ac_cv_func_preadv2" && value == "no")
        );
        assert!(
            env_pairs
                .iter()
                .any(|(key, value)| key == "ac_cv_func_pwritev2" && value == "no")
        );
    }
}
