// ./crates/enchi-core/src/pip/held.rs
//! Packages that a bulk update must leave alone.
//!
//! `pip list --outdated` compares what is installed with the newest release on the default index.
//! That is wrong for a package installed from a different index on purpose. The usual case is
//! PyTorch: `torch 2.14.1+cu126` comes from `download.pytorch.org/whl/cu126` because that build has
//! kernels for older GPUs (a Tesla P40 needs it), but PyPI's newest `torch 2.15.0` is the CUDA 13
//! build without them. "Updating" it breaks the GPU, and `pip check` cannot notice. Such builds
//! carry a PEP 440 *local version label* (`+cu126`, `+cpu`, `+rocm6.2`), so they are held back.
//!
//! Their dependencies are held too whenever the newest release would break what the held package
//! requires: `torch 2.14.1+cu126` pins `cuda-toolkit[cublas,...]==12.6.3` (which pins the CUDA
//! libraries through its extras), `triton~=3.8.0` and `cuda-bindings<13`, so none of those may move
//! to a release outside those ranges. The check follows requirements and extras transitively and
//! evaluates them with pip's own `packaging`, so it understands every specifier and marker.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;

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

/// Split `outdated` into the packages to update and the ones to hold back.
///
/// Held: packages whose installed version has a local build label, and outdated packages whose
/// newest release would break a requirement of a held package. `py` is used to read the installed
/// packages' requirements; if that fails, only the labelled packages are held.
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
    if !outdated
        .iter()
        .any(|package| local_label(&package.version).is_some())
    {
        // Fast path. A local build that is already current is not reported as outdated, and then
        // nothing it pins can be outdated in a way that matters before it is updated itself.
        return (outdated, Vec::new());
    }

    let blocked = blocked_updates(py, &outdated, python_path).unwrap_or_default();

    let mut updatable = Vec::new();
    let mut held = Vec::new();
    for package in outdated {
        if let Some(label) = local_label(&package.version) {
            held.push(HeldPackage {
                reason: format!("local build +{label}, installed from another index on purpose"),
                name: package.name,
                version: package.version,
            });
        } else if let Some((requirer, requirement, holder)) = blocked.get(&normalize(&package.name))
        {
            let who = if normalize(requirer) == normalize(holder) {
                requirer.clone()
            } else {
                format!("{requirer} (needed by {holder})")
            };
            held.push(HeldPackage {
                reason: format!(
                    "{who} requires {}{requirement}, which {} would break",
                    package.name, package.latest_version
                ),
                name: package.name,
                version: package.version,
            });
        } else {
            updatable.push(package);
        }
    }
    (updatable, held)
}

// Raw string on purpose: a `\` line continuation would strip the indentation Python needs.
//
// Input (stdin): {"latest": {name: newest version}}. Output: {name: [requirer, specifier, holder]} for every
// candidate whose newest version violates a requirement of an installed package carrying a local
// version label, following requirements (and the extras they ask for) transitively. Output:
// {name: [package stating the requirement, specifier, local build it is needed by]}.
const BLOCKED_SCRIPT: &str = r#"
import importlib.metadata as m, json, sys
try:
    from packaging.requirements import Requirement
    from packaging.utils import canonicalize_name
except ImportError:
    from pip._vendor.packaging.requirements import Requirement
    from pip._vendor.packaging.utils import canonicalize_name

latest = {canonicalize_name(k): v for k, v in json.load(sys.stdin)["latest"].items()}
holders = sorted({canonicalize_name(d.metadata["Name"]) for d in m.distributions() if "+" in (d.version or "")})
blocked, seen, queue = {}, set(), [(h, frozenset(), h) for h in holders]
while queue:
    name, extras, holder = queue.pop()
    if (name, extras) in seen:
        continue
    seen.add((name, extras))
    try:
        requires = m.distribution(name).requires or []
    except m.PackageNotFoundError:
        continue
    for raw in requires:
        try:
            req = Requirement(raw)
        except Exception:
            continue
        if req.marker is not None and not any(req.marker.evaluate({"extra": e}) for e in (extras or {""})):
            continue
        dep = canonicalize_name(req.name)
        version = latest.get(dep)
        if version is not None and len(req.specifier) and not req.specifier.contains(version, prereleases=True):
            blocked.setdefault(dep, [name, str(req.specifier), holder])
        queue.append((dep, frozenset(canonicalize_name(e) for e in req.extras), holder))
print(json.dumps(blocked))
"#;

