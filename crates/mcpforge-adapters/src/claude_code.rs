use crate::common::{read_mcp_servers_from_json, write_mcp_servers_to_json};
use crate::traits::{ClientAdapter, ConfigLocation, TransportSupport};
use anyhow::Result;
use mcp_core::types::{Scope, ServerEntry};
use std::path::PathBuf;

#[derive(Default)]
pub struct ClaudeCodeAdapter;

impl ClaudeCodeAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl ClientAdapter for ClaudeCodeAdapter {
    fn id(&self) -> &'static str {
        "claude-code"
    }

    fn display_name(&self) -> &'static str {
        "Claude Code"
    }

    fn detect(&self) -> Vec<ConfigLocation> {
        let mut locs = Vec::new();

        // 1. Global ~/.claude.json
        if let Some(home) = dirs::home_dir() {
            let global_path = home.join(".claude.json");
            let exists = global_path.exists();
            locs.push(ConfigLocation {
                client_id: self.id().to_string(),
                display_name: format!("{} (Global)", self.display_name()),
                path: global_path,
                scope: Scope::Global,
                exists,
            });
        }

        // 2. Project ./.mcp.json
        let project_path = PathBuf::from(".mcp.json");
        let exists = project_path.exists();
        locs.push(ConfigLocation {
            client_id: self.id().to_string(),
            display_name: format!("{} (Project)", self.display_name()),
            path: project_path,
            scope: Scope::Project,
            exists,
        });

        locs
    }

    fn read_servers(&self, loc: &ConfigLocation) -> Result<Vec<ServerEntry>> {
        read_mcp_servers_from_json(&loc.path, "mcpServers", loc)
    }

    fn write_servers(&self, loc: &ConfigLocation, entries: &[ServerEntry]) -> Result<()> {
        write_mcp_servers_to_json(&loc.path, "mcpServers", self.id(), entries)
    }

    fn supports(&self) -> TransportSupport {
        TransportSupport::stdio_and_http()
    }

    fn backup_path(&self, _loc: &ConfigLocation) -> PathBuf {
        crate::backup::default_backup_dir().join(format!("{}.bak", self.id()))
    }
}
