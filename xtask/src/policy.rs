// ./xtask/src/policy.rs
//! `cargo xtask policy`: repository policy checks that used to be `scripts/check-*.sh`.
//!
//! * `eol`  — no tracked shell script contains CRLF line endings
//! * `gui`  — guardrails for the GUI's JavaScript/CSS/HTML patterns
//! * `i18n` — Fluent catalogs agree with `en-US`, and overlay JSON is in sync with the catalogs
//!
//! Pure Rust, so the checks run identically on Windows, macOS and Linux without `sh`, `grep` or `rg`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::{Args, ValueEnum};

use crate::repo_root;

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Which {
    Eol,
    Gui,
    I18n,
}

#[derive(Args)]
pub struct PolicyArgs {
    /// Run only these checks (default: all).
    #[arg(value_enum)]
    only: Vec<Which>,
    /// Fail instead of skipping when Python is missing for the overlay-sync step of `i18n`.
    #[arg(long)]
    strict: bool,
}

pub fn run(args: &PolicyArgs) -> Result<()> {
    run_in(&repo_root(), &args.only, args.strict)
}

pub fn run_in(root: &Path, only: &[Which], strict: bool) -> Result<()> {
    let wants = |which: Which| only.is_empty() || only.contains(&which);
    if wants(Which::Eol) {
        check_shell_eol(root)?;
    }
    if wants(Which::Gui) {
        check_gui_patterns(root)?;
    }
    if wants(Which::I18n) {
        check_i18n(root, strict)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// eol
// ---------------------------------------------------------------------------------------------

const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules"];

fn walk(dir: &Path, visit: &mut dyn FnMut(&Path)) -> Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if !SKIP_DIRS.contains(&name) {
                walk(&path, visit)?;
            }
        } else {
            visit(&path);
        }
    }
    Ok(())
}

fn check_shell_eol(root: &Path) -> Result<()> {
    let mut offenders = Vec::new();
    let mut scanned = 0usize;
    walk(root, &mut |path| {
        if path.extension().and_then(|ext| ext.to_str()) != Some("sh") {
            return;
        }
        scanned += 1;
        if fs::read(path)
            .map(|bytes| bytes.contains(&b'\r'))
            .unwrap_or(false)
        {
            offenders.push(
                path.strip_prefix(root)
                    .unwrap_or(path)
                    .display()
                    .to_string(),
            );
        }
    })?;
    if !offenders.is_empty() {
        bail!(
            "shell scripts must use LF line endings; CRLF found in:\n  {}",
            offenders.join("\n  ")
        );
    }
    println!("All {scanned} shell scripts use LF line endings.");
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// gui
// ---------------------------------------------------------------------------------------------

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

fn css_files(ui: &Path) -> Result<Vec<PathBuf>> {
    let mut files = vec![ui.join("styles.css")];
    let mut partials: Vec<PathBuf> = fs::read_dir(ui.join("styles"))
        .with_context(|| "reading ui/styles")?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("css"))
        .collect();
    partials.sort();
    files.extend(partials);
    Ok(files)
}

/// `.innerHTML` followed by optional whitespace and a single `=` (assignment, not `==`/`===`).
fn assigns_inner_html(source: &str) -> bool {
    let mut rest = source;
    while let Some(index) = rest.find(".innerHTML") {
        let after = rest[index + ".innerHTML".len()..].trim_start();
        if let Some(tail) = after.strip_prefix('=')
            && !tail.starts_with('=')
        {
            return true;
        }
        rest = &rest[index + ".innerHTML".len()..];
    }
    false
}

