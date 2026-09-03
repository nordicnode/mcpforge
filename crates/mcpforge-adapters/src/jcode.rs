use crate::common::{read_mcp_servers_from_json, write_mcp_servers_to_json};
use crate::traits::{ClientAdapter, ConfigLocation, TransportSupport};
use anyhow::Result;
use mcp_core::types::{Scope, ServerEntry};
use std::path::PathBuf;

#[derive(Default)]
pub struct JcodeAdapter;

impl JcodeAdapter {
    pub fn new() -> Self {
        Self
    }

    fn default_config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".jcode").join("servers.json"));
        }
        if let Some(config_dir) = dirs::config_dir() {
            paths.push(config_dir.join("jcode").join("servers.json"));
        }
        paths
    }
}

impl ClientAdapter for JcodeAdapter {
    fn id(&self) -> &'static str {
        "jcode"
    }

    fn display_name(&self) -> &'static str {
        "J-Code"
    }

    fn detect(&self) -> Vec<ConfigLocation> {
        let mut locs = Vec::new();
        for path in Self::default_config_paths() {
            let exists = path.exists();
            let label = if path.to_string_lossy().contains(".jcode") {
                "J-Code (Home)"
            } else {
                "J-Code (Config)"
            };
            locs.push(ConfigLocation {
                client_id: self.id().to_string(),
                display_name: label.to_string(),
                path,
                scope: Scope::Global,
                exists,
            });
        }
        locs
    }

    fn read_servers(&self, loc: &ConfigLocation) -> Result<Vec<ServerEntry>> {
        let mut servers = read_mcp_servers_from_json(&loc.path, "mcpServers", loc)?;
        if servers.is_empty() {
            // Check top level
            servers = read_mcp_servers_from_json(&loc.path, "", loc)?;
        }
        Ok(servers)
    }

    fn write_servers(&self, loc: &ConfigLocation, entries: &[ServerEntry]) -> Result<()> {
        write_mcp_servers_to_json(&loc.path, "mcpServers", self.id(), entries)
    }

    fn supports(&self) -> TransportSupport {
        TransportSupport::all()
    }

    fn backup_path(&self, _loc: &ConfigLocation) -> PathBuf {
        crate::backup::default_backup_dir().join(format!("{}.bak", self.id()))
    }
}
