// ./crates/pyenv-core/src/upgrade/tests.rs
use super::*;
use tempfile::TempDir;

#[test]
fn plain_patch_line_accepts_only_release_triples() {
    assert_eq!(plain_patch_line("3.14.1").as_deref(), Some("3.14"));
    assert_eq!(plain_patch_line("2.7.18").as_deref(), Some("2.7"));
    for not_plain in [
        "3.14",
        "3.14.1.2",
        "3.15.0a1",
        "3.13.0t",
        "pypy3.11-7.3.19",
        "system",
        "3..1",
    ] {
        assert_eq!(plain_patch_line(not_plain), None, "{not_plain}");
    }
}

#[test]
fn skip_reason_explains_why_a_runtime_is_left_alone() {
    assert!(skip_reason("3.14.1").is_none());
    assert!(skip_reason("system").is_some());
    assert!(skip_reason("3.14.1/envs/app").is_some());
    assert!(skip_reason("pypy3.11-7.3.19").unwrap().contains("PyPy"));
    assert!(skip_reason("3.13.0t").unwrap().contains("free-threaded"));
    assert!(skip_reason("3.15.0rc1").unwrap().contains("pre-release"));
}

#[test]
fn targets_match_exact_names_and_minor_lines_only() {
    let none: Vec<String> = Vec::new();
    assert!(target_matches(&none, "3.14.1"));
    let line = vec!["3.14".to_string()];
    assert!(target_matches(&line, "3.14.1"));
    assert!(
        !target_matches(&line, "3.140.1"),
        "3.14 must not match 3.140.x"
    );
    assert!(!target_matches(&line, "3.13.7"));
    let exact = vec!["3.14.1".to_string()];
    assert!(target_matches(&exact, "3.14.1"));
    assert!(!target_matches(&exact, "3.14.10"));
}

#[test]
fn replace_pin_rewrites_exact_lines_and_keeps_the_rest() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("version");
    fs::write(&file, "3.14.1\n3.13.7\n# note\n").unwrap();
    assert!(replace_pin_in_file(&file, "3.14.1", "3.14.9"));
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "3.14.9\n3.13.7\n# note\n"
    );
    assert!(
        !replace_pin_in_file(&file, "3.14.1", "3.14.9"),
        "second run is a no-op"
    );
    assert!(!replace_pin_in_file(&dir.path().join("missing"), "a", "b"));
}

#[test]
fn text_report_lists_status_venvs_and_dry_run_hint() {
    let report = UpgradeReport {
        dry_run: true,
        items: vec![
            RuntimeUpgrade {
                installed: "3.14.1".into(),
                latest: Some("3.14.9".into()),
                status: UpgradeStatus::Available,
                message: String::new(),
                venvs: vec![],
                repinned: vec![],
                removed_old: false,
            },
            RuntimeUpgrade {
                installed: "3.13.7".into(),
                latest: Some("3.13.7".into()),
                status: UpgradeStatus::UpToDate,
                message: String::new(),
                venvs: vec![],
                repinned: vec![],
                removed_old: false,
            },
        ],
    };
    let text = render_text(&report).join("\n");
    assert!(text.contains("3.14.1  -> 3.14.9  available"));
    assert!(text.contains("3.13.7  up to date"));
    assert!(text.contains("1 runtime(s) can be upgraded"));
    assert!(!report.has_failures());
}

#[test]
fn partial_and_failed_items_are_failures() {
    let mut item = RuntimeUpgrade {
        installed: "3.14.1".into(),
        latest: Some("3.14.9".into()),
        status: UpgradeStatus::Partial,
        message: "x".into(),
        venvs: vec![],
        repinned: vec![],
        removed_old: false,
    };
    let mut report = |status| {
        item.status = status;
        UpgradeReport {
            dry_run: false,
            items: vec![item.clone()],
        }
    };
    assert!(report(UpgradeStatus::Partial).has_failures());
    assert!(report(UpgradeStatus::Failed).has_failures());
    assert!(!report(UpgradeStatus::Upgraded).has_failures());
    assert!(!report(UpgradeStatus::Skipped).has_failures());
}
