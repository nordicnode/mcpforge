use crate::common::{read_mcp_servers_from_json, write_mcp_servers_to_json};
use crate::traits::{ClientAdapter, ConfigLocation, TransportSupport};
use anyhow::Result;
use mcp_core::types::{Scope, ServerEntry};
use std::path::PathBuf;

#[derive(Default)]
pub struct PrimeAdapter;

impl PrimeAdapter {
    pub fn new() -> Self {
        Self
    }

    fn possible_paths() -> Vec<(PathBuf, Scope, &'static str)> {
        let mut paths = Vec::new();

        if let Some(home) = dirs::home_dir() {
            paths.push((
                home.join(".prime").join("agent").join("mcp.json"),
                Scope::Global,
                "Prime Agent (agent/mcp.json)",
            ));
            paths.push((
                home.join(".prime").join("config.json"),
                Scope::Global,
                "Prime Agent (config.json)",
            ));
        }

        paths.push((
            PathBuf::from(".prime").join("mcp.json"),
            Scope::Project,
            "Prime Agent (Project .prime/mcp.json)",
        ));
        paths.push((
            PathBuf::from("prime.json"),
            Scope::Project,
            "Prime Agent (Project prime.json)",
        ));

        paths
    }
}

impl ClientAdapter for PrimeAdapter {
    fn id(&self) -> &'static str {
        "prime"
    }

    fn display_name(&self) -> &'static str {
        "Prime Agent"
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
