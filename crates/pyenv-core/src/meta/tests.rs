// ./crates/pyenv-core/src/meta/tests.rs
//! Regression coverage for command discovery, help rendering, shim listing, and completion
//! behavior in the `meta` command-surface helpers.

use std::env;
use std::ffi::OsString;
use std::fs;

use tempfile::TempDir;

use crate::config::AppConfig;
use crate::context::AppContext;

use super::{cmd_commands, cmd_completions, cmd_help, cmd_shims};

fn test_context() -> (TempDir, AppContext) {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path().join(".pyenv");
    let dir = temp.path().join("work");
    fs::create_dir_all(root.join("plugins")).expect("plugins");
    fs::create_dir_all(root.join("versions")).expect("versions");
    fs::create_dir_all(root.join("shims")).expect("shims");
    fs::create_dir_all(&dir).expect("work");

    let ctx = AppContext {
        root,
        dir,
        exe_path: std::path::PathBuf::from("pyenv"),
        env_version: None,
        env_shell: None,
        path_env: Some(OsString::from("C:\\Windows\\System32")),
        path_ext: Some(OsString::from(".EXE;.CMD;.BAT;.PS1")),
        config: AppConfig::default(),
    };

    (temp, ctx)
}

#[test]
fn commands_lists_core_and_plugin_commands() {
    let (_temp, ctx) = test_context();
    let plugin_bin = ctx.root.join("plugins").join("demo").join("bin");
    fs::create_dir_all(&plugin_bin).expect("plugin bin");
    fs::write(plugin_bin.join("pyenv-hello.cmd"), "@echo off").expect("plugin");

    let report = cmd_commands(&ctx, false, false);
    assert_eq!(report.exit_code, 0);
    assert!(report.stdout.iter().any(|line| line == "help"));
    assert!(report.stdout.iter().any(|line| line == "update"));
    assert!(report.stdout.iter().any(|line| line == "hello"));
    assert!(report.stdout.iter().all(|line| !line.starts_with("sh-")));
}

#[test]
fn commands_detect_path_plugins_in_directories_with_spaces() {
    let (_temp, mut ctx) = test_context();
    let path_dir = ctx.root.join("path plugins");
    fs::create_dir_all(&path_dir).expect("path dir");
    fs::write(path_dir.join("pyenv-sh-hello.cmd"), "@echo off").expect("plugin");
    let existing_path = ctx.path_env.clone().expect("path env");
    let mut joined = env::split_paths(&existing_path).collect::<Vec<_>>();
    joined.insert(0, path_dir);
    ctx.path_env = Some(env::join_paths(joined).expect("join path"));

    let report = cmd_commands(&ctx, true, false);
    assert_eq!(report.exit_code, 0);
    assert!(report.stdout.iter().any(|line| line == "hello"));
}

#[test]
fn help_prints_usage_and_summary() {
    let (_temp, ctx) = test_context();
    let report = cmd_help(&ctx, Some("install"), false);
    assert_eq!(report.exit_code, 0);
    assert_eq!(
        report.stdout.first().expect("usage"),
        "Usage: pyenv install [-l|--list] [--known] [--family <family>] [--dry-run] [--json] [-f|--force] <version> ..."
    );

    let usage_report = cmd_help(&ctx, None, true);
    assert_eq!(usage_report.stdout, vec!["Usage: pyenv <command> [<args>]"]);
}

#[test]
fn help_parses_external_plugin_doc_headers() {
    let (_temp, ctx) = test_context();
    let plugin_bin = ctx.root.join("plugins").join("demo").join("bin");
    fs::create_dir_all(&plugin_bin).expect("plugin bin");
    fs::write(
        plugin_bin.join("pyenv-hello"),
        "#!/usr/bin/env sh\n# Usage: pyenv hello <world>\n#        pyenv hi [everybody]\n# Summary: Says hello to you.\n# This is extended help.\n#\n# And paragraphs.\nexit 0\n",
    )
    .expect("plugin");

    let report = cmd_help(&ctx, Some("hello"), false);
    assert_eq!(report.exit_code, 0);
    assert_eq!(
        report.stdout,
        vec![
            "Usage: pyenv hello <world>\n       pyenv hi [everybody]".to_string(),
            String::new(),
            "This is extended help.".to_string(),
            String::new(),
            "And paragraphs.".to_string(),
        ]
    );
}

#[test]
fn help_falls_back_to_plugin_summary_without_extended_text() {
    let (_temp, ctx) = test_context();
    let plugin_bin = ctx.root.join("plugins").join("demo").join("bin");
    fs::create_dir_all(&plugin_bin).expect("plugin bin");
    fs::write(
        plugin_bin.join("pyenv-hello"),
        "#!/usr/bin/env sh\n# Usage: pyenv hello <world>\n# Summary: Says hello to you.\nexit 0\n",
    )
    .expect("plugin");

    let report = cmd_help(&ctx, Some("hello"), false);
    assert_eq!(
        report.stdout,
        vec![
            "Usage: pyenv hello <world>".to_string(),
            String::new(),
            "Says hello to you.".to_string(),
        ]
    );

    let usage_only = cmd_help(&ctx, Some("hello"), true);
    assert_eq!(
        usage_only.stdout,
        vec!["Usage: pyenv hello <world>".to_string()]
    );
}

#[test]
fn shims_prefers_primary_launcher_per_command() {
    let (_temp, ctx) = test_context();
    fs::write(ctx.shims_dir().join("python.exe"), "").expect("python exe");
    fs::write(ctx.shims_dir().join("python.cmd"), "").expect("python cmd");
    fs::write(ctx.shims_dir().join("pip.cmd"), "").expect("pip cmd");
    fs::write(ctx.shims_dir().join(".pyenv-shims.json"), "{}").expect("manifest");

    let short = cmd_shims(&ctx, true);
    assert_eq!(short.exit_code, 0);
    assert_eq!(short.stdout, vec!["pip".to_string(), "python".to_string()]);

    let full = cmd_shims(&ctx, false);
    assert!(full.stdout.iter().any(|line| line.ends_with("python.exe")));
    assert!(full.stdout.iter().any(|line| line.ends_with("pip.cmd")));
}

#[test]
fn completions_include_help_and_dynamic_values() {
    let (_temp, ctx) = test_context();
    fs::create_dir_all(ctx.versions_dir().join("3.12.6")).expect("version");

    let report = cmd_completions(&ctx, "global", &[]);
    assert_eq!(report.exit_code, 0);
    assert!(report.stdout.iter().any(|line| line == "--help"));
    assert!(report.stdout.iter().any(|line| line == "3.12.6"));
    assert!(report.stdout.iter().any(|line| line == "system"));

    let hooks = cmd_completions(&ctx, "hooks", &[]);
    assert_eq!(hooks.exit_code, 0);
    assert!(hooks.stdout.iter().any(|line| line == "install"));
    assert!(hooks.stdout.iter().any(|line| line == "rehash"));
}
