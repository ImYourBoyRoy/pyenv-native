// ./crates/enchi-mcp/src/ops/tests.rs
//! Regression tests for MCP client-config generation, install docs, and venv-path helpers.

#[cfg(test)]
mod cases {
    use std::path::Path;

    use tempfile::TempDir;

    use super::super::context::build_client_config;
    use super::super::docs::build_install_instructions;
    use super::super::project::venv_python_path;

    #[test]
    fn client_config_contains_enchi_root() {
        let config = build_client_config(
            Path::new("/tmp/enchi-mcp"),
            Path::new("/tmp/.pyenv"),
            "enchi",
        );
        let server = config.mcp_servers.get("enchi").expect("server entry");
        assert_eq!(server.command, "/tmp/enchi-mcp");
        assert_eq!(
            server.env.get("PYENV_ROOT").expect("PYENV_ROOT"),
            "/tmp/.pyenv"
        );
    }

    #[test]
    fn install_instructions_reference_repo() {
        let instructions = build_install_instructions(
            "imyourboyroy/pyenv-native",
            Some(Path::new("/tmp/.pyenv")),
            "enchi",
            Path::new("/tmp/enchi-mcp"),
            Path::new("/tmp/.pyenv"),
        );
        assert!(
            instructions
                .latest_release
                .linux_or_macos
                .contains("raw.githubusercontent.com/imyourboyroy/pyenv-native")
        );
        assert!(
            instructions
                .pip_bootstrap
                .pipx
                .contains("pipx install enchi")
        );
    }

    #[test]
    fn toolkit_guide_includes_lifecycle_tools() {
        let guide = super::super::docs::build_toolkit_guide(
            "imyourboyroy/pyenv-native",
            Some(Path::new("/tmp/.pyenv")),
            "enchi",
            Path::new("/tmp/enchi-mcp"),
            Path::new("/tmp/.pyenv"),
        );
        let names: Vec<&str> = guide
            .tool_summaries
            .iter()
            .map(|summary| summary.tool_name.as_str())
            .collect();
        for expected in [
            "get_config",
            "set_config",
            "self_update",
            "venv_upgrade",
            "doctor_fix",
            "list_venvs",
            "which",
            "uninstall_runtime",
            "venv_delete",
            "venv_rename",
        ] {
            assert!(names.contains(&expected), "missing tool {expected}");
        }
    }

    #[test]
    fn toolkit_guide_documents_every_registered_tool() {
        let guide = crate::ops::build_toolkit_guide(
            "imyourboyroy/pyenv-native",
            Some(Path::new("/tmp/.enchi")),
            "enchi",
            Path::new("/tmp/enchi-mcp"),
            Path::new("/tmp/.enchi"),
        );
        let documented: Vec<&str> = guide
            .tool_summaries
            .iter()
            .map(|summary| summary.tool_name.as_str())
            .collect();
        for name in crate::service::EnchiMcpServer::registered_tool_names() {
            assert!(
                documented.contains(&name.as_str()),
                "tool `{name}` is registered but missing from get_toolkit_guide"
            );
        }
    }

    #[test]
    fn venv_python_path_uses_platform_layout() {
        let temp = TempDir::new().expect("tempdir");
        let path = venv_python_path(temp.path());
        if cfg!(windows) {
            assert!(path.ends_with("Scripts\\python.exe"));
        } else {
            assert!(path.ends_with("bin/python"));
        }
    }
}
