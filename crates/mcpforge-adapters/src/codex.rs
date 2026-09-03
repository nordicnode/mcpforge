use crate::common::{read_mcp_servers_from_json, write_mcp_servers_to_json};
use crate::traits::{ClientAdapter, ConfigLocation, TransportSupport};
use anyhow::Result;
use mcp_core::types::{Scope, ServerEntry};
use std::path::PathBuf;

#[derive(Default)]
pub struct CodexAdapter;

impl CodexAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl ClientAdapter for CodexAdapter {
    fn id(&self) -> &'static str {
        "codex"
    }

    fn display_name(&self) -> &'static str {
        "Codex"
    }

    fn detect(&self) -> Vec<ConfigLocation> {
        let mut locs = Vec::new();

        if let Some(home) = dirs::home_dir() {
            let path = home.join(".codex").join("config.json");
            let exists = path.exists();
            locs.push(ConfigLocation {
                client_id: self.id().to_string(),
                display_name: format!("{} (Home)", self.display_name()),
                path,
                scope: Scope::Global,
                exists,
            });
        }

        if let Some(config_dir) = dirs::config_dir() {
            let path = config_dir.join("codex").join("mcp.json");
            let exists = path.exists();
            locs.push(ConfigLocation {
                client_id: self.id().to_string(),
                display_name: format!("{} (Config)", self.display_name()),
                path,
                scope: Scope::Global,
                exists,
            });
        }

        let project_path = PathBuf::from(".codex").join("mcp.json");
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
