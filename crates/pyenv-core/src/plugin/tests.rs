// ./crates/pyenv-core/src/plugin/tests.rs
//! Regression coverage for plugin command discovery, hook parsing, and hook execution.

#[cfg(test)]
mod tests {
    use std::env;
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use crate::config::AppConfig;
    use crate::context::AppContext;

    use super::super::commands::{cmd_hooks, complete_plugin_command};
    use super::super::discovery::{
        discover_hook_scripts, discover_plugin_commands, find_plugin_command,
        hook_search_roots_with_extra, system_hook_roots,
    };
    use super::super::hooks::{parse_hook_actions, run_hook_scripts};

    fn test_path_ext() -> Option<OsString> {
        if cfg!(windows) {
            Some(OsString::from(".exe;.cmd;.bat"))
        } else {
            None
        }
    }

    fn write_plugin_script(path: &PathBuf, body: &str) {
        fs::write(path, body).expect("plugin");
    }

    fn test_context() -> (TempDir, AppContext) {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path().join(".pyenv");
        let dir = temp.path().join("work");
        let system_bin = temp.path().join("system-bin");
        fs::create_dir_all(root.join("plugins")).expect("plugins");
        fs::create_dir_all(&dir).expect("work");
        fs::create_dir_all(&system_bin).expect("system bin");

        let ctx = AppContext {
            root,
            dir,
            exe_path: std::path::PathBuf::from("pyenv"),
            env_version: None,
            env_shell: None,
            path_env: Some(env::join_paths([system_bin]).expect("path env")),
            path_ext: test_path_ext(),
            config: AppConfig::default(),
        };

        (temp, ctx)
    }

    #[test]
    fn finds_plugin_command_under_root_plugins() {
        let (_temp, ctx) = test_context();
        let plugin_bin = ctx.root.join("plugins").join("demo").join("bin");
        fs::create_dir_all(&plugin_bin).expect("plugin bin");
        let plugin_path = if cfg!(windows) {
            plugin_bin.join("pyenv-hello.cmd")
        } else {
            plugin_bin.join("pyenv-hello.sh")
        };
        write_plugin_script(
            &plugin_path,
            if cfg!(windows) {
                "@echo off\r\n"
            } else {
                "#!/usr/bin/env sh\n"
            },
        );

        let path = find_plugin_command(&ctx, "hello").expect("plugin path");
        assert_eq!(path, plugin_path);
    }

    #[test]
    fn plugin_commands_are_discovered_and_sorted() {
        let (_temp, ctx) = test_context();
        let plugin_bin = ctx.root.join("plugins").join("demo").join("bin");
        fs::create_dir_all(&plugin_bin).expect("plugin bin");
        if cfg!(windows) {
            fs::write(plugin_bin.join("pyenv-zeta.cmd"), "@echo off").expect("plugin");
            fs::write(plugin_bin.join("pyenv-alpha.ps1"), "Write-Output alpha").expect("plugin");
        } else {
            fs::write(plugin_bin.join("pyenv-zeta"), "#!/usr/bin/env sh\n").expect("plugin");
            fs::write(plugin_bin.join("pyenv-alpha.sh"), "#!/usr/bin/env sh\n").expect("plugin");
        }

        let commands = discover_plugin_commands(&ctx);
        assert_eq!(commands, vec!["alpha".to_string(), "zeta".to_string()]);
    }

    #[test]
    fn finds_plugin_commands_on_path_in_directories_with_spaces() {
        let (_temp, mut ctx) = test_context();
        let path_dir = ctx.root.join("path plugins");
        fs::create_dir_all(&path_dir).expect("path dir");
        let plugin_path = if cfg!(windows) {
            path_dir.join("pyenv-sh-hello.cmd")
        } else {
            path_dir.join("pyenv-sh-hello.sh")
        };
        write_plugin_script(
            &plugin_path,
            if cfg!(windows) {
                "@echo off\r\n"
            } else {
                "#!/usr/bin/env sh\n"
            },
        );
        let existing_path = ctx.path_env.clone().expect("path env");
        let mut joined = env::split_paths(&existing_path).collect::<Vec<_>>();
        joined.insert(0, path_dir.clone());
        ctx.path_env = Some(env::join_paths(joined).expect("join path"));

        let commands = discover_plugin_commands(&ctx);
        assert!(commands.iter().any(|command| command == "sh-hello"));

        let resolved = find_plugin_command(&ctx, "sh-hello").expect("plugin path");
        assert_eq!(resolved, plugin_path);
    }

    #[test]
    fn hooks_lists_sorted_supported_scripts() {
        let (_temp, ctx) = test_context();
        let hook_dir = ctx.root.join("pyenv.d").join("rehash");
        fs::create_dir_all(&hook_dir).expect("hook dir");
        fs::write(hook_dir.join("zeta.cmd"), "@echo off").expect("hook");
        fs::write(hook_dir.join("alpha.ps1"), "Write-Output alpha").expect("hook");
        fs::write(hook_dir.join("skip.txt"), "").expect("skip");

        let hooks = discover_hook_scripts(&ctx, "rehash").expect("hooks");
        assert_eq!(hooks.len(), 2);
        assert!(hooks[0].ends_with("alpha.ps1"));

        let report = cmd_hooks(&ctx, "rehash");
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.stdout.len(), 2);

