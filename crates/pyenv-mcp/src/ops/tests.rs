// ./crates/pyenv-mcp/src/ops/tests.rs
//! Regression tests for MCP client-config generation, install docs, and venv-path helpers.

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tempfile::TempDir;

    use super::super::context::build_client_config;
    use super::super::docs::build_install_instructions;
    use super::super::project::venv_python_path;

    #[test]
    fn client_config_contains_pyenv_root() {
        let config = build_client_config(
            Path::new("/tmp/pyenv-mcp"),
            Path::new("/tmp/.pyenv"),
            "pyenv-native",
        );
        let server = config
            .mcp_servers
            .get("pyenv-native")
            .expect("server entry");
        assert_eq!(server.command, "/tmp/pyenv-mcp");
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
            "pyenv-native",
            Path::new("/tmp/pyenv-mcp"),
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
                .contains("pipx install pyenv-native")
        );
    }

    #[test]
    fn toolkit_guide_includes_lifecycle_tools() {
        let guide = super::super::docs::build_toolkit_guide(
            "imyourboyroy/pyenv-native",
            Some(Path::new("/tmp/.pyenv")),
            "pyenv-native",
            Path::new("/tmp/pyenv-mcp"),
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
        ] {
            assert!(names.contains(&expected), "missing tool {expected}");
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
