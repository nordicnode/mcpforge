use crate::common::{read_mcp_servers_from_json, write_mcp_servers_to_json};
use crate::traits::{ClientAdapter, ConfigLocation, TransportSupport};
use anyhow::Result;
use mcp_core::types::{Scope, ServerEntry};
use std::path::PathBuf;

#[derive(Default)]
pub struct ZedAdapter;

impl ZedAdapter {
    pub fn new() -> Self {
        Self
    }

    fn default_config_path() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir().map(|h| {
                h.join("Library")
                    .join("Application Support")
                    .join("Zed")
                    .join("settings.json")
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            dirs::config_dir().map(|c| c.join("zed").join("settings.json"))
        }
    }
}

impl ClientAdapter for ZedAdapter {
    fn id(&self) -> &'static str {
        "zed"
    }

    fn display_name(&self) -> &'static str {
        "Zed"
    }

    fn detect(&self) -> Vec<ConfigLocation> {
        let mut locs = Vec::new();
        if let Some(path) = Self::default_config_path() {
            let exists = path.exists();
            locs.push(ConfigLocation {
                client_id: self.id().to_string(),
                display_name: self.display_name().to_string(),
                path,
                scope: Scope::Global,
                exists,
            });
        }
        locs
    }

    fn read_servers(&self, loc: &ConfigLocation) -> Result<Vec<ServerEntry>> {
        // Check context_servers first, then mcpServers
        let mut servers = read_mcp_servers_from_json(&loc.path, "context_servers", loc)?;
        if servers.is_empty() {
            servers = read_mcp_servers_from_json(&loc.path, "mcpServers", loc)?;
        }
        Ok(servers)
    }

    fn write_servers(&self, loc: &ConfigLocation, entries: &[ServerEntry]) -> Result<()> {
        write_mcp_servers_to_json(&loc.path, "context_servers", self.id(), entries)
    }

    fn supports(&self) -> TransportSupport {
        TransportSupport::stdio_only()
    }

    fn backup_path(&self, _loc: &ConfigLocation) -> PathBuf {
        crate::backup::default_backup_dir().join(format!("{}.bak", self.id()))
    }
}
