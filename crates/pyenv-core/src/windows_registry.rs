// ./crates/pyenv-core/src/windows_registry.rs
//! Optional PEP-514 registration for managed Windows interpreters.
//!
//! Purpose: when `windows.registry_mode=pep514`, expose pyenv-native runtimes
//! under HKCU so IDEs can discover them without colliding with python.org's
//! `PythonCore` company keys.
//! How to use: call `apply_pep514_registration` after a successful install and
//! `remove_pep514_registration` during uninstall.
//! Inputs: install plan / version name and the configured registry mode.
//! Outputs: Ok when applied, skipped, or not applicable (non-Windows).
//! Notes: company is always `PyenvNative`. Writes are HKCU-only.

use crate::config::RegistryMode;
use crate::context::AppContext;
use crate::error::PyenvError;
use crate::install::InstallPlan;

pub const PEP514_COMPANY: &str = "PyenvNative";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pep514Spec {
    pub company: String,
    pub tag: String,
    pub display_name: String,
    pub support_url: String,
    pub version: String,
    pub sys_version: String,
    pub sys_architecture: String,
    pub install_dir: String,
    pub executable: String,
    pub windowed_executable: String,
}

impl Pep514Spec {
    pub fn from_plan(plan: &InstallPlan) -> Self {
        from_install_paths(
            &plan.resolved_version,
            &plan.architecture,
            &plan.install_dir,
            &plan.python_executable,
        )
    }

    pub fn key_path(&self) -> String {
        format!(r"Software\Python\{}\{}", self.company, self.tag)
    }
}

pub fn from_install_paths(
    resolved_version: &str,
    architecture: &str,
    install_dir: &std::path::Path,
    python_executable: &std::path::Path,
) -> Pep514Spec {
    let tag = resolved_version.trim().to_string();
    let sys_version = sys_version_from_tag(&tag);
    Pep514Spec {
        company: PEP514_COMPANY.to_string(),
        tag: tag.clone(),
        display_name: format!("Python {tag} (pyenv-native)"),
        support_url: "https://github.com/imyourboyroy/pyenv-native".to_string(),
        version: tag,
        sys_version,
        sys_architecture: pep514_sys_architecture(architecture),
        install_dir: install_dir.display().to_string(),
        executable: python_executable.display().to_string(),
        windowed_executable: windowed_executable_path(python_executable),
    }
}

fn windowed_executable_path(python_executable: &std::path::Path) -> String {
    let displayed = python_executable.display().to_string();
    let lowered = displayed.to_ascii_lowercase();
    if lowered.ends_with("python.exe") {
        format!(
            "{}pythonw.exe",
            &displayed[..displayed.len() - "python.exe".len()]
        )
    } else if lowered.ends_with("/python") || lowered.ends_with("\\python") {
        format!("{}pythonw", &displayed[..displayed.len() - "python".len()])
    } else {
        displayed
    }
}

fn sys_version_from_tag(tag: &str) -> String {
    let core = tag.trim_end_matches('t');
    let mut parts = core.split('.');
    match (parts.next(), parts.next()) {
        (Some(major), Some(minor)) if !major.is_empty() && !minor.is_empty() => {
            format!("{major}.{minor}")
        }
        _ => core.to_string(),
    }
}

fn pep514_sys_architecture(architecture: &str) -> String {
    match architecture.trim().to_ascii_lowercase().as_str() {
        "x86" | "win32" | "i386" | "i686" => "32bit".to_string(),
        _ => "64bit".to_string(),
    }
}

pub fn apply_pep514_registration(
    ctx: &AppContext,
    plan: &InstallPlan,
) -> Result<String, PyenvError> {
    if ctx.config.windows.registry_mode != RegistryMode::Pep514 {
        return Ok("PEP-514 registration skipped (windows.registry_mode=disabled)".to_string());
    }
    #[cfg(not(windows))]
    {
        let _ = plan;
        Ok("PEP-514 registration skipped (Windows-only)".to_string())
    }
    #[cfg(windows)]
    {
        let spec = Pep514Spec::from_plan(plan);
        write_pep514(&spec)?;
        Ok(format!(
            "registered PEP-514 interpreter at HKCU\\{}",
            spec.key_path()
        ))
    }
}

pub fn remove_pep514_registration(version: &str) -> Result<(), PyenvError> {
    let tag = version.trim();
    if tag.is_empty() {
        return Ok(());
    }
    delete_pep514_key(&format!(r"Software\Python\{PEP514_COMPANY}\{tag}"))
}

#[cfg(windows)]
fn write_pep514(spec: &Pep514Spec) -> Result<(), PyenvError> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey_with_flags(spec.key_path(), KEY_WRITE)
        .map_err(registry_io)?;
    key.set_value("DisplayName", &spec.display_name)
        .map_err(registry_io)?;
    key.set_value("SupportUrl", &spec.support_url)
        .map_err(registry_io)?;
    key.set_value("Version", &spec.version)
        .map_err(registry_io)?;
    key.set_value("SysVersion", &spec.sys_version)
        .map_err(registry_io)?;
    key.set_value("SysArchitecture", &spec.sys_architecture)
        .map_err(registry_io)?;
    let (install, _) = key
        .create_subkey_with_flags("InstallPath", KEY_WRITE)
        .map_err(registry_io)?;
    install
        .set_value("", &spec.install_dir)
        .map_err(registry_io)?;
    install
        .set_value("ExecutablePath", &spec.executable)
        .map_err(registry_io)?;
    install
        .set_value("WindowedExecutablePath", &spec.windowed_executable)
        .map_err(registry_io)?;
    Ok(())
}

#[cfg(windows)]
fn delete_pep514_key(key_path: &str) -> Result<(), PyenvError> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    match hkcu.delete_subkey_all(key_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(registry_io(error)),
    }
}

#[cfg(not(windows))]
fn delete_pep514_key(_key_path: &str) -> Result<(), PyenvError> {
    Ok(())
}

#[cfg(windows)]
fn registry_io(error: std::io::Error) -> PyenvError {
    PyenvError::Io(format!("pyenv: PEP-514 registry update failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{PEP514_COMPANY, from_install_paths};
    use std::path::Path;

    #[test]
    fn pep514_spec_uses_dedicated_company_and_full_tag() {
        let spec = from_install_paths(
            "3.14.7t",
            "x64",
            Path::new(r"C:\Users\me\.pyenv\versions\3.14.7t"),
            Path::new(r"C:\Users\me\.pyenv\versions\3.14.7t\python.exe"),
        );
        assert_eq!(spec.company, PEP514_COMPANY);
        assert_eq!(spec.tag, "3.14.7t");
        assert_eq!(spec.sys_version, "3.14");
        assert_eq!(spec.sys_architecture, "64bit");
        assert_eq!(spec.key_path(), r"Software\Python\PyenvNative\3.14.7t");
        assert!(spec.windowed_executable.ends_with("pythonw.exe"));
    }

    #[test]
    fn pep514_x86_is_32bit() {
        let spec = from_install_paths(
            "3.12.10",
            "x86",
            Path::new("/opt/pyenv/versions/3.12.10"),
            Path::new("/opt/pyenv/versions/3.12.10/bin/python"),
        );
        assert_eq!(spec.sys_architecture, "32bit");
    }
}
