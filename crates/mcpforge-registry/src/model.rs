use mcp_core::types::{ServerEntry, Transport};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub required_env: Vec<String>,
    #[serde(default)]
    pub official_repo: Option<String>,
    #[serde(default = "default_transport")]
    pub transport: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub last_verified: Option<String>,
    #[serde(default)]
    pub maintainer: Option<String>,
}

fn default_transport() -> String {
    "stdio".to_string()
}

impl CatalogEntry {
    pub fn to_server_entry(&self, env: BTreeMap<String, String>) -> ServerEntry {
        ServerEntry {
            id: self.id.clone(),
            transport: Transport::Stdio {
                command: self.command.clone(),
                args: self.args.clone(),
                env,
            },
            enabled: true,
            clients: Vec::new(),
            tags: self.tags.clone(),
            notes: Some(self.description.clone()),
        }
    }
}
