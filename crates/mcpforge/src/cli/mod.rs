pub mod handlers;

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

    /// Remove an MCP server from one or more clients (or all clients)
    Remove {
        /// Identifier / name of the server to remove
        server: String,

        /// Specific target client IDs to remove from (comma-separated). Defaults to all clients containing this server
        #[arg(short, long, value_delimiter = ',')]
        from: Option<Vec<String>>,

        /// Remove from all clients without prompting
        #[arg(short, long)]
        all: bool,
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

    /// Audit and verify client configuration schemas for drift, syntax errors, and corruption
    Verify {
        /// Target specific client adapter only (e.g. "claude-code", "codex", "freebuff")
        #[arg(short, long)]
        client: Option<String>,

        /// Audit all 27 client adapters including uninstalled baselines
        #[arg(short, long)]
        all: bool,

        /// Run deep matrix cross-compatibility verification across all 27 adapters and 110 catalog servers
        #[arg(short, long)]
        matrix: bool,

        /// Output verification results as JSON
        #[arg(long)]
        json: bool,
    },

    /// Import server configurations from an exported JSON file
    Import {
        /// Path to the JSON export file
        input: PathBuf,

        /// Target client IDs to install into (comma-separated, e.g. "cursor,vscode")
        #[arg(short, long, value_delimiter = ',')]
        to: Option<Vec<String>>,
    },

    /// Test an MCP server or direct command with live handshake and latency diagnostics
    Test {
        /// Server identifier from configured servers or curated registry
        server: Option<String>,

        /// Direct command executable to test (e.g. 'npx', 'uvx')
        #[arg(short, long)]
        command: Option<String>,

        /// Arguments for the direct command
        #[arg(short, long, num_args = 0..)]
        args: Vec<String>,

        /// Timeout in seconds
        #[arg(short, long, default_value_t = 5)]
        timeout: u64,
    },

    /// Roll back client configurations to their previous backup snapshot
    Rollback {
        /// Specific client adapter to roll back (defaults to the most recently modified client)
        #[arg(short, long)]
        client: Option<String>,
    },

    /// Inspect, compare, or restore configuration backups
    Backup {
        #[command(subcommand)]
        command: BackupCommands,
    },

    /// List all tools exposed by an MCP server with schemas and descriptions
    Tools {
        /// Server identifier from configured servers or curated registry
        server: String,

        /// Output results as JSON
        #[arg(long)]
        json: bool,

        /// Timeout in seconds
        #[arg(short, long, default_value_t = 10)]
        timeout: u64,
    },

    /// Execute a tool call on a server with JSON parameters
    Call {
        /// Server identifier
        server: String,

        /// Tool name to invoke
        tool: String,

        /// Arguments as JSON string (e.g. '{"path": "/tmp"}')
        #[arg(default_value = "{}")]
        params: String,

        /// Output raw JSON result
        #[arg(long)]
        json: bool,

        /// Timeout in seconds
        #[arg(short, long, default_value_t = 30)]
        timeout: u64,
    },

    /// Generate shell autocompletion scripts for bash, zsh, fish, or powershell
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Run the configuration safeguard daemon to detect external edits and prevent corruption
    Watch {
        /// Automatically replicate newly added servers across active clients
        #[arg(long)]
        sync: bool,

        /// Polling interval in seconds
        #[arg(short, long, default_value_t = 2)]
        interval: u64,
    },
}

#[derive(Debug, Subcommand)]
pub enum BackupCommands {
    /// List all available configuration backups
    List {
        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },

    /// View diff between a backup and the current configuration file
    Diff {
        /// Client ID or path to backup file
        target: String,
    },

    /// Restore a specific backup file
    Restore {
        /// Path to the backup file
        backup_file: PathBuf,

        /// Target configuration path to restore to
        #[arg(short, long)]
        target: Option<PathBuf>,
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
