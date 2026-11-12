// ./crates/pyenv-cli/tests/self_install.rs
//! End-to-end coverage for `pyenv self-install` / `self-uninstall` using the real binary.
//!
//! Each test runs in an isolated HOME so no real shell profile or install is ever touched.
//! The installer's source directory is a throwaway copy of the test binary, mirroring how a
//! release bundle is laid out.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

struct Sandbox {
    _temp: TempDir,
    home: PathBuf,
    bundle: PathBuf,
    root: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let temp = TempDir::new().expect("tempdir");
        let home = temp.path().join("home");
        let bundle = temp.path().join("bundle");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&bundle).unwrap();
        let exe = env!("CARGO_BIN_EXE_pyenv");
        fs::copy(exe, bundle.join("pyenv")).expect("copy pyenv into the fake bundle");
        let root = temp.path().join("root");
        Self {
            _temp: temp,
            home,
            bundle,
            root,
        }
    }

    fn run(&self, program: &Path, args: &[&str]) -> Output {
        // Parallel tests fork while another test has the freshly copied binary open for writing,
        // which makes `exec` fail with ETXTBSY. The window is tiny, so retry briefly.
        let mut attempts = 0;
        loop {
            let result = Command::new(program)
                .args(args)
                .env("HOME", &self.home)
                .env("SHELL", "/bin/bash")
                .env("PYENV_ROOT", &self.root)
                .env_remove("PYENV_VERSION")
                .output();
            match result {
                Err(error)
                    if error.kind() == std::io::ErrorKind::ExecutableFileBusy && attempts < 40 =>
                {
                    attempts += 1;
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
                other => return other.expect("run pyenv"),
            }
        }
    }

    fn install(&self, extra: &[&str]) -> Output {
        let mut args = vec!["self-install", "--yes", "--install-root"];
        let root = self.root.to_string_lossy().into_owned();
        args.push(&root);
        args.extend_from_slice(extra);
        self.run(&self.bundle.join("pyenv"), &args)
    }

    fn installed(&self) -> PathBuf {
        self.root.join("bin").join("pyenv")
    }

    fn bashrc(&self) -> PathBuf {
        self.home.join(".bashrc")
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn installs_binaries_profile_block_and_log() {
    let sandbox = Sandbox::new();
    fs::write(sandbox.bashrc(), "export KEEP=1\n").unwrap();

    let output = sandbox.install(&[]);
    assert!(output.status.success(), "{}", text(&output));

    assert!(sandbox.installed().is_file());
    for dir in ["shims", "versions", "cache", "logs"] {
        assert!(sandbox.root.join(dir).is_dir(), "missing {dir}");
    }
    let profile = fs::read_to_string(sandbox.bashrc()).unwrap();
    assert!(profile.starts_with("export KEEP=1\n"), "{profile}");
    assert_eq!(profile.matches("# >>> pyenv-native init >>>").count(), 1);
    assert!(profile.contains("init - bash"));

    let logs: Vec<_> = fs::read_dir(sandbox.root.join("logs")).unwrap().collect();
    assert_eq!(logs.len(), 1, "exactly one install log");

    let version = sandbox.run(&sandbox.installed(), &["--version"]);
    assert!(version.status.success());
}

#[test]
fn reinstalling_is_idempotent_and_repairing_from_the_installed_copy_works() {
    let sandbox = Sandbox::new();
    assert!(sandbox.install(&[]).status.success());
    assert!(sandbox.install(&[]).status.success());
    let profile = fs::read_to_string(sandbox.bashrc()).unwrap();
    assert_eq!(profile.matches("# >>> pyenv-native init >>>").count(), 1);

    // Running from the installed location is a repair: nothing to copy, integration refreshed.
    let output = sandbox.run(
        &sandbox.installed(),
        &["self-install", "--yes", "--shell", "bash"],
    );
    assert!(output.status.success(), "{}", text(&output));
    assert!(
        text(&output).contains("already in place"),
        "{}",
        text(&output)
    );
}

#[test]
fn no_confirmation_flag_refuses_to_touch_the_disk() {
    let sandbox = Sandbox::new();
    let root = sandbox.root.to_string_lossy().into_owned();
    // stdin is not a terminal under the test harness, so the prompt must refuse, not hang.
    let output = sandbox.run(
        &sandbox.bundle.join("pyenv"),
        &["self-install", "--install-root", &root],
    );
    assert!(!output.status.success());
    assert!(text(&output).contains("--yes"), "{}", text(&output));
    assert!(
        !sandbox.root.exists(),
        "nothing may be created before confirmation"
    );
    assert!(!sandbox.bashrc().exists());
}

#[test]
fn update_shell_profile_false_leaves_profiles_alone() {
    let sandbox = Sandbox::new();
    let output = sandbox.install(&["--update-shell-profile", "no"]);
    assert!(output.status.success(), "{}", text(&output));
    assert!(sandbox.installed().is_file());
    assert!(!sandbox.bashrc().exists());
}

#[test]
fn keep_data_uninstall_removes_programs_and_integration_only() {
    let sandbox = Sandbox::new();
    assert!(sandbox.install(&[]).status.success());
    fs::create_dir_all(sandbox.root.join("versions").join("3.12.0")).unwrap();

    let output = sandbox.run(
        &sandbox.installed(),
        &["self-uninstall", "--yes", "--keep-data"],
    );
    assert!(output.status.success(), "{}", text(&output));
    assert!(!sandbox.root.join("bin").exists());
    assert!(!sandbox.root.join("shims").exists());
    assert!(sandbox.root.join("versions").join("3.12.0").is_dir());
    assert!(!sandbox.bashrc().exists(), "profile held only our block");
}

#[test]
fn full_uninstall_removes_the_root_and_restores_the_profile() {
    let sandbox = Sandbox::new();
    fs::write(sandbox.bashrc(), "export KEEP=1\nalias ll='ls -l'\n").unwrap();
    assert!(sandbox.install(&[]).status.success());

    let output = sandbox.run(&sandbox.installed(), &["self-uninstall", "--yes"]);
    assert!(output.status.success(), "{}", text(&output));
    assert!(!sandbox.root.exists());
    let profile = fs::read_to_string(sandbox.bashrc()).unwrap();
    assert_eq!(profile.trim_end(), "export KEEP=1\nalias ll='ls -l'");
}

#[test]
fn uninstall_refuses_roots_that_are_not_installs() {
    let sandbox = Sandbox::new();
    fs::create_dir_all(&sandbox.root).unwrap();
    fs::write(sandbox.root.join("important.txt"), "do not delete").unwrap();

    let output = sandbox.run(&sandbox.bundle.join("pyenv"), &["self-uninstall", "--yes"]);
    assert!(!output.status.success());
    assert!(text(&output).contains("refusing"), "{}", text(&output));
    assert!(sandbox.root.join("important.txt").is_file());

    // PYENV_ROOT pointing at the home directory must never be removable either.
    let home_run = Command::new(sandbox.bundle.join("pyenv"))
        .args(["self-uninstall", "--yes"])
        .env("HOME", &sandbox.home)
        .env("PYENV_ROOT", &sandbox.home)
        .output()
        .unwrap();
    assert!(!home_run.status.success());
    assert!(sandbox.home.is_dir());
}
