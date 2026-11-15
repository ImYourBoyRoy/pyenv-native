// ./crates/enchi-cli/tests/self_install.rs
//! End-to-end coverage for `enchi self-install` / `self-uninstall` using the real binary.
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
        // Each sandbox holds several copies of the (large, in debug builds) binary, and /tmp is often
        // a small tmpfs, so use Cargo's on-disk scratch directory instead.
        let temp = tempfile::Builder::new()
            .prefix("selfinstall-")
            .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
            .expect("tempdir");
        let home = temp.path().join("home");
        let bundle = temp.path().join("bundle");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&bundle).unwrap();
        let exe = env!("CARGO_BIN_EXE_enchi");
        fs::copy(exe, bundle.join("enchi")).expect("copy enchi into the fake bundle");
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
                other => return other.expect("run enchi"),
            }
        }
    }

    fn install(&self, extra: &[&str]) -> Output {
        let mut args = vec!["self-install", "--yes", "--install-root"];
        let root = self.root.to_string_lossy().into_owned();
        args.push(&root);
        args.extend_from_slice(extra);
        self.run(&self.bundle.join("enchi"), &args)
    }

    fn installed(&self) -> PathBuf {
        self.root.join("bin").join("enchi")
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
    assert_eq!(profile.matches("# >>> enchi init >>>").count(), 1);
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
    assert_eq!(profile.matches("# >>> enchi init >>>").count(), 1);

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
        &sandbox.bundle.join("enchi"),
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

    let output = sandbox.run(&sandbox.bundle.join("enchi"), &["self-uninstall", "--yes"]);
    assert!(!output.status.success());
    assert!(text(&output).contains("refusing"), "{}", text(&output));
    assert!(sandbox.root.join("important.txt").is_file());

    // PYENV_ROOT pointing at the home directory must never be removable either.
    let home_run = Command::new(sandbox.bundle.join("enchi"))
        .args(["self-uninstall", "--yes"])
        .env("HOME", &sandbox.home)
        .env("PYENV_ROOT", &sandbox.home)
        .output()
        .unwrap();
    assert!(!home_run.status.success());
    assert!(sandbox.home.is_dir());
}

// ---- upgrading a pyenv-native install ---------------------------------------------------------

/// The arguments the pyenv-native updater passes to `<bundle>/pyenv self-install` (see
/// `installer_command` in the releases that shipped it). A bridge release must satisfy exactly this.
#[test]
fn install_dry_run_describes_the_install_and_writes_nothing() {
    let sandbox = Sandbox::new();
    fs::write(sandbox.bashrc(), "export KEEP=1\n").unwrap();

    let output = sandbox.install(&["--dry-run"]);
    let shown = text(&output);
    assert!(output.status.success(), "{shown}");
    assert!(shown.contains("Would install:"), "{shown}");
    assert!(
        shown.contains("Would add the Enchi init block to"),
        "{shown}"
    );
    assert!(shown.contains("Dry run: nothing was changed."), "{shown}");
    assert!(!sandbox.root.exists(), "a dry run must not create the root");
    assert_eq!(
        fs::read_to_string(sandbox.bashrc()).unwrap(),
        "export KEEP=1\n"
    );
}

#[test]
fn uninstall_dry_run_lists_removals_and_keeps_everything() {
    let sandbox = Sandbox::new();
    assert!(sandbox.install(&[]).status.success());
    let profile_before = fs::read_to_string(sandbox.bashrc()).unwrap();

    let output = sandbox.run(&sandbox.installed(), &["self-uninstall", "--dry-run"]);
    let shown = text(&output);
    assert!(output.status.success(), "{shown}");
    assert!(
        shown.contains("Would remove the Enchi block from"),
        "{shown}"
    );
    assert!(shown.contains("Would remove "), "{shown}");
    assert!(shown.contains("Dry run: nothing was changed."), "{shown}");
    assert!(sandbox.installed().is_file(), "binaries must still exist");
    assert_eq!(
        fs::read_to_string(sandbox.bashrc()).unwrap(),
        profile_before
    );

    let kept = sandbox.run(
        &sandbox.installed(),
        &["self-uninstall", "--dry-run", "--keep-data"],
    );
    assert!(text(&kept).contains("Would keep"), "{}", text(&kept));
}

#[test]
fn machine_mode_install_and_uninstall_never_prompt() {
    let sandbox = Sandbox::new();

    let refused = sandbox.run(
        &sandbox.bundle.join("enchi"),
        &[
            "self-install",
            "--json",
            "--install-root",
            &sandbox.root.to_string_lossy(),
        ],
    );
    assert_eq!(refused.status.code(), Some(1));
    assert!(
        text(&refused).contains("never prompts"),
        "{}",
        text(&refused)
    );
    assert!(!sandbox.root.exists());

    let planned = sandbox.run(
        &sandbox.bundle.join("enchi"),
        &[
            "self-install",
            "--json",
            "--dry-run",
            "--install-root",
            &sandbox.root.to_string_lossy(),
        ],
    );
    let value: serde_json::Value =
        serde_json::from_slice(&planned.stdout).expect("self-install --json prints JSON");
    assert_eq!(value["command"], "self-install");
    assert_eq!(value["dry_run"], true);
    assert_eq!(value["ok"], true, "{value}");
    assert!(!sandbox.root.exists());

    assert!(sandbox.install(&[]).status.success());
    let refused = sandbox.run(&sandbox.installed(), &["self-uninstall", "--json"]);
    assert_eq!(refused.status.code(), Some(1));
    assert!(text(&refused).contains("never prompts"));
    assert!(sandbox.installed().is_file());
}

