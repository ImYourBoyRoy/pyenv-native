// ./crates/pyenv-core/src/install/archive.rs
//! Package download, extraction, and install receipt helpers.

use std::ffi::OsStr;
use std::fs;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use tar::Archive;
use zip::ZipArchive;

use crate::error::PyenvError;
use crate::http::build_blocking_client;

use super::report::{io_error, pip_wrapper_names, sanitize_for_fs};
use super::types::{INSTALL_RECEIPT_FILE, InstallPlan, InstallReceipt};

pub(super) fn download_package(plan: &InstallPlan) -> Result<(), PyenvError> {
    if plan.cache_path.is_file() {
        return Ok(());
    }

    let parent = plan
        .cache_path
        .parent()
        .ok_or_else(|| PyenvError::Io("pyenv: invalid cache path".to_string()))?;
    fs::create_dir_all(parent).map_err(io_error)?;

    let extension = plan
        .cache_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("download");
    let partial_path = parent.join(format!(
        ".partial-{}.{}",
        sanitize_for_fs(&plan.package_name),
        extension
    ));
    if partial_path.exists() {
        let _ = fs::remove_file(&partial_path);
    }

    let client = build_blocking_client()
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to build HTTP client: {error}")))?;

    let response = client.get(&plan.download_url).send().map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to download {}: {error}",
            plan.download_url
        ))
    })?;

    let mut response = response.error_for_status().map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to download {}: {error}",
            plan.download_url
        ))
    })?;

    let mut file = fs::File::create(&partial_path).map_err(io_error)?;
    response.copy_to(&mut file).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to write {}: {error}",
            partial_path.display()
        ))
    })?;
    file.flush().map_err(io_error)?;
    fs::rename(&partial_path, &plan.cache_path).map_err(io_error)
}

pub(super) fn extract_archive(plan: &InstallPlan, destination: &Path) -> Result<(), PyenvError> {
    if is_zip_extension(plan.cache_path.as_path()) && plan.provider != "windows-cpython-nuget" {
        extract_root_archive(&plan.cache_path, destination)
    } else if is_tar_archive_extension(plan.cache_path.as_path()) {
        extract_tar_root_archive(&plan.cache_path, destination)
    } else {
        extract_tools_archive(&plan.cache_path, destination)
    }
}

pub(super) fn extract_tools_archive(
    archive_path: &Path,
    destination: &Path,
) -> Result<(), PyenvError> {
    prepare_clean_directory(destination)?;

    let file = fs::File::open(archive_path).map_err(io_error)?;
    let mut archive = ZipArchive::new(file).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to open {}: {error}",
            archive_path.display()
        ))
    })?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| {
            PyenvError::Io(format!("pyenv: failed to read archive entry: {error}"))
        })?;
        let Some(path) = entry.enclosed_name().map(|value| value.to_path_buf()) else {
            continue;
        };
        let Ok(relative) = path.strip_prefix("tools") else {
            continue;
        };
        if relative.as_os_str().is_empty() {
            continue;
        }

        let output_path = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&output_path).map_err(io_error)?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }

        let mut output = fs::File::create(&output_path).map_err(io_error)?;
        std::io::copy(&mut entry, &mut output).map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to extract {}: {error}",
                output_path.display()
            ))
        })?;
        output.flush().map_err(io_error)?;
    }

    Ok(())
}

pub(super) fn extract_root_archive(
    archive_path: &Path,
    destination: &Path,
) -> Result<(), PyenvError> {
    prepare_clean_directory(destination)?;

    let file = fs::File::open(archive_path).map_err(io_error)?;
    let mut archive = ZipArchive::new(file).map_err(|error| {
        PyenvError::Io(format!(
            "pyenv: failed to open {}: {error}",
            archive_path.display()
        ))
    })?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| {
            PyenvError::Io(format!("pyenv: failed to read archive entry: {error}"))
        })?;
        let Some(path) = entry.enclosed_name().map(|value| value.to_path_buf()) else {
            continue;
        };

        let mut components = path.components();
        let _ = components.next();
        let relative = components.collect::<PathBuf>();
        if relative.as_os_str().is_empty() {
            continue;
        }

        let output_path = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&output_path).map_err(io_error)?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }

        let mut output = fs::File::create(&output_path).map_err(io_error)?;
        std::io::copy(&mut entry, &mut output).map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to extract {}: {error}",
                output_path.display()
            ))
        })?;
        output.flush().map_err(io_error)?;
    }

    Ok(())
}

pub(super) fn extract_tar_root_archive(
    archive_path: &Path,
    destination: &Path,
) -> Result<(), PyenvError> {
    prepare_clean_directory(destination)?;

    let file = fs::File::open(archive_path).map_err(io_error)?;
    let file_name = archive_path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();

    if file_name.ends_with(".tar.bz2")
        || file_name.ends_with(".tbz2")
        || file_name.ends_with(".tbz")
    {
        let decoder = BzDecoder::new(BufReader::new(file));
        let mut archive = Archive::new(decoder);
        archive.unpack(destination).map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to extract {}: {error}",
                archive_path.display()
            ))
        })?;
    } else if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
        let decoder = GzDecoder::new(BufReader::new(file));
        let mut archive = Archive::new(decoder);
        archive.unpack(destination).map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to extract {}: {error}",
                archive_path.display()
            ))
        })?;
    } else if file_name.ends_with(".tar") {
        let mut archive = Archive::new(BufReader::new(file));
        archive.unpack(destination).map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to extract {}: {error}",
                archive_path.display()
            ))
        })?;
    } else {
        return Err(PyenvError::Io(format!(
            "pyenv: unsupported archive format: {}",
            archive_path.display()
        )));
    }

    flatten_single_top_level_directory(destination)
}

