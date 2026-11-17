// ./crates/enchi-core/src/pip/held.rs
//! Packages that a bulk update must leave alone.
//!
//! `pip list --outdated` compares what is installed with the newest release on the default index.
//! That is wrong for a package installed from a different index on purpose. The usual case is
//! PyTorch: `torch 2.14.1+cu126` comes from `download.pytorch.org/whl/cu126` because that build has
//! kernels for older GPUs (a Tesla P40 needs it), but PyPI's newest `torch 2.15.0` is the CUDA 13
//! build without them. "Updating" it breaks the GPU, and `pip check` cannot notice. Such builds
//! carry a PEP 440 *local version label* (`+cu126`, `+cpu`, `+rocm6.2`), so they are held back,
//! together with the dependencies they pin to an exact version (`nvidia-cublas-cu12==12.6.4.1`),
//! because moving those apart from the package that pins them breaks it.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use super::operations::pip_subcommand;
use super::types::OutdatedPackage;

/// A package left out of a bulk update, with the reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HeldPackage {
    pub name: String,
    pub version: String,
    pub reason: String,
}

/// The local version label of `version`, if it has one (`2.14.1+cu126` gives `cu126`).
pub fn local_label(version: &str) -> Option<&str> {
    let (_, label) = version.split_once('+')?;
    (!label.is_empty()).then_some(label)
}

/// Canonical form used to compare project names (PEP 503).
pub fn normalize(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut separator = false;
    for ch in name.chars() {
        if matches!(ch, '-' | '_' | '.') {
            separator = true;
        } else {
            if separator && !out.is_empty() {
                out.push('-');
            }
            separator = false;
            out.push(ch.to_ascii_lowercase());
        }
    }
    out
}

/// Names that `requirements` (the `Requires-Dist` lines of one distribution) pin to an exact
/// version, for example `triton==3.5.1; platform_system == "Linux"` gives `triton`. Requirements
/// that only apply to an optional extra are ignored.
pub fn exact_pins(requirements: &[String]) -> Vec<String> {
    let mut pinned = Vec::new();
    for requirement in requirements {
        let (spec, marker) = match requirement.split_once(';') {
            Some((spec, marker)) => (spec.trim(), marker),
            None => (requirement.trim(), ""),
        };
        if marker.contains("extra ==") || marker.contains("extra==") {
            continue;
        }
        let name_end = spec
            .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
            .unwrap_or(spec.len());
        let name = &spec[..name_end];
        if name.is_empty() {
            continue;
        }
        let mut rest = spec[name_end..].trim_start();
        if rest.starts_with('[') {
            match rest.find(']') {
                Some(close) => rest = rest[close + 1..].trim_start(),
                None => continue,
            }
        }
        let rest = rest.trim_start_matches('(').trim_start();
        // `==` pins a version; `===` is an arbitrary-equality escape hatch, treated the same.
        if rest.starts_with("==") && !rest.contains(',') {
            pinned.push(normalize(name));
        }
    }
    pinned
}

/// Split `outdated` into the packages to update and the ones to hold back.
///
/// Held: packages whose installed version has a local build label, and outdated packages that a
/// held package pins exactly. `py` is only used to read the held packages' requirements; if that
/// fails, only the first group is held.
pub fn partition(
    py: &Path,
    outdated: Vec<OutdatedPackage>,
) -> (Vec<OutdatedPackage>, Vec<HeldPackage>) {
    partition_with_path(py, outdated, None)
}

/// `partition`, with `python_path` added to the interpreter's search path while it reads package
/// metadata. Lets tests describe an environment with plain directories.
pub(crate) fn partition_with_path(
    py: &Path,
    outdated: Vec<OutdatedPackage>,
    python_path: Option<&Path>,
) -> (Vec<OutdatedPackage>, Vec<HeldPackage>) {
    let local_builds: Vec<&OutdatedPackage> = outdated
        .iter()
        .filter(|package| local_label(&package.version).is_some())
        .collect();
    if local_builds.is_empty() {
        return (outdated, Vec::new());
    }

    // Everything installed with a local label is a reason to hold its pins, not only the ones that
    // happen to be outdated: `torch` may be current while its CUDA libraries are not.
    let labelled: Vec<String> = installed_with_local_label(py, python_path)
        .unwrap_or_else(|| local_builds.iter().map(|p| p.name.clone()).collect());
    let requirements = requirements_of(py, &labelled, python_path).unwrap_or_default();

    let mut pinned_by: BTreeMap<String, String> = BTreeMap::new();
    for (holder, lines) in &requirements {
        for pin in exact_pins(lines) {
            pinned_by.entry(pin).or_insert_with(|| holder.clone());
        }
    }

    let mut updatable = Vec::new();
    let mut held = Vec::new();
    for package in outdated {
        if let Some(label) = local_label(&package.version) {
            held.push(HeldPackage {
                reason: format!("local build +{label}, installed from another index on purpose"),
                name: package.name,
                version: package.version,
            });
        } else if let Some(holder) = pinned_by.get(&normalize(&package.name)) {
            held.push(HeldPackage {
                reason: format!("pinned to an exact version by {holder}"),
                name: package.name,
                version: package.version,
            });
        } else {
            updatable.push(package);
        }
    }
    (updatable, held)
}

