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
    /// Auto-discover all installed AI clients, active processes, and configs on this machine
    Discover {
        /// Output discovery results as JSON
        #[arg(long)]
        json: bool,
    },

    /// List all configured MCP servers across detected clients
    List {
        /// Filter by client id (e.g. 'cursor', 'claude-code', 'claude-desktop', 'vscode', 'antigravity')
        #[arg(short, long)]
        client: Option<String>,

        /// Output results in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Automated one-command setup: auto-resolves tokens, checks runtimes, installs to all active clients, and verifies health
    Setup {
        /// Server identifier from curated registry (e.g. 'github', 'filesystem', 'postgres', 'brave-search')
        server: String,

        /// Optional target client IDs (comma-separated). Defaults to ALL detected clients
        #[arg(short, long, value_delimiter = ',')]
        to: Option<Vec<String>>,
    },

    /// Manage and install curated multi-server packs
    Pack {
        #[command(subcommand)]
        command: PackCommands,
    },

    /// Run health checks on configured servers (with optional auto-fix)
    Doctor {
        /// Automatically attempt self-healing on broken configurations
        #[arg(long)]
        fix: bool,

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

    /// Sync configured servers between clients (or auto-sync across all clients)
    Sync {
        /// Automatically synchronize all servers across every detected client on the machine
        #[arg(long)]
        auto: bool,

        /// Destination client to sync servers into (when not using --auto)
        target: Option<String>,

        /// Source client to copy server configurations from
        #[arg(long)]
        from: Option<String>,
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

#[derive(Debug, Subcommand)]
pub enum PackCommands {
    /// List available server packs
    List,

    /// Install a server pack across all detected clients
    Install {
        /// Pack ID (e.g. 'dev-core', 'data', 'web-research', 'cloud-dev')
        name: String,

        /// Optional target clients (comma-separated). Defaults to ALL detected clients
        #[arg(short, long, value_delimiter = ',')]
        to: Option<Vec<String>>,
    },
}
