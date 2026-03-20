// ./crates/pyenv-mcp/src/ops/context.rs
//! App-context and MCP-client-config helpers shared across MCP operations.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

use pyenv_core::{AppContext, resolve_dir};

use crate::model::{McpClientConfig, McpServerEntry};

pub const DEFAULT_GITHUB_REPO: &str = "imyourboyroy/pyenv-native";
pub const DEFAULT_SERVER_NAME: &str = "pyenv-native";

pub fn build_context(project_dir: Option<PathBuf>) -> Result<AppContext> {
    let mut ctx = AppContext::from_system().map_err(|error| anyhow!(error.to_string()))?;
    if let Some(dir) = project_dir {
        let resolved =
            resolve_dir(Some(OsString::from(dir))).map_err(|error| anyhow!(error.to_string()))?;
        ctx.dir = resolved;
    }
    Ok(ctx)
}

pub fn build_client_config(
    command_path: &Path,
    pyenv_root: &Path,
    server_name: &str,
) -> McpClientConfig {
    let mut env = BTreeMap::new();
    env.insert("PYENV_ROOT".to_string(), pyenv_root.display().to_string());

    let mut servers = BTreeMap::new();
    servers.insert(
        server_name.to_string(),
        McpServerEntry {
            command: command_path.display().to_string(),
            args: Vec::new(),
            env,
        },
    );

    McpClientConfig {
        mcp_servers: servers,
    }
}
