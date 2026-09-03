use crate::common::{read_mcp_servers_from_json, write_mcp_servers_to_json};
use crate::traits::{ClientAdapter, ConfigLocation, TransportSupport};
use anyhow::Result;
use mcp_core::types::{Scope, ServerEntry};
use std::path::PathBuf;

#[derive(Default)]
pub struct ClineAdapter;

impl ClineAdapter {
    pub fn new() -> Self {
        Self
    }

    fn possible_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(config_dir) = dirs::config_dir() {
            // VS Code Cline
            paths.push(
                config_dir
                    .join("Code")
                    .join("User")
                    .join("globalStorage")
                    .join("saoudrizwan.claude-dev")
                    .join("settings")
                    .join("cline_mcp_settings.json"),
            );
            // Cursor Cline
            paths.push(
                config_dir
                    .join("Cursor")
                    .join("User")
                    .join("globalStorage")
                    .join("saoudrizwan.claude-dev")
                    .join("settings")
                    .join("cline_mcp_settings.json"),
            );
        }
        #[cfg(target_os = "macos")]
        {
            if let Some(home) = dirs::home_dir() {
                paths.push(
                    home.join("Library")
                        .join("Application Support")
                        .join("Code")
                        .join("User")
                        .join("globalStorage")
                        .join("saoudrizwan.claude-dev")
                        .join("settings")
                        .join("cline_mcp_settings.json"),
                );
            }
        }
        paths
    }
}

impl ClientAdapter for ClineAdapter {
    fn id(&self) -> &'static str {
        "cline"
    }

    fn display_name(&self) -> &'static str {
        "Cline"
    }

    fn detect(&self) -> Vec<ConfigLocation> {
        let mut locs = Vec::new();
        for path in Self::possible_paths() {
            let exists = path.exists();
            let label = if path.to_string_lossy().contains("Cursor") {
                "Cline (Cursor)"
            } else {
                "Cline (VS Code)"
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