fn prepare_clean_directory(destination: &Path) -> Result<(), PyenvError> {
    if destination.exists() {
        fs::remove_dir_all(destination).map_err(io_error)?;
    }
    fs::create_dir_all(destination).map_err(io_error)
}

fn flatten_single_top_level_directory(destination: &Path) -> Result<(), PyenvError> {
    let mut entries = fs::read_dir(destination)
        .map_err(io_error)?
        .filter_map(|entry| entry.ok())
        .collect::<Vec<_>>();
    if entries.len() != 1 {
        return Ok(());
    }

    let root = entries.pop().expect("single entry").path();
    if !root.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(&root).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        move_path(&source_path, &destination_path)?;
    }

    fs::remove_dir_all(&root).map_err(io_error)
}

fn move_path(source: &Path, destination: &Path) -> Result<(), PyenvError> {
    match fs::rename(source, destination) {
        Ok(_) => Ok(()),
        Err(_) => {
            if source.is_dir() {
                copy_dir_recursive(source, destination)?;
                fs::remove_dir_all(source).map_err(io_error)
            } else {
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent).map_err(io_error)?;
                }
                fs::copy(source, destination).map_err(io_error)?;
                fs::remove_file(source).map_err(io_error)
            }
        }
    }
}

fn is_zip_extension(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|value| {
            value.eq_ignore_ascii_case("zip") || value.eq_ignore_ascii_case("nupkg")
        })
}

fn is_tar_archive_extension(path: &Path) -> bool {
    let lower = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    lower.ends_with(".tar")
        || lower.ends_with(".tar.gz")
        || lower.ends_with(".tgz")
        || lower.ends_with(".tar.bz2")
        || lower.ends_with(".tbz")
        || lower.ends_with(".tbz2")
}

pub(super) fn move_directory(source: &Path, destination: &Path) -> Result<(), PyenvError> {
    match fs::rename(source, destination) {
        Ok(_) => Ok(()),
        Err(_) => {
            copy_dir_recursive(source, destination)?;
            fs::remove_dir_all(source).map_err(io_error)
        }
    }
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<(), PyenvError> {
    fs::create_dir_all(destination).map_err(io_error)?;
    for entry in fs::read_dir(source).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type().map_err(io_error)?.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent).map_err(io_error)?;
            }
            fs::copy(&source_path, &destination_path).map_err(io_error)?;
        }
    }
    Ok(())
}

pub(super) fn validate_python(python_executable: &Path) -> Result<(), PyenvError> {
    run_python(python_executable, &["-V"])
}

pub(super) fn run_python(python_executable: &Path, args: &[&str]) -> Result<(), PyenvError> {
    let output = Command::new(python_executable)
        .args(args)
        .output()
        .map_err(|error| {
            PyenvError::Io(format!(
                "pyenv: failed to run {}: {error}",
                python_executable.display()
            ))
        })?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("exit status {}", output.status)
        };
        Err(PyenvError::Io(format!(
            "pyenv: command `{}` failed: {detail}",
            render_command(python_executable, args)
        )))
    }
}

fn render_command(executable: &Path, args: &[&str]) -> String {
    let mut parts = vec![executable.display().to_string()];
    parts.extend(args.iter().map(|arg| arg.to_string()));
    parts.join(" ")
}

pub(super) fn write_install_receipt(plan: &InstallPlan) -> Result<PathBuf, PyenvError> {
    let receipt = InstallReceipt {
        requested_version: plan.requested_version.clone(),
        resolved_version: plan.resolved_version.clone(),
        provider: plan.provider.clone(),
        family: plan.family.clone(),
        architecture: plan.architecture.clone(),
        runtime_version: plan.runtime_version.clone(),
        package_name: plan.package_name.clone(),
        package_version: plan.package_version.clone(),
        download_url: plan.download_url.clone(),
        cache_path: plan.cache_path.clone(),
        python_executable: plan.python_executable.clone(),
        bootstrap_pip: plan.bootstrap_pip,
        base_venv_path: plan.base_venv_path.clone(),
        installed_at_epoch_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let receipt_path = plan.install_dir.join(INSTALL_RECEIPT_FILE);
    let contents = serde_json::to_string_pretty(&receipt)
        .map_err(|error| PyenvError::Io(format!("pyenv: failed to serialize receipt: {error}")))?;
    fs::write(&receipt_path, contents).map_err(io_error)?;
    Ok(receipt_path)
}

pub(super) fn ensure_pip_wrappers(plan: &InstallPlan) -> Result<(), PyenvError> {
    let scripts_dir = plan.install_dir.join("Scripts");
    fs::create_dir_all(&scripts_dir).map_err(io_error)?;

    let wrappers = pip_wrapper_names(&plan.runtime_version);
    let wrapper_body = "@echo off\r\n\"%~dp0..\\python.exe\" -m pip %*\r\n";

    for wrapper_name in wrappers {
        let wrapper_path = scripts_dir.join(format!("{wrapper_name}.cmd"));
        if !wrapper_path.exists() {
            fs::write(wrapper_path, wrapper_body).map_err(io_error)?;
        }
    }

    Ok(())
}