        let completion = cmd_hooks(&ctx, "--complete");
        assert_eq!(completion.exit_code, 0);
        assert!(completion.stdout.iter().any(|line| line == "rehash"));
    }

    #[test]
    fn hook_actions_parse_supported_directives() {
        let actions = parse_hook_actions(&[
            "PATH+=C:\\tools".to_string(),
            "ENV:DEMO=value".to_string(),
            "PYENV_COMMAND_PATH=C:\\demo\\python.exe".to_string(),
            "python".to_string(),
        ]);

        assert_eq!(
            actions.command_path,
            Some(PathBuf::from("C:\\demo\\python.exe"))
        );
        assert_eq!(actions.prepend_paths, vec![PathBuf::from("C:\\tools")]);
        assert_eq!(
            actions.env_pairs,
            vec![("DEMO".to_string(), "value".to_string())]
        );
        assert_eq!(actions.passthrough_lines, vec!["python".to_string()]);
    }

    #[test]
    fn hook_actions_parse_shell_style_assignments() {
        let actions = parse_hook_actions(&[
            "export PYENV_VERSION=3.12.6".to_string(),
            "PYENV_VERSION_ORIGIN=\"hook-origin\"".to_string(),
            "PATH=/tmp/demo".to_string(),
        ]);

        assert_eq!(
            actions.env_pairs,
            vec![
                ("PYENV_VERSION".to_string(), "3.12.6".to_string()),
                (
                    "PYENV_VERSION_ORIGIN".to_string(),
                    "hook-origin".to_string()
                ),
                ("PATH".to_string(), "/tmp/demo".to_string()),
            ]
        );
    }

    #[test]
    fn run_hook_scripts_executes_cmd_and_collects_output() {
        let (_temp, ctx) = test_context();
        let hook_dir = ctx.root.join("pyenv.d").join("rehash");
        fs::create_dir_all(&hook_dir).expect("hook dir");
        let hook_name = if cfg!(windows) {
            let path = hook_dir.join("alpha.cmd");
            fs::write(&path, "@echo one\r\n@echo two").expect("hook");
            path
        } else {
            let path = hook_dir.join("alpha.sh");
            fs::write(&path, "#!/usr/bin/env sh\necho one\necho two\n").expect("hook");
            path
        };

        let results = run_hook_scripts(&ctx, "rehash", &[]).expect("results");
        assert_eq!(results.len(), 1);
        let expected_path = if cfg!(windows) {
            hook_name
        } else {
            fs::canonicalize(hook_name).expect("canonical hook")
        };
        assert_eq!(results[0].path, expected_path);
        assert_eq!(
            results[0].stdout,
            vec!["one".to_string(), "two".to_string()]
        );
    }

    #[test]
    fn plugin_completion_runs_complete_mode() {
        let (_temp, ctx) = test_context();
        let plugin_bin = ctx.root.join("plugins").join("demo").join("bin");
        fs::create_dir_all(&plugin_bin).expect("plugin bin");
        if cfg!(windows) {
            fs::write(
                plugin_bin.join("pyenv-hello.cmd"),
                "@if \"%~1\"==\"--complete\" (\r\n@echo world\r\n@echo friend\r\n@exit /b 0\r\n)\r\n@exit /b 0\r\n",
            )
            .expect("plugin");
        } else {
            fs::write(
                plugin_bin.join("pyenv-hello.sh"),
                "#!/usr/bin/env sh\nif [ \"$1\" = \"--complete\" ]; then\n  echo world\n  echo friend\n  exit 0\nfi\nexit 0\n",
            )
            .expect("plugin");
        }

        let completions = complete_plugin_command(&ctx, "hello", &[String::from("he")])
            .expect("completion")
            .expect("plugin completions");
        assert_eq!(completions, vec!["world".to_string(), "friend".to_string()]);
    }

    #[test]
    fn hook_search_path_keeps_custom_and_default_roots() {
        let (_temp, ctx) = test_context();
        let default_hook_dir = ctx.root.join("pyenv.d").join("rehash");
        let custom_root = ctx.root.join("custom-hooks");
        let custom_hook_dir = custom_root.join("rehash");
        fs::create_dir_all(&default_hook_dir).expect("default hook dir");
        fs::create_dir_all(&custom_hook_dir).expect("custom hook dir");
        fs::write(default_hook_dir.join("beta.cmd"), "@echo beta").expect("default hook");
        fs::write(custom_hook_dir.join("alpha.cmd"), "@echo alpha").expect("custom hook");

        let roots = hook_search_roots_with_extra(&ctx, Some(custom_root.clone().into_os_string()));
        assert_eq!(roots[0], custom_root);
        assert!(roots.iter().any(|path| path == &ctx.root.join("pyenv.d")));
    }

    #[test]
    fn system_hook_roots_match_upstream_posix_locations() {
        if cfg!(windows) {
            assert!(system_hook_roots().is_empty());
        } else {
            let roots = system_hook_roots();
            assert!(
                roots
                    .iter()
                    .any(|path| path == &PathBuf::from("/etc/pyenv.d"))
            );
            assert!(
                roots
                    .iter()
                    .any(|path| path == &PathBuf::from("/usr/lib/pyenv/hooks"))
            );
        }
    }
}