pub fn gui_violations(app_js: &str, dom_utils: &str, index_html: &str, css: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if app_js.contains("onclick=") {
        problems.push("inline onclick handlers are not allowed in app.js".to_string());
    }
    if assigns_inner_html(app_js) {
        problems.push(
            "innerHTML assignments are not allowed in app.js (use DomUtils helpers)".to_string(),
        );
    }
    if !index_html.contains("dom-utils.js") {
        problems.push("index.html must load dom-utils.js before app.js".to_string());
    }
    if !dom_utils.contains("createElement") {
        problems.push("dom-utils.js must provide safe DOM helpers".to_string());
    }
    let everything = [index_html, css, app_js];
    let remote_font = everything.iter().any(|text| {
        text.contains("JetBrains Mono")
            || text.contains("fonts.googleapis.com")
            || text.contains("fonts.gstatic.com")
    });
    if remote_font {
        problems.push(
            "remote or unvendored webfonts are not allowed; use local Inter and --font-mono"
                .to_string(),
        );
    }
    for (file, message) in [
        (
            "fonts/InterVariable.woff2",
            "styles.css or styles/*.css must load the vendored Inter variable font",
        ),
        (
            "fonts/NotoSansSC-400.woff2",
            "CJK Noto Sans SC must be vendored and loaded locally",
        ),
        (
            "fonts/NotoSansJP-400.woff2",
            "Japanese Noto Sans JP must be vendored and loaded locally",
        ),
        (
            "fonts/NotoSansKR-400.woff2",
            "Korean Noto Sans KR must be vendored and loaded locally",
        ),
        (
            "fonts/NotoNaskhArabic-400.woff2",
            "Arabic Noto Naskh must be vendored and loaded locally",
        ),
        (
            "fonts/NotoSansDevanagari-400.woff2",
            "Devanagari Noto Sans must be vendored and loaded locally",
        ),
    ] {
        if !css.contains(file) {
            problems.push(message.to_string());
        }
    }
    if !index_html.contains("i18n.js") {
        problems.push("index.html must load i18n.js".to_string());
    }
    problems
}

fn check_gui_patterns(root: &Path) -> Result<()> {
    let ui = root.join("crates").join("pyenv-gui").join("ui");
    let app_js = read(&ui.join("app.js"))?;
    let dom_utils = read(&ui.join("dom-utils.js"))?;
    let index_html = read(&ui.join("index.html"))?;
    let mut css = String::new();
    for file in css_files(&ui)? {
        css.push_str(&read(&file)?);
        css.push('\n');
    }
    let problems = gui_violations(&app_js, &dom_utils, &index_html, &css);
    if !problems.is_empty() {
        bail!("GUI pattern check failed:\n  {}", problems.join("\n  "));
    }
    println!("GUI pattern checks passed.");
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// i18n
// ---------------------------------------------------------------------------------------------

const MIN_LOCALES: usize = 14;
const UNSUPPORTED_CHINESE: &[&str] = &["zh-TW", "zh-Hant", "zh-HK", "zh-MO"];

/// Message ids in a Fluent file: lines that start with `[A-Za-z0-9_-]+` then optional blanks and `=`.
pub fn message_ids(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let end = line
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
                .unwrap_or(line.len());
            if end == 0 {
                return None;
            }
            let rest = line[end..].trim_start_matches([' ', '\t']);
            rest.starts_with('=').then(|| line[..end].to_string())
        })
        .collect()
}

fn ftl_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("ftl"))
        .collect();
    files.sort();
    Ok(files)
}

fn python() -> Option<&'static str> {
    ["python3", "python"].into_iter().find(|candidate| {
        Command::new(candidate)
            .arg("--version")
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    })
}

