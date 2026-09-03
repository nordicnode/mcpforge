use crate::common::{read_mcp_servers_from_json, write_mcp_servers_to_json};
use crate::traits::{ClientAdapter, ConfigLocation, TransportSupport};
use anyhow::Result;
use mcp_core::types::{Scope, ServerEntry};
use std::path::PathBuf;

#[derive(Default)]
pub struct RooCodeAdapter;

impl RooCodeAdapter {
    pub fn new() -> Self {
        Self
    }

    fn possible_paths() -> Vec<(PathBuf, Scope, &'static str)> {
        let mut paths = Vec::new();
        if let Some(config_dir) = dirs::config_dir() {
            paths.push((
                config_dir
                    .join("Code")
                    .join("User")
                    .join("globalStorage")
                    .join("rooveterinaryinc.roo-cline")
                    .join("settings")
                    .join("cline_mcp_settings.json"),
                Scope::Global,
                "Roo Code (VS Code)",
            ));
            paths.push((
                config_dir
                    .join("Cursor")
                    .join("User")
                    .join("globalStorage")
                    .join("rooveterinaryinc.roo-cline")
                    .join("settings")
                    .join("cline_mcp_settings.json"),
                Scope::Global,
                "Roo Code (Cursor)",
            ));
        }
        paths.push((
            PathBuf::from(".roo").join("mcp.json"),
            Scope::Project,
            "Roo Code (Project)",
        ));
        paths
    }
}

impl ClientAdapter for RooCodeAdapter {
    fn id(&self) -> &'static str {
        "roo-code"
    }

    fn display_name(&self) -> &'static str {
        "Roo Code"
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