// Raw strings on purpose: a `\` line continuation would strip the indentation Python needs.
const LIST_LOCAL_SCRIPT: &str = r#"
import importlib.metadata as m, json
print(json.dumps(sorted({d.metadata['Name'] for d in m.distributions() if '+' in (d.version or '')})))
"#;

const REQUIRES_SCRIPT: &str = r#"
import importlib.metadata as m, json, sys
out = {}
for name in sys.argv[1:]:
    try:
        out[name] = list(m.distribution(name).requires or [])
    except m.PackageNotFoundError:
        out[name] = []
print(json.dumps(out))
"#;

fn installed_with_local_label(py: &Path, python_path: Option<&Path>) -> Option<Vec<String>> {
    let mut command = pip_subcommand(py);
    command.args(["-c", LIST_LOCAL_SCRIPT]);
    if let Some(path) = python_path {
        command.env("PYTHONPATH", path);
    }
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

fn requirements_of(
    py: &Path,
    names: &[String],
    python_path: Option<&Path>,
) -> Option<BTreeMap<String, Vec<String>>> {
    if names.is_empty() {
        return Some(BTreeMap::new());
    }
    let mut command = pip_subcommand(py);
    command.args(["-c", REQUIRES_SCRIPT]).args(names);
    if let Some(path) = python_path {
        command.env("PYTHONPATH", path);
    }
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

/// One line per held package, for human output.
pub fn describe(held: &[HeldPackage]) -> Vec<String> {
    held.iter()
        .map(|package| {
            format!(
                "held back {} {}: {}",
                package.name, package.version, package.reason
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn local_labels_are_detected() {
        assert_eq!(local_label("2.14.1+cu126"), Some("cu126"));
        assert_eq!(local_label("2.6.0+cpu"), Some("cpu"));
        assert_eq!(local_label("6.2.0+rocm6.2"), Some("rocm6.2"));
        assert_eq!(local_label("2.14.1"), None);
        assert_eq!(local_label("1.0+"), None);
    }

    #[test]
    fn names_compare_the_way_pip_does() {
        assert_eq!(normalize("NVIDIA_cuBLAS.cu12"), "nvidia-cublas-cu12");
        assert_eq!(normalize("Jinja2"), "jinja2");
        assert_eq!(normalize("zope.interface"), "zope-interface");
    }

    #[test]
    fn exact_pins_are_found_in_requires_dist_lines() {
        let torch = lines(&[
            "filelock",
            "typing-extensions>=4.10.0",
            "sympy>=1.13.3",
            "nvidia-cublas-cu12==12.6.4.1; platform_system == \"Linux\" and platform_machine == \"x86_64\"",
            "nvidia-cudnn-cu12==9.10.2.21; platform_system == \"Linux\"",
            "triton==3.5.1; platform_system == \"Linux\"",
            "cuda-toolkit[cublas,cudart]==12.6.3; platform_system == \"Linux\"",
            "setuptools; python_version >= \"3.12\"",
            "opt-einsum>=3.3; extra == \"opt-einsum\"",
            "numpy==2.0.0; extra == \"optional\"",
            "ranged>=1,<2",
            "spaced == 1.2",
            "paren (==3.3)",
            "tilde~=1.4",
        ]);
        assert_eq!(
            exact_pins(&torch),
            [
                "nvidia-cublas-cu12",
                "nvidia-cudnn-cu12",
                "triton",
                "cuda-toolkit",
                "spaced",
                "paren"
            ]
        );
    }

    #[test]
    fn a_pin_with_more_than_one_clause_is_not_exact() {
        assert!(exact_pins(&lines(&["pkg==1.0,!=1.0.1"])).is_empty());
    }

    fn outdated(name: &str, version: &str, latest: &str) -> OutdatedPackage {
        OutdatedPackage {
            name: name.to_string(),
            version: version.to_string(),
            latest_version: latest.to_string(),
        }
    }

    #[test]
    fn nothing_is_held_without_a_local_build() {
        let packages = vec![
            outdated("requests", "2.30.0", "2.34.2"),
            outdated("pip", "25.0", "26.3"),
        ];
        let (updatable, held) = partition(Path::new("/nonexistent/python"), packages.clone());
        assert_eq!(updatable, packages);
        assert!(held.is_empty());
    }

    #[test]
    fn a_local_build_is_held_even_if_its_requirements_cannot_be_read() {
        // No interpreter at that path, so the pin lookup falls back to holding only the local build.
        let packages = vec![
            outdated("torch", "2.14.1+cu126", "2.15.0"),
            outdated("requests", "2.30.0", "2.34.2"),
        ];
        let (updatable, held) = partition(Path::new("/nonexistent/python"), packages);
        assert_eq!(updatable.len(), 1);
        assert_eq!(updatable[0].name, "requests");
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].name, "torch");
        assert!(held[0].reason.contains("+cu126"), "{}", held[0].reason);
    }

    /// A site-packages directory holding only metadata: a CUDA build of `torchlike` that pins two
    /// libraries exactly, a pinned library, and a package that nothing pins.
    fn fake_site_packages(dir: &Path) {
        let dist_info = |name: &str, version: &str, requires: &[&str]| {
            let folder = dir.join(format!("{name}-{version}.dist-info"));
            std::fs::create_dir_all(&folder).unwrap();
            let mut metadata = format!("Metadata-Version: 2.1\nName: {name}\nVersion: {version}\n");
            for requirement in requires {
                metadata.push_str(&format!("Requires-Dist: {requirement}\n"));
            }
            std::fs::write(folder.join("METADATA"), metadata).unwrap();
        };
        dist_info(
            "torchlike",
            "2.14.1+cu126",
            &[
                "nvidia-cublas-cu12==12.6.4.1; platform_system == \"Linux\"",
                "triton==3.5.1",
                "sympy>=1.13",
                "extra-only==1.0; extra == \"opt\"",
            ],
        );
        dist_info("nvidia-cublas-cu12", "12.6.4.1", &[]);
        dist_info("triton", "3.5.1", &[]);
        dist_info("sympy", "1.13.0", &[]);
        dist_info("requests", "2.30.0", &[]);
    }

    fn python3() -> Option<std::path::PathBuf> {
        ["python3", "python"].into_iter().find_map(|name| {
            let output = std::process::Command::new(name)
                .arg("--version")
                .output()
                .ok()?;
            output
                .status
                .success()
                .then(|| std::path::PathBuf::from(name))
        })
    }

    #[test]
    fn a_cuda_build_and_the_libraries_it_pins_are_held_with_a_real_interpreter() {
        let Some(python) = python3() else {
            return; // no interpreter on this machine; the pure tests above still run
        };
        let site = tempfile::tempdir().unwrap();
        fake_site_packages(site.path());
        let outdated = vec![
            outdated("torchlike", "2.14.1+cu126", "2.15.0"),
            outdated("nvidia-cublas-cu12", "12.6.4.1", "12.9.3.13"),
            outdated("triton", "3.5.1", "3.9.0"),
            outdated("sympy", "1.13.0", "1.14.0"),
            outdated("requests", "2.30.0", "2.34.2"),
        ];
        let (updatable, held) = partition_with_path(&python, outdated, Some(site.path()));

        let names =
            |items: &[HeldPackage]| items.iter().map(|p| p.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&held), ["torchlike", "nvidia-cublas-cu12", "triton"]);
        assert!(held[0].reason.contains("+cu126"), "{}", held[0].reason);
        assert_eq!(held[1].reason, "pinned to an exact version by torchlike");
        // `sympy>=` is a range, not a pin, and an unrelated package is updated as usual.
        assert_eq!(
            updatable
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["sympy", "requests"]
        );
    }

    #[test]
    fn describe_gives_one_readable_line_per_held_package() {
        let held = vec![HeldPackage {
            name: "torch".into(),
            version: "2.14.1+cu126".into(),
            reason: "local build +cu126, installed from another index on purpose".into(),
        }];
        assert_eq!(
            describe(&held),
            [
                "held back torch 2.14.1+cu126: local build +cu126, installed from another index on purpose"
            ]
        );
        assert!(describe(&[]).is_empty());
    }
}