fn check_i18n(root: &Path, strict: bool) -> Result<()> {
    let locales = root.join("locales");
    let en_dir = locales.join("en-US");
    if !en_dir.is_dir() {
        bail!("i18n check failed: missing {}", en_dir.display());
    }
    let en_files = ftl_files(&en_dir)?;
    let mut en_all = BTreeSet::new();
    for file in &en_files {
        en_all.extend(message_ids(&read(file)?));
    }

    let mut locale_count = 0usize;
    let mut dirs: Vec<PathBuf> = fs::read_dir(&locales)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    for dir in dirs {
        let tag = dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string();
        locale_count += 1;
        if UNSUPPORTED_CHINESE.contains(&tag.as_str()) {
            bail!(
                "i18n check failed: Traditional Chinese catalog {tag} is not shipped; map those tags to zh-CN"
            );
        }
        for en_file in &en_files {
            let name = en_file.file_name().expect("catalog has a file name");
            let target = dir.join(name);
            let display = format!("{tag}/{}", name.to_string_lossy());
            if !target.is_file() {
                bail!(
                    "i18n check failed: {tag} is missing {}",
                    name.to_string_lossy()
                );
            }
            let ids = message_ids(&read(&target)?);
            let mut seen = BTreeSet::new();
            let duplicates: Vec<&String> =
                ids.iter().filter(|id| !seen.insert(id.as_str())).collect();
            if !duplicates.is_empty() {
                bail!(
                    "i18n check failed: {display} duplicate ids:\n  {}",
                    join(&duplicates)
                );
            }
            let expected: BTreeSet<String> = message_ids(&read(en_file)?).into_iter().collect();
            let actual: BTreeSet<String> = ids.into_iter().collect();
            let missing: Vec<&String> = expected.difference(&actual).collect();
            let extra: Vec<&String> = actual.difference(&expected).collect();
            if !missing.is_empty() {
                bail!(
                    "i18n check failed: {display} missing ids:\n  {}",
                    join(&missing)
                );
            }
            if !extra.is_empty() {
                bail!(
                    "i18n check failed: {display} extra ids:\n  {}",
                    join(&extra)
                );
            }
        }
    }
    if locale_count < MIN_LOCALES {
        bail!(
            "i18n check failed: expected {MIN_LOCALES} locale directories (en-US + {} translations); found {locale_count}",
            MIN_LOCALES - 1
        );
    }

    // Overlay JSON ↔ catalog sync is still generated by the Python seeder.
    match python() {
        Some(py) => {
            let status = Command::new(py)
                .arg(root.join("scripts").join("i18n").join("seed_locales.py"))
                .arg("--check")
                .current_dir(root)
                .status()?;
            if !status.success() {
                bail!("i18n check failed: seed_locales.py --check reported drift");
            }
        }
        None if strict => {
            bail!("python is required in --strict mode (scripts/i18n/seed_locales.py --check)")
        }
        None => println!("skipping: python not found (scripts/i18n/seed_locales.py --check)"),
    }

    println!(
        "i18n checks passed ({locale_count} locales, {} message ids).",
        en_all.len()
    );
    Ok(())
}

fn join(items: &[&String]) -> String {
    items
        .iter()
        .map(|item| item.as_str())
        .collect::<Vec<_>>()
        .join("\n  ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_fluent_message_ids() {
        let source = "# comment\ncli-about = Hello\n  indented = no\nkey-with_dash =Value\nbad line = x\n\tother = tab\n";
        assert_eq!(message_ids(source), vec!["cli-about", "key-with_dash"]);
    }

    #[test]
    fn inner_html_assignment_detection() {
        assert!(assigns_inner_html("el.innerHTML = '<b>'"));
        assert!(assigns_inner_html("el.innerHTML='x'"));
        assert!(!assigns_inner_html("if (el.innerHTML == '') {}"));
        assert!(!assigns_inner_html("if (el.innerHTML === '') {}"));
        assert!(!assigns_inner_html("const html = el.innerHTML;"));
    }

    fn ok_inputs() -> (String, String, String, String) {
        let css = "fonts/InterVariable.woff2 fonts/NotoSansSC-400.woff2 fonts/NotoSansJP-400.woff2 \
                   fonts/NotoSansKR-400.woff2 fonts/NotoNaskhArabic-400.woff2 fonts/NotoSansDevanagari-400.woff2";
        (
            "console.log(1);".into(),
            "document.createElement('div')".into(),
            "<script src=\"dom-utils.js\"></script><script src=\"i18n.js\"></script>".into(),
            css.into(),
        )
    }

    #[test]
    fn clean_gui_sources_pass() {
        let (app, dom, html, css) = ok_inputs();
        assert!(gui_violations(&app, &dom, &html, &css).is_empty());
    }

    #[test]
    fn gui_violations_are_reported() {
        let (_, dom, html, css) = ok_inputs();
        let bad_app = "<a onclick=\"x()\"> el.innerHTML = 'x'";
        let problems = gui_violations(bad_app, &dom, &html, &css);
        assert_eq!(problems.len(), 2);
        let problems = gui_violations("", &dom, &html, "@import url(https://fonts.googleapis.com)");
        assert!(problems.iter().any(|p| p.contains("remote or unvendored")));
        assert!(problems.iter().any(|p| p.contains("vendored Inter")));
    }

    #[test]
    fn repository_passes_its_own_policy() {
        // Everything except the Python overlay sync (covered by `cargo xtask check`).
        let root = repo_root();
        check_shell_eol(&root).unwrap();
        check_gui_patterns(&root).unwrap();
        check_i18n(&root, false).unwrap();
    }
}
