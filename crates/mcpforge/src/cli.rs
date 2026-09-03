use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "mcpforge",
    author,
    version,
    about = "Terminal UI and CLI for managing, discovering, and health-checking MCP servers"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Run doctor checks on all servers at startup
    #[arg(short, long)]
    pub doctor: bool,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// List all configured MCP servers across detected clients
    List {
        /// Filter by client id (e.g. 'cursor', 'claude-code', 'claude-desktop', 'vscode')
        #[arg(short, long)]
        client: Option<String>,

        /// Output results in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Run health checks on configured servers
    Doctor {
        /// Output health results in JSON format
        #[arg(long)]
        json: bool,

        /// Health check timeout in seconds
        #[arg(short, long, default_value_t = 5)]
        timeout: u64,
    },

    /// Add an MCP server to one or more clients
    Add {
        /// Identifier / name of the server to add
        name: Option<String>,

        /// Pull definition from curated registry
        #[arg(long)]
        from_registry: bool,

        /// Read JSON server snippet from stdin
        #[arg(long)]
        stdin: bool,

        /// Server command executable
        #[arg(short, long)]
        command: Option<String>,

        /// Arguments for the command
        #[arg(short, long, num_args = 0..)]
        args: Vec<String>,

        /// Target client IDs to install into (comma-separated, e.g. "cursor,claude-code")
        #[arg(short, long, value_delimiter = ',')]
        to: Vec<String>,
    },

    /// Sync configured servers between clients
    Sync {
        /// Destination client to sync servers into
        target: String,

        /// Source client to copy server configurations from
        #[arg(long)]
        from: String,
    },

    /// Export canonical server configuration to a portable JSON file
    Export {
        /// Output file path (defaults to stdout if omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Include decrypted environment variables and tokens
        #[arg(long)]
        include_secrets: bool,
    },

    /// Import server configurations from an exported JSON file
    Import {
        /// Path to the JSON export file
        input: PathBuf,

        /// Target client IDs to install into (comma-separated, e.g. "cursor,vscode")
        #[arg(short, long, value_delimiter = ',')]
        to: Option<Vec<String>>,
    },
}
