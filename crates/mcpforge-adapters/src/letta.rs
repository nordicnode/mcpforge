use crate::common::{read_mcp_servers_from_json, write_mcp_servers_to_json};
use crate::traits::{ClientAdapter, ConfigLocation, TransportSupport};
use anyhow::Result;
use mcp_core::types::{Scope, ServerEntry};
use std::path::PathBuf;

#[derive(Default)]
pub struct LettaAdapter;

impl LettaAdapter {
    pub fn new() -> Self {
        Self
    }

    fn possible_paths() -> Vec<(PathBuf, Scope, &'static str)> {
        let mut paths = Vec::new();

        if let Some(home) = dirs::home_dir() {
            paths.push((
                home.join(".letta").join("mcp.json"),
                Scope::Global,
                "Letta Agent (mcp.json)",
            ));
            paths.push((
                home.join(".letta").join("config.json"),
                Scope::Global,
                "Letta Agent (config.json)",
            ));
        }

        paths.push((
            PathBuf::from(".letta").join("mcp.json"),
            Scope::Project,
            "Letta Agent (Project .letta/mcp.json)",
        ));
        paths.push((
            PathBuf::from("letta.json"),
            Scope::Project,
            "Letta Agent (Project letta.json)",
        ));

        paths
    }
}

impl ClientAdapter for LettaAdapter {
    fn id(&self) -> &'static str {
        "letta"
    }

    fn display_name(&self) -> &'static str {
        "Letta Agent"
    }

    fn detect(&self) -> Vec<ConfigLocation> {
        let mut locs = Vec::new();
        for (path, scope, label) in Self::possible_paths() {
            let exists = path.exists();
            locs.push(ConfigLocation {
                client_id: self.id().to_string(),
                display_name: label.to_string(),
                path,
                scope,
                exists,
            });
        }
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
