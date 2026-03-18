// ./crates/pyenv-mcp/src/main.rs
//! Stdio MCP server and companion utilities for agent-friendly pyenv-native workflows.

mod model;
mod ops;
mod service;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use rmcp::{ServiceExt, transport::stdio};

use crate::ops::{
    DEFAULT_GITHUB_REPO, DEFAULT_SERVER_NAME, build_client_config, build_context,
    build_toolkit_guide,
};
use crate::service::PyenvNativeMcpServer;

#[derive(Debug, Parser)]
#[command(
    name = "pyenv-mcp",
    version,
    about = "Agent-friendly MCP server for pyenv-native",
    long_about = "Agent-friendly MCP server for pyenv-native.\n\nMCP (Model Context Protocol) lets AI agents and MCP-capable IDEs interact with\npyenv-native through structured JSON tools instead of parsing shell output.\n\nRunning `pyenv-mcp` with no arguments starts the stdio MCP server.\nYour MCP client (VS Code, Cursor, Claude Desktop, etc.) launches this automatically.",
    after_help = "QUICK START:\n  1. Run:  pyenv-mcp print-config\n  2. Paste the JSON into your MCP client configuration\n  3. The MCP client will launch `pyenv-mcp` automatically\n\nFor agents, run `pyenv-mcp guide` to get a structured JSON onboarding blob.\nFull documentation: https://github.com/imyourboyroy/pyenv-native/blob/main/MCP.md"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "Start the stdio MCP server (default when no subcommand is given)")]
    Serve,
    #[command(about = "Print a ready-to-paste MCP client configuration JSON block")]
    PrintConfig {
        #[arg(long = "server-name", default_value = DEFAULT_SERVER_NAME)]
        server_name: String,
        #[arg(long = "pyenv-root")]
        pyenv_root: Option<PathBuf>,
        #[arg(long = "command")]
        command: Option<PathBuf>,
    },
    #[command(about = "Print a structured JSON toolkit guide for AI model onboarding")]
    Guide {
        #[arg(long = "github-repo", default_value = DEFAULT_GITHUB_REPO)]
        github_repo: String,
        #[arg(long = "install-root")]
        install_root: Option<PathBuf>,
        #[arg(long = "server-name", default_value = DEFAULT_SERVER_NAME)]
        server_name: String,
        #[arg(long = "mcp-command")]
        mcp_command: Option<PathBuf>,
        #[arg(long = "pyenv-root")]
        pyenv_root: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Serve) {
        Commands::Serve => {
            PyenvNativeMcpServer::new()
                .serve(stdio())
                .await?
                .waiting()
                .await?;
        }
        Commands::PrintConfig {
            server_name,
            pyenv_root,
            command,
        } => {
            let ctx = build_context(None)?;
            let pyenv_root = pyenv_root.unwrap_or_else(|| ctx.root.clone());
            let command = command.unwrap_or_else(|| {
                std::env::current_exe().unwrap_or_else(|_| PathBuf::from("pyenv-mcp"))
            });
            let config = build_client_config(&command, &pyenv_root, &server_name);
            println!("{}", serde_json::to_string_pretty(&config)?);
        }
        Commands::Guide {
            github_repo,
            install_root,
            server_name,
            mcp_command,
            pyenv_root,
        } => {
            let ctx = build_context(None)?;
            let mcp_command = mcp_command.unwrap_or_else(|| {
                std::env::current_exe().unwrap_or_else(|_| PathBuf::from("pyenv-mcp"))
            });
            let pyenv_root = pyenv_root.unwrap_or_else(|| ctx.root.clone());
            let guide = build_toolkit_guide(
                &github_repo,
                install_root.as_deref(),
                &server_name,
                &mcp_command,
                &pyenv_root,
            );
            println!("{}", serde_json::to_string_pretty(&guide)?);
        }
    }

    Ok(())
}
