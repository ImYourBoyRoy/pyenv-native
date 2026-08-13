// ./crates/pyenv-core/src/preflight/intel.rs
//! Build the user/agent-facing platform intelligence snapshot used by preflight.

use std::env;
use std::fs;
use std::path::Path;

use crate::context::AppContext;
use crate::doctor::{DoctorStatus, collect_checks, doctor_fix_plan};
use crate::install::resolve_install_plan;

use super::android::inspect_android_toolchain;
use super::macos::inspect_macos_toolchain;
use super::types::{
    PlatformFact, PlatformIntelligence, PreflightVerdict, affects_install_readiness,
    is_blocking_check,
};

pub fn build_platform_intelligence(ctx: &AppContext) -> PlatformIntelligence {
    let os = env::consts::OS.to_string();
    let arch = env::consts::ARCH.to_string();
    let os_pretty_name = detect_os_pretty_name(&os);
    let shell = ctx.env_shell.clone().or_else(|| {
        env::var("SHELL").ok().and_then(|value| {
            Path::new(&value)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
    });
    let install_strategy = describe_install_strategy(&os);
    let source_build_required = matches!(os.as_str(), "macos" | "linux" | "android");

    let checks = collect_checks(ctx);

    let mut blocking_issues = Vec::new();
    let mut warnings = Vec::new();
    for check in &checks {
        match check.status {
            DoctorStatus::Warn if is_blocking_check(&check.name) => {
                blocking_issues.push(format!("{}: {}", check.name, check.detail));
            }
            DoctorStatus::Warn if affects_install_readiness(&check.name) => {
                warnings.push(format!("{}: {}", check.name, check.detail));
            }
            // Shell PATH / shim / selection health stay in doctor output; they do not
            // dilute Host Environment install-readiness.
            DoctorStatus::Warn => {}
            _ => {}
        }
    }

    let recommended_actions = doctor_fix_plan(ctx)
        .into_iter()
        .filter(|fix| {
            fix.key.contains("source-build")
                || fix.key.contains("macos")
                || fix.key.contains("termux")
                || fix.key.contains("android")
                || fix.key.contains("path-")
                || fix.automated
        })
        .collect::<Vec<_>>();

    let verdict = PlatformIntelligence::derive_verdict(&checks, &blocking_issues);
    let ready_to_install = matches!(
        verdict,
        PreflightVerdict::Ready | PreflightVerdict::NeedsAttention
    ) && blocking_issues.is_empty();

    let summary = match verdict {
        PreflightVerdict::Ready => format!(
            "{os_pretty_name} looks ready for pyenv-native runtime installs via {install_strategy}. Shell PATH and shim health are reported in the shell cards and doctor section below."
        ),
        PreflightVerdict::NeedsAttention => format!(
            "{os_pretty_name} can mostly install runtimes via {install_strategy}, but some install prerequisites need attention."
        ),
        PreflightVerdict::Blocked => format!(
            "{os_pretty_name} is not ready for a successful source/runtime install yet. Resolve blocking prerequisites first."
        ),
    };

    let mut facts = vec![
        PlatformFact {
            key: "os".to_string(),
            label: "Operating system".to_string(),
            value: os_pretty_name.clone(),
        },
        PlatformFact {
            key: "arch".to_string(),
            label: "CPU architecture".to_string(),
            value: arch.clone(),
        },
        PlatformFact {
            key: "install-strategy".to_string(),
            label: "Python install strategy".to_string(),
            value: install_strategy.clone(),
        },
        PlatformFact {
            key: "pyenv-root".to_string(),
            label: "PYENV_ROOT".to_string(),
            value: ctx.root.display().to_string(),
        },
    ];

    if let Some(shell) = shell.clone() {
        facts.push(PlatformFact {
            key: "shell".to_string(),
            label: "Shell".to_string(),
            value: shell,
        });
    }

    facts.push(PlatformFact {
        key: "build-mode".to_string(),
        label: "Build mode".to_string(),
        value: if source_build_required {
            "Source compile (configure + make)".to_string()
        } else {
            "Prebuilt package extract".to_string()
        },
    });

    if os == "macos" {
        let macos = inspect_macos_toolchain();
        facts.push(PlatformFact {
            key: "xcode-clt".to_string(),
            label: "Xcode / CLT".to_string(),
            value: macos.clt_detail.clone(),
        });
        if let Some(version) = macos.xcodebuild_version {
            facts.push(PlatformFact {
                key: "xcodebuild".to_string(),
                label: "xcodebuild".to_string(),
                value: version,
            });
        }
        facts.push(PlatformFact {
            key: "openssl".to_string(),
            label: "OpenSSL (TLS build dep)".to_string(),
            value: macos.openssl_detail.clone(),
        });
        facts.push(PlatformFact {
            key: "homebrew".to_string(),
            label: "Homebrew".to_string(),
            value: if macos.brew_available {
                "available".to_string()
            } else {
                "not found".to_string()
            },
        });
    }

    let android = inspect_android_toolchain();
    if os == "android" || android.is_termux {
        facts.push(PlatformFact {
            key: "termux-prefix".to_string(),
            label: "Termux PREFIX".to_string(),
            value: android
                .prefix
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "missing".to_string()),
        });
        if let Some(api) = android.api_level {
            facts.push(PlatformFact {
                key: "android-api".to_string(),
                label: "Android API level".to_string(),
                value: api.to_string(),
            });
        }
        facts.push(PlatformFact {
            key: "termux-readiness".to_string(),
            label: "Termux build readiness".to_string(),
            value: android.detail,
        });
    }

    if let Ok(plan) = resolve_install_plan(ctx, "3") {
        facts.push(PlatformFact {
            key: "sample-provider".to_string(),
            label: "Latest CPython provider".to_string(),
            value: format!("{} → {}", plan.provider, plan.resolved_version),
        });
    }

    PlatformIntelligence {
        os,
        arch,
        os_pretty_name,
        shell,
        pyenv_root: ctx.root.display().to_string(),
        install_strategy,
        source_build_required,
        ready_to_install,
        verdict,
        summary,
        facts,
        checks,
        blocking_issues,
        warnings,
        recommended_actions,
    }
}

fn describe_install_strategy(os: &str) -> String {
    match os {
        "windows" => "windows NuGet / embeddable packages (no local CPython compile)".to_string(),
        "macos" => "official CPython source build (requires Xcode CLT + OpenSSL)".to_string(),
        "android" => {
            "Termux CPython source build (requires clang/make/OpenSSL headers)".to_string()
        }
        "linux" => {
            "official CPython source build (requires compiler toolchain + headers)".to_string()
        }
        other => format!("{other} native provider resolution"),
    }
}

fn detect_os_pretty_name(os: &str) -> String {
    match os {
        "macos" => {
            let product = read_command_line(&["sw_vers", "-productName"])
                .unwrap_or_else(|| "macOS".to_string());
            let version = read_command_line(&["sw_vers", "-productVersion"]).unwrap_or_default();
            if version.is_empty() {
                product
            } else {
                format!("{product} {version}")
            }
        }
        "windows" => {
            // %OS% is usually the opaque "Windows_NT"; prefer a human host label.
            let product = env::var("OS").unwrap_or_else(|_| "Windows".to_string());
            if product.eq_ignore_ascii_case("Windows_NT") || product.is_empty() {
                format!("Windows ({})", env::consts::ARCH)
            } else {
                product
            }
        }
        "android" => {
            let release = read_command_line(&["getprop", "ro.build.version.release"])
                .unwrap_or_else(|| "Android".to_string());
            if release.eq_ignore_ascii_case("android") {
                "Android / Termux".to_string()
            } else {
                format!("Android {release} (Termux)")
            }
        }
        _ => {
            if let Some(pretty) = read_os_release_pretty_name() {
                pretty
            } else {
                format!("{} {}", os, env::consts::ARCH)
            }
        }
    }
}

fn read_os_release_pretty_name() -> Option<String> {
    let contents = fs::read_to_string("/etc/os-release").ok()?;
    for line in contents.lines() {
        if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
            return Some(value.trim().trim_matches('"').to_string());
        }
    }
    None
}

fn read_command_line(args: &[&str]) -> Option<String> {
    let (program, rest) = args.split_first()?;
    let output = std::process::Command::new(program)
        .args(rest)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

#[cfg(test)]
mod tests {
    use super::describe_install_strategy;

    #[test]
    fn macos_strategy_mentions_source_build() {
        let strategy = describe_install_strategy("macos");
        assert!(strategy.contains("source"));
        assert!(strategy.to_ascii_lowercase().contains("openssl"));
    }

    #[test]
    fn windows_strategy_is_prebuilt_not_source() {
        let strategy = describe_install_strategy("windows");
        let lower = strategy.to_ascii_lowercase();
        assert!(lower.contains("nuget") || lower.contains("embeddable"));
        assert!(lower.contains("no local cpython compile") || !lower.contains("source build"));
    }
}