fn legacy_updater_args(root: &Path) -> Vec<String> {
    [
        "self-install",
        "--install-root",
        &root.to_string_lossy(),
        "--shell",
        "none",
        "--add-to-user-path",
        "false",
        "--update-shell-profile",
        "false",
        "--refresh-shims",
        "true",
        "--install-powershell7",
        "false",
        "--yes",
        "--force",
    ]
    .map(String::from)
    .to_vec()
}

impl Sandbox {
    /// What a pyenv-native install looks like on disk: `bin/pyenv`, shims, a Python version, and a
    /// profile block fenced with the old markers.
    fn seed_legacy_install(&self) {
        let bin = self.root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("pyenv"), "old pyenv-native binary").unwrap();
        fs::create_dir_all(self.root.join("shims")).unwrap();
        fs::create_dir_all(self.root.join("versions").join("3.12.0")).unwrap();
        let old = bin.join("pyenv");
        fs::write(
            self.bashrc(),
            format!(
                "export KEEP=1\n\n# >>> pyenv-native init >>>\nif [ -x '{0}' ]; then\n  eval \"$('{0}' init - bash)\"\nfi\n# <<< pyenv-native init <<<\n",
                old.display()
            ),
        )
        .unwrap();
    }

    fn copy_into_bundle(&self, name: &str) {
        fs::copy(env!("CARGO_BIN_EXE_enchi"), self.bundle.join(name)).expect("copy into bundle");
    }
}

fn run_legacy_updater(sandbox: &Sandbox, bundle_binary: &str) -> Output {
    let args = legacy_updater_args(&sandbox.root);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    sandbox.run(&sandbox.bundle.join(bundle_binary), &args)
}

#[test]
fn the_pyenv_native_updater_upgrades_in_place_and_keeps_everything() {
    let sandbox = Sandbox::new();
    sandbox.seed_legacy_install();
    // The legacy release archive holds the CLI under both names, as the bridge release ships it.
    sandbox.copy_into_bundle("pyenv");

    let output = run_legacy_updater(&sandbox, "pyenv");
    assert!(output.status.success(), "{}", text(&output));

    let bin = sandbox.root.join("bin");
    assert!(
        bin.join("enchi").is_file(),
        "enchi installed beside the old binary"
    );
    assert_eq!(
        fs::read(bin.join("pyenv")).unwrap(),
        fs::read(bin.join("enchi")).unwrap(),
        "the pyenv alias is refreshed to the new binary"
    );
    assert!(
        sandbox.root.join("versions").join("3.12.0").is_dir(),
        "installed Python versions are never touched"
    );

    // The updater says --update-shell-profile false, but a block with the old markers is moved.
    let profile = fs::read_to_string(sandbox.bashrc()).unwrap();
    assert!(profile.starts_with("export KEEP=1\n"), "{profile}");
    assert_eq!(
        profile.matches("# >>> enchi init >>>").count(),
        1,
        "{profile}"
    );
    assert!(!profile.contains("pyenv-native init"), "{profile}");

    // Both command names work afterwards.
    for name in ["enchi", "pyenv"] {
        let version = sandbox.run(&bin.join(name), &["--version"]);
        assert!(version.status.success(), "{name}: {}", text(&version));
    }
    // And the root is still the old one, not a new ~/.enchi.
    let root = sandbox.run(&bin.join("enchi"), &["root"]);
    assert_eq!(
        text(&root).trim(),
        sandbox.root.to_string_lossy(),
        "the install root does not move"
    );
}

#[test]
fn a_bundle_that_only_has_the_pyenv_binary_still_installs_enchi() {
    let sandbox = Sandbox::new();
    sandbox.seed_legacy_install();
    fs::remove_file(sandbox.bundle.join("enchi")).unwrap();
    sandbox.copy_into_bundle("pyenv");

    let output = run_legacy_updater(&sandbox, "pyenv");
    assert!(output.status.success(), "{}", text(&output));
    assert!(sandbox.root.join("bin").join("enchi").is_file());
}

#[test]
fn fresh_installs_do_not_claim_the_pyenv_name_unless_asked() {
    let sandbox = Sandbox::new();
    assert!(sandbox.install(&[]).status.success());
    assert!(!sandbox.root.join("bin").join("pyenv").exists());

    let sandbox = Sandbox::new();
    assert!(
        sandbox
            .install(&["--legacy-aliases", "always"])
            .status
            .success()
    );
    assert!(sandbox.root.join("bin").join("pyenv").is_file());
}

#[test]
fn the_root_comes_from_enchi_root_then_pyenv_root_then_the_home_directory() {
    let sandbox = Sandbox::new();
    let root_of = |env: &[(&str, &Path)]| -> String {
        let mut command = Command::new(env!("CARGO_BIN_EXE_enchi"));
        command
            .arg("root")
            .env("HOME", &sandbox.home)
            .env_remove("PYENV_ROOT")
            .env_remove("ENCHI_ROOT");
        for (name, value) in env {
            command.env(name, value);
        }
        String::from_utf8_lossy(&command.output().unwrap().stdout)
            .trim()
            .to_string()
    };
    let a = sandbox.home.join("a");
    let b = sandbox.home.join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();

    assert_eq!(
        root_of(&[("ENCHI_ROOT", &a), ("PYENV_ROOT", &b)]),
        a.to_string_lossy()
    );
    assert_eq!(root_of(&[("PYENV_ROOT", &b)]), b.to_string_lossy());

    // Nothing set: a fresh home gets ~/.enchi; an existing ~/.pyenv is kept.
    assert_eq!(root_of(&[]), sandbox.home.join(".enchi").to_string_lossy());
    fs::create_dir_all(sandbox.home.join(".pyenv")).unwrap();
    assert_eq!(root_of(&[]), sandbox.home.join(".pyenv").to_string_lossy());
}
