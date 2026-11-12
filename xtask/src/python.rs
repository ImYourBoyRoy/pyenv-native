// ./xtask/src/python.rs
//! `cargo xtask python test|build|publish`: the pip/pipx install package under `python-package/`.
//!
//! The package itself stays Python (that is what `pipx` installs); only the automation around it
//! lives here, so it behaves the same on Windows, macOS and Linux.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand, ValueEnum};

use crate::repo_root;

#[derive(Subcommand)]
pub enum PythonCommand {
    /// Run the install package's unit tests.
    Test(InterpreterArgs),
    /// Build the wheel and sdist into `python-package/dist`.
    Build(BuildArgs),
    /// Validate (and optionally upload) the wheel/sdist with Twine.
    Publish(PublishArgs),
}

#[derive(Args, Clone)]
pub struct InterpreterArgs {
    /// Python interpreter to use (default: first working python3/python/py -3/pyenv python).
    #[arg(long, env = "XTASK_PYTHON")]
    python: Option<String>,
}

#[derive(Args)]
pub struct BuildArgs {
    #[command(flatten)]
    interpreter: InterpreterArgs,
    /// Output directory (default: python-package/dist).
    #[arg(long)]
    output_root: Option<PathBuf>,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Repository {
    Pypi,
    Testpypi,
}

#[derive(Args)]
pub struct PublishArgs {
    #[command(flatten)]
    interpreter: InterpreterArgs,
    #[arg(long, value_enum, default_value = "pypi")]
    repository: Repository,
    /// Upload to this index URL instead of `--repository`.
    #[arg(long)]
    repository_url: Option<String>,
    #[arg(long)]
    skip_tests: bool,
    #[arg(long)]
    skip_build: bool,
    /// Run `twine check` only; do not upload.
    #[arg(long)]
    check_only: bool,
    /// Print the commands instead of running them.
    #[arg(long)]
    dry_run: bool,
}

/// An interpreter invocation: a program plus fixed leading arguments (such as `py -3`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interpreter {
    program: String,
    prefix: Vec<String>,
}

impl Interpreter {
    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.prefix);
        command
    }

    /// The real interpreter path, or `None` when this candidate is not a working Python.
    fn probe(&self) -> Option<String> {
        if is_store_alias(&self.program) {
            return None;
        }
        let output = self
            .command()
            .args([
                "-c",
                "import os, sys; print(os.path.realpath(sys.executable))",
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let path = String::from_utf8_lossy(&output.stdout)
            .lines()
            .last()?
            .trim()
            .to_string();
        (!path.is_empty()).then_some(path)
    }
}

/// Windows ships `python.exe` stubs under `WindowsApps` that open the Store instead of running.
fn is_store_alias(program: &str) -> bool {
    program.replace('\\', "/").contains("/WindowsApps/")
}

/// Resolve an interpreter: explicit choice first, then `python3`, `python`, `py -3`, `pyenv`.
pub fn resolve(explicit: Option<&str>) -> Result<Interpreter> {
    if let Some(explicit) = explicit {
        let candidate = Interpreter {
            program: explicit.to_string(),
            prefix: Vec::new(),
        };
        let probed = candidate.probe().with_context(|| {
            format!("Python interpreter '{explicit}' was not found or is not a working interpreter")
        })?;
        return Ok(Interpreter {
            program: probed,
            prefix: Vec::new(),
        });
    }
    let candidates = [
        Interpreter {
            program: "python3".into(),
            prefix: vec![],
        },
        Interpreter {
            program: "python".into(),
            prefix: vec![],
        },
        Interpreter {
            program: "py".into(),
            prefix: vec!["-3".into()],
        },
    ];
    for candidate in candidates {
        if let Some(path) = candidate.probe() {
            return Ok(Interpreter {
                program: path,
                prefix: Vec::new(),
            });
        }
    }
    if let Ok(output) = Command::new("pyenv").args(["which", "python"]).output()
        && output.status.success()
    {
        let path = String::from_utf8_lossy(&output.stdout)
            .lines()
            .last()
            .unwrap_or("")
            .trim()
            .to_string();
        let candidate = Interpreter {
            program: path,
            prefix: Vec::new(),
        };
        if !candidate.program.is_empty()
            && let Some(path) = candidate.probe()
        {
            return Ok(Interpreter {
                program: path,
                prefix: Vec::new(),
            });
        }
    }
    bail!(
        "no usable Python interpreter was found; install Python, disable the broken Windows Store \
         alias, or pass --python <path>"
    )
}

pub fn run(command: &PythonCommand) -> Result<()> {
    match command {
        PythonCommand::Test(args) => test(&resolve(args.python.as_deref())?),
        PythonCommand::Build(args) => {
            let python = resolve(args.interpreter.python.as_deref())?;
            build(&python, args.output_root.as_deref())
        }
        PythonCommand::Publish(args) => publish(args),
    }
}

fn package_root() -> PathBuf {
    repo_root().join("python-package")
}

fn clean_caches(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if matches!(
                name.as_ref(),
                "__pycache__" | ".pytest_cache" | ".mypy_cache"
            ) {
                let _ = fs::remove_dir_all(&path);
            } else if name != ".git" && name != "dist" {
                clean_caches(&path);
            }
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("pyc") {
            let _ = fs::remove_file(&path);
        }
    }
}

