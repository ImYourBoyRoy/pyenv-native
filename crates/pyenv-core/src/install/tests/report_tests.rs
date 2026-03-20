// ./crates/pyenv-core/src/install/tests/report_tests.rs
//! Install-report rendering regression tests for live-progress-friendly summaries.

use std::path::PathBuf;

use super::super::report::{render_outcome_lines, render_outcome_summary_lines};
use super::super::{InstallOutcome, InstallPlan};

fn sample_outcome() -> InstallOutcome {
    InstallOutcome {
        plan: InstallPlan {
            requested_version: "3.13".to_string(),
            resolved_version: "3.13.12".to_string(),
            family: "CPython".to_string(),
            provider: "windows-cpython-nuget".to_string(),
            architecture: "x64".to_string(),
            runtime_version: "3.13.12".to_string(),
            free_threaded: false,
            package_name: "python".to_string(),
            package_version: "3.13.12".to_string(),
            download_url: "https://example.invalid/python.3.13.12.nupkg".to_string(),
            cache_path: PathBuf::from("cache/python.3.13.12.nupkg"),
            install_dir: PathBuf::from("versions/3.13.12"),
            python_executable: PathBuf::from("versions/3.13.12/python.exe"),
            bootstrap_pip: true,
            create_base_venv: false,
            base_venv_path: None,
        },
        receipt_path: PathBuf::from("versions/3.13.12/.pyenv-install.json"),
        pip_bootstrapped: true,
        base_venv_created: false,
        progress_steps: vec![
            "plan: resolved 3.13 -> 3.13.12 via windows-cpython-nuget [x64]".to_string(),
            "download: fetching package".to_string(),
        ],
    }
}

#[test]
fn render_outcome_lines_include_progress_section() {
    let lines = render_outcome_lines(&[sample_outcome()]);
    assert_eq!(lines.first().expect("progress header"), "Progress:");
    assert!(
        lines
            .iter()
            .any(|line| line.contains("download: fetching package"))
    );
}

#[test]
fn render_outcome_summary_lines_omit_progress_section() {
    let lines = render_outcome_summary_lines(&[sample_outcome()]);
    assert!(!lines.iter().any(|line| line == "Progress:"));
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("download: fetching package"))
    );
    assert!(lines.iter().any(|line| line == "Installed 3.13 -> 3.13.12"));
}
