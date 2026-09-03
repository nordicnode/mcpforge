use anyhow::Result;
use clap::Parser;
use mcpforge::cli::{self, Cli};
use mcpforge::tui;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        return cli::handlers::execute(cmd).await;
    }

    // Launch interactive TUI command center
    tui::run(cli.doctor).await
}