/// Run the package's stdlib unittest suite with `PYTHONPATH=python-package/src`.
pub fn test(python: &Interpreter) -> Result<()> {
    let root = package_root();
    clean_caches(&root);
    let status = python
        .command()
        .args(["-m", "unittest", "discover", "-s", "tests", "-v"])
        .current_dir(&root)
        .env("PYTHONPATH", root.join("src"))
        .status()
        .context("failed to start Python")?;
    if !status.success() {
        bail!("Python install-package tests failed ({status})");
    }
    Ok(())
}

pub fn build(python: &Interpreter, output_root: Option<&Path>) -> Result<()> {
    let root = package_root();
    let out = output_root
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("dist"));
    fs::create_dir_all(&out)?;
    let status = python
        .command()
        .args(["-m", "pip", "install", "--upgrade", "build"])
        .status()
        .context("failed to start Python")?;
    if !status.success() {
        bail!("failed to install Python build tooling ({status})");
    }
    let status = python
        .command()
        .args(["-m", "build", "--outdir"])
        .arg(&out)
        .arg(&root)
        .status()?;
    if !status.success() {
        bail!("Python package build failed ({status})");
    }
    println!("output_root: {}", out.display());
    Ok(())
}

fn artifacts(dist: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dist)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.ends_with(".whl") || name.ends_with(".tar.gz")
        })
        .collect();
    found.sort();
    found
}

/// Twine environment: map `PYPI_API_TOKEN` to token auth when no explicit password is set.
pub fn twine_credentials(
    twine_password: Option<&str>,
    pypi_token: Option<&str>,
) -> Result<Option<(&'static str, String)>> {
    match (twine_password, pypi_token) {
        (Some(_), _) => Ok(None),
        (None, Some(token)) => Ok(Some(("__token__", token.to_string()))),
        (None, None) => bail!(
            "PyPI credentials were not found; set TWINE_PASSWORD/TWINE_USERNAME or PYPI_API_TOKEN before uploading"
        ),
    }
}