fn blocked_updates(
    py: &Path,
    outdated: &[OutdatedPackage],
    python_path: Option<&Path>,
) -> Option<BTreeMap<String, (String, String, String)>> {
    let latest: BTreeMap<&str, &str> = outdated
        .iter()
        .map(|package| (package.name.as_str(), package.latest_version.as_str()))
        .collect();
    let input = serde_json::json!({ "latest": latest }).to_string();

    let mut command = pip_subcommand(py);
    command
        .args(["-c", BLOCKED_SCRIPT])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(path) = python_path {
        command.env("PYTHONPATH", path);
    }
    let mut child = command.spawn().ok()?;
    child.stdin.take()?.write_all(input.as_bytes()).ok()?;
    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    let parsed: BTreeMap<String, (String, String, String)> =
        serde_json::from_slice(&output.stdout).ok()?;
    Some(parsed)
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

    /// A site-packages directory holding only metadata, shaped like a real CUDA PyTorch install: a
    /// build with a local label that pins libraries through another package's extras, a
    /// compatible-release pin, an upper bound and plain ranges.
    fn fake_site_packages(dir: &Path) {
        let dist_info = |name: &str, version: &str, requires: &[&str]| {
            // Wheels name the metadata folder with underscores, which is what lookup by name expects.
            let folder = dir.join(format!("{}-{version}.dist-info", name.replace('-', "_")));
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
                // No platform markers here: the real ones say `platform_system == "Linux"`, which is
                // false on macOS and Windows runners, and the check rightly honours markers (see
                // `a_requirement_that_does_not_apply_here_holds_nothing`).
                "cuda-toolkit[cublas]==12.6.3",
                "nvidia-cudnn-cu12==9.10.2.21",
                "triton~=3.8.0",
                "plan9-only==1.0; platform_system == \"Plan9\"",
                "cuda-bindings<13,>=12.9.4",
                "sympy>=1.13",
                "extra-only==1.0; extra == \"opt\"",
            ],
        );
        dist_info(
            "cuda-toolkit",
            "12.6.3",
            &[
                "nvidia-cublas-cu12==12.6.4.1.*; extra == 'cublas'",
                "nvidia-cufft-cu12==11.3.0.4.*; extra == 'cufft'",
            ],
        );
        for (name, version) in [
            ("nvidia-cublas-cu12", "12.6.4.1"),
            ("nvidia-cufft-cu12", "11.3.0.4"),
            ("nvidia-cudnn-cu12", "9.10.2.21"),
            ("triton", "3.8.0"),
            ("cuda-bindings", "12.9.10"),
            ("sympy", "1.13.0"),
            ("plan9-only", "1.0"),
            ("requests", "2.30.0"),
        ] {
            dist_info(name, version, &[]);
        }
    }

    /// An interpreter that can run the helper (it needs `packaging`, from pip's vendored copy at
    /// worst). `None` skips the end-to-end tests on a machine without one.
    fn python3() -> Option<std::path::PathBuf> {
        ["python3", "python"].into_iter().find_map(|name| {
            let ok = std::process::Command::new(name)
                .args(["-c", "import pip._vendor.packaging.requirements"])
                .output()
                .ok()?
                .status
                .success();
            ok.then(|| std::path::PathBuf::from(name))
        })
    }

    fn held_names(held: &[HeldPackage]) -> Vec<String> {
        held.iter().map(|package| package.name.clone()).collect()
    }

    #[test]
    fn a_cuda_build_and_everything_that_would_break_it_is_held_with_a_real_interpreter() {
        let Some(python) = python3() else {
            return;
        };
        let site = tempfile::tempdir().unwrap();
        fake_site_packages(site.path());
        let outdated = vec![
            outdated("torchlike", "2.14.1+cu126", "2.15.0"),
            // Pinned through cuda-toolkit's `cublas` extra, which torchlike asks for.
            outdated("nvidia-cublas-cu12", "12.6.4.1", "12.9.3.13"),
            // Not requested by any extra, so cuda-toolkit does not constrain it.
            outdated("nvidia-cufft-cu12", "11.3.0.4", "11.4.1.4"),
            outdated("nvidia-cudnn-cu12", "9.10.2.21", "9.28.1.4"),
            outdated("triton", "3.8.0", "3.9.0"),
            outdated("cuda-bindings", "12.9.10", "13.5.0"),
            outdated("sympy", "1.13.0", "1.14.0"),
            outdated("requests", "2.30.0", "2.34.2"),
        ];
        let (updatable, held) = partition_with_path(&python, outdated, Some(site.path()));

        assert_eq!(
            held_names(&held),
            [
                "torchlike",
                "nvidia-cublas-cu12",
                "nvidia-cudnn-cu12",
                "triton",
                "cuda-bindings"
            ]
        );
        assert!(held[0].reason.contains("+cu126"), "{}", held[0].reason);
        assert_eq!(
            held[1].reason,
            "cuda-toolkit (needed by torchlike) requires nvidia-cublas-cu12==12.6.4.1.*, which 12.9.3.13 would break"
        );
        assert!(held[3].reason.contains("~=3.8.0"), "{}", held[3].reason);
        assert!(held[4].reason.contains("<13"), "{}", held[4].reason);
        // A range the new version satisfies, an unconstrained library and an unrelated package.
        assert_eq!(
            updatable
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["nvidia-cufft-cu12", "sympy", "requests"]
        );
    }

    #[test]
    fn a_requirement_that_does_not_apply_here_holds_nothing() {
        let Some(python) = python3() else {
            return;
        };
        let site = tempfile::tempdir().unwrap();
        fake_site_packages(site.path());
        // torchlike requires `plan9-only==1.0` only on Plan 9, so on any machine this test runs on
        // its marker is false and the package is free to update.
        let outdated = vec![
            outdated("torchlike", "2.14.1+cu126", "2.15.0"),
            outdated("plan9-only", "1.0", "2.0"),
        ];
        let (updatable, held) = partition_with_path(&python, outdated, Some(site.path()));
        assert_eq!(held_names(&held), ["torchlike"]);
        assert_eq!(updatable[0].name, "plan9-only");
    }

    #[test]
    fn a_newer_release_that_still_fits_the_held_package_is_not_held() {
        let Some(python) = python3() else {
            return;
        };
        let site = tempfile::tempdir().unwrap();
        fake_site_packages(site.path());
        let outdated = vec![
            outdated("torchlike", "2.14.1+cu126", "2.15.0"),
            // `triton~=3.8.0` allows any 3.8.x, so a patch release is safe to take.
            outdated("triton", "3.8.0", "3.8.5"),
        ];
        let (updatable, held) = partition_with_path(&python, outdated, Some(site.path()));
        assert_eq!(held_names(&held), ["torchlike"]);
        assert_eq!(updatable.len(), 1);
        assert_eq!(updatable[0].name, "triton");
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
