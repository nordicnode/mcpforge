use crate::common::{read_mcp_servers_from_json, write_mcp_servers_to_json};
use crate::traits::{ClientAdapter, ConfigLocation, TransportSupport};
use anyhow::Result;
use mcp_core::types::{Scope, ServerEntry};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct CustomHarnessConfigFile {
    #[serde(default)]
    pub clients: Vec<CustomClientDef>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CustomClientDef {
    pub id: String,
    pub display_name: String,
    pub path: String,
    #[serde(default = "default_key")]
    pub servers_key: String,
}

fn default_key() -> String {
    "mcpServers".to_string()
}

pub struct CustomHarnessAdapter {
    definitions: Vec<CustomClientDef>,
}

impl CustomHarnessAdapter {
    pub fn load() -> Self {
        let mut defs = Vec::new();

        // 1. Check local ./mcpforge.toml
        Self::try_load_file(Path::new("mcpforge.toml"), &mut defs);

        // 2. Check ~/.config/mcpforge/mcpforge.toml
        if let Some(config_dir) = dirs::config_dir() {
            let global_path = config_dir.join("mcpforge").join("mcpforge.toml");
            Self::try_load_file(&global_path, &mut defs);
        }

        Self { definitions: defs }
    }

    fn try_load_file(path: &Path, acc: &mut Vec<CustomClientDef>) {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(cfg) = toml::from_str::<CustomHarnessConfigFile>(&content) {
                    acc.extend(cfg.clients);
                }
            }
        }
    }
}

impl ClientAdapter for CustomHarnessAdapter {
    fn id(&self) -> &'static str {
        "custom"
    }

    fn display_name(&self) -> &'static str {
        "Custom Harness"
    }

    fn detect(&self) -> Vec<ConfigLocation> {
        let mut locs = Vec::new();
        for def in &self.definitions {
            let expanded_path = if def.path.starts_with("~/") {
                if let Some(home) = dirs::home_dir() {
                    home.join(&def.path[2..])
                } else {
                    PathBuf::from(&def.path)
                }
            } else {
                PathBuf::from(&def.path)
            };

            let exists = expanded_path.exists();
            locs.push(ConfigLocation {
                client_id: def.id.clone(),
                display_name: def.display_name.clone(),
                path: expanded_path,
                scope: Scope::Global,
                exists,
            });
        }
        locs
    }

    fn read_servers(&self, loc: &ConfigLocation) -> Result<Vec<ServerEntry>> {
        let key = self
            .definitions
            .iter()
            .find(|d| d.id == loc.client_id)
            .map(|d| d.servers_key.as_str())
            .unwrap_or("mcpServers");
        read_mcp_servers_from_json(&loc.path, key, loc)
    }

    fn write_servers(&self, loc: &ConfigLocation, entries: &[ServerEntry]) -> Result<()> {
        let key = self
            .definitions
            .iter()
            .find(|d| d.id == loc.client_id)
            .map(|d| d.servers_key.as_str())
            .unwrap_or("mcpServers");
        write_mcp_servers_to_json(&loc.path, key, &loc.client_id, entries)
    }

    fn supports(&self) -> TransportSupport {
        TransportSupport::all()
    }

    fn backup_path(&self, loc: &ConfigLocation) -> PathBuf {
        crate::backup::default_backup_dir().join(format!("{}.bak", loc.client_id))
    }
}