fn render(python: &Interpreter, args: &[String]) -> String {
    let mut parts = vec![python.program.clone()];
    parts.extend(python.prefix.iter().cloned());
    parts.extend(args.iter().cloned());
    parts
        .iter()
        .map(|part| {
            if part.contains(char::is_whitespace) {
                format!("'{part}'")
            } else {
                part.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn publish(args: &PublishArgs) -> Result<()> {
    let python = resolve(args.interpreter.python.as_deref())?;
    if !args.skip_tests {
        println!("[python publish] running install-package tests");
        if args.dry_run {
            println!("DRY-RUN: cargo xtask python test");
        } else {
            test(&python)?;
        }
    }
    if !args.skip_build {
        println!("[python publish] building install-package artifacts");
        if args.dry_run {
            println!("DRY-RUN: cargo xtask python build");
        } else {
            build(&python, None)?;
        }
    }

    let dist = package_root().join("dist");
    let files = artifacts(&dist);
    if files.is_empty() && !args.dry_run {
        bail!(
            "no Python distribution artifacts were found under {}",
            dist.display()
        );
    }
    let file_args: Vec<String> = files
        .iter()
        .map(|path| path.display().to_string())
        .collect();

    let twine = |extra: &[&str]| -> Vec<String> {
        let mut all: Vec<String> = vec!["-m".into(), "twine".into()];
        all.extend(extra.iter().map(|s| s.to_string()));
        all.extend(file_args.iter().cloned());
        all
    };
    let run_python = |arguments: &[String], envs: &[(&str, String)]| -> Result<()> {
        if args.dry_run {
            println!("DRY-RUN: {}", render(&python, arguments));
            return Ok(());
        }
        let status = python
            .command()
            .args(arguments)
            .envs(envs.iter().map(|(k, v)| (*k, v)))
            .status()?;
        if !status.success() {
            bail!("`{}` failed ({status})", render(&python, arguments));
        }
        Ok(())
    };

    if !args.dry_run {
        let available = python.command().args(["-m", "twine", "--version"]).output();
        if !available.map(|out| out.status.success()).unwrap_or(false) {
            bail!(
                "Twine is not available; install it with `{} -m pip install twine`",
                python.program
            );
        }
    }
    println!("[python publish] running Twine metadata checks");
    run_python(&twine(&["check"]), &[])?;

    if args.check_only {
        println!("[python publish] check-only mode; skipping upload");
        return Ok(());
    }

    let mut envs: Vec<(&str, String)> = Vec::new();
    if !args.dry_run {
        let password = std::env::var("TWINE_PASSWORD").ok();
        let token = std::env::var("PYPI_API_TOKEN").ok();
        if let Some((user, pass)) = twine_credentials(password.as_deref(), token.as_deref())? {
            envs.push(("TWINE_USERNAME", user.to_string()));
            envs.push(("TWINE_PASSWORD", pass));
        }
    }
    let mut upload = vec!["upload", "--non-interactive"];
    let url_holder;
    if let Some(url) = &args.repository_url {
        url_holder = url.clone();
        upload.extend(["--repository-url", url_holder.as_str()]);
    } else if matches!(args.repository, Repository::Testpypi) {
        upload.extend(["--repository", "testpypi"]);
    }
    println!("[python publish] uploading {} artifact(s)", files.len());
    run_python(&twine(&upload), &envs)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_store_aliases_are_rejected() {
        assert!(is_store_alias(
            r"C:\Users\me\AppData\Local\Microsoft\WindowsApps\python.exe"
        ));
        assert!(!is_store_alias("/usr/bin/python3"));
        let alias = Interpreter {
            program: r"C:\x\WindowsApps\python.exe".into(),
            prefix: vec![],
        };
        assert_eq!(alias.probe(), None);
    }

    #[test]
    fn twine_credentials_prefer_explicit_password() {
        assert_eq!(twine_credentials(Some("pw"), Some("tok")).unwrap(), None);
        assert_eq!(
            twine_credentials(None, Some("tok")).unwrap(),
            Some(("__token__", "tok".to_string()))
        );
        assert!(twine_credentials(None, None).is_err());
    }

    #[test]
    fn rendering_quotes_arguments_with_spaces() {
        let python = Interpreter {
            program: "py".into(),
            prefix: vec!["-3".into()],
        };
        assert_eq!(
            render(&python, &["-m".into(), "a b".into()]),
            "py -3 -m 'a b'"
        );
    }

    #[test]
    fn artifact_discovery_picks_wheels_and_sdists() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["a-1-py3-none-any.whl", "a-1.tar.gz", "notes.txt"] {
            fs::write(dir.path().join(name), "").unwrap();
        }
        let names: Vec<String> = artifacts(dir.path())
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["a-1-py3-none-any.whl", "a-1.tar.gz"]);
    }

    #[test]
    fn cache_cleaning_removes_bytecode_but_keeps_sources() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("pkg").join("__pycache__");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("x.pyc"), "").unwrap();
        fs::write(dir.path().join("pkg").join("mod.py"), "").unwrap();
        fs::write(dir.path().join("stray.pyc"), "").unwrap();
        clean_caches(dir.path());
        assert!(!cache.exists());
        assert!(!dir.path().join("stray.pyc").exists());
        assert!(dir.path().join("pkg").join("mod.py").exists());
    }
}
