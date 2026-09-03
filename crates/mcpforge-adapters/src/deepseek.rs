use crate::common::{read_mcp_servers_from_json, write_mcp_servers_to_json};
use crate::traits::{ClientAdapter, ConfigLocation, TransportSupport};
use anyhow::Result;
use mcp_core::types::{Scope, ServerEntry};
use std::path::PathBuf;

#[derive(Default)]
pub struct DeepSeekAdapter;

impl DeepSeekAdapter {
    pub fn new() -> Self {
        Self
    }

    fn possible_paths() -> Vec<(PathBuf, Scope, &'static str)> {
        let mut paths = Vec::new();

        if let Some(home) = dirs::home_dir() {
            paths.push((
                home.join(".deepseek").join("config.json"),
                Scope::Global,
                "DeepSeek Harness (Home Config)",
            ));
            paths.push((
                home.join(".config").join("deepseek").join("mcp.json"),
                Scope::Global,
                "DeepSeek Harness (Config Dir)",
            ));
        }

        paths.push((
            PathBuf::from(".deepseek").join("mcp.json"),
            Scope::Project,
            "DeepSeek (Project .deepseek)",
        ));
        paths.push((
            PathBuf::from("deepseek.json"),
            Scope::Project,
            "DeepSeek (Project deepseek.json)",
        ));

        paths
    }
}

impl ClientAdapter for DeepSeekAdapter {
    fn id(&self) -> &'static str {
        "deepseek"
    }

    fn display_name(&self) -> &'static str {
        "DeepSeek Harness"
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
