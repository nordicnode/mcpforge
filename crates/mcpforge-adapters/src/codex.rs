use crate::backup::{atomic_write, create_backup};
use crate::common::{read_mcp_servers_from_json, write_mcp_servers_to_json};
use crate::traits::{ClientAdapter, ConfigLocation, TransportSupport};
use anyhow::{Context, Result};
use mcp_core::types::{ClientRef, Scope, ServerEntry, Transport};
use std::collections::BTreeMap;
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

        // 1. Global ~/.codex/config.toml (Codex CLI native)
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".codex").join("config.toml");
            let exists = path.exists();
            locs.push(ConfigLocation {
                client_id: self.id().to_string(),
                display_name: format!("{} (CLI Global)", self.display_name()),
                path,
                scope: Scope::Global,
                exists,
            });
        }

        // 2. Project .codex/config.toml
        let project_toml = PathBuf::from(".codex").join("config.toml");
        let exists = project_toml.exists();
        locs.push(ConfigLocation {
            client_id: self.id().to_string(),
            display_name: format!("{} (CLI Project)", self.display_name()),
            path: project_toml,
            scope: Scope::Project,
            exists,
        });

        // 3. Optional IDE ~/.config/codex/mcp.json
        if let Some(config_dir) = dirs::config_dir() {
            let path = config_dir.join("codex").join("mcp.json");
            let exists = path.exists();
            locs.push(ConfigLocation {
                client_id: self.id().to_string(),
                display_name: format!("{} (IDE Config)", self.display_name()),
                path,
                scope: Scope::Global,
                exists,
            });
        }

        locs
    }

    fn read_servers(&self, loc: &ConfigLocation) -> Result<Vec<ServerEntry>> {
        if !loc.path.exists() {
            return Ok(Vec::new());
        }

        if loc.path.extension().and_then(|e| e.to_str()) == Some("toml") {
            let content = std::fs::read_to_string(&loc.path)?;
            let root: toml::Value = match toml::from_str(&content) {
                Ok(v) => v,
                Err(_) => return Ok(Vec::new()),
            };

            let mcp_servers = match root.get("mcp_servers").and_then(|v| v.as_table()) {
                Some(t) => t,
                None => return Ok(Vec::new()),
            };

            let mut entries = Vec::new();
            for (name, val) in mcp_servers {
                let client_ref = ClientRef {
                    client_id: self.id().to_string(),
                    display_name: loc.display_name.clone(),
                    scope: loc.scope,
                    config_path: loc.path.clone(),
                };

                if let Some(url) = val.get("url").and_then(|u| u.as_str()) {
                    entries.push(ServerEntry {
                        id: name.clone(),
                        transport: Transport::StreamableHttp {
                            url: url.to_string(),
                            headers: BTreeMap::new(),
                        },
                        enabled: true,
                        clients: vec![client_ref],
                        tags: Vec::new(),
                        notes: None,
                    });
                } else if let Some(cmd) = val.get("command").and_then(|c| c.as_str()) {
                    let args = val
                        .get("args")
                        .and_then(|a| a.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    let mut env = BTreeMap::new();
                    if let Some(e_tab) = val.get("env").and_then(|e| e.as_table()) {
                        for (ek, ev) in e_tab {
                            if let Some(s) = ev.as_str() {
                                env.insert(ek.clone(), s.to_string());
                            }
                        }
                    }

                    entries.push(ServerEntry {
                        id: name.clone(),
                        transport: Transport::Stdio {
                            command: cmd.to_string(),
                            args,
                            env,
                        },
                        enabled: true,
                        clients: vec![client_ref],
                        tags: Vec::new(),
                        notes: None,
                    });
                }
            }
            Ok(entries)
        } else {
            read_mcp_servers_from_json(&loc.path, "mcpServers", loc)
        }
    }

    fn write_servers(&self, loc: &ConfigLocation, entries: &[ServerEntry]) -> Result<()> {
        if loc.path.extension().and_then(|e| e.to_str()) == Some("toml") {
            let mut root: toml::Value = if loc.path.exists() {
                let content = std::fs::read_to_string(&loc.path)?;
                toml::from_str(&content)
                    .unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()))
            } else {
                toml::Value::Table(toml::map::Map::new())
            };

            let root_table = match root.as_table_mut() {
                Some(t) => t,
                None => {
                    root = toml::Value::Table(toml::map::Map::new());
                    root.as_table_mut().unwrap()
                }
            };

            if !root_table.contains_key("mcp_servers") {
                root_table.insert(
                    "mcp_servers".to_string(),
                    toml::Value::Table(toml::map::Map::new()),
                );
            }

            let mcp_table = root_table
                .get_mut("mcp_servers")
                .unwrap()
                .as_table_mut()
                .context("mcp_servers is not a table")?;

            for entry in entries {
                if !entry.enabled {
                    mcp_table.remove(&entry.id);
                    continue;
                }

                let mut server_tab = toml::map::Map::new();
                match &entry.transport {
                    Transport::Stdio { command, args, env } => {
                        server_tab
                            .insert("command".to_string(), toml::Value::String(command.clone()));
                        server_tab.insert(
                            "args".to_string(),
                            toml::Value::Array(
                                args.iter()
                                    .map(|a| toml::Value::String(a.clone()))
                                    .collect(),
                            ),
                        );
                        if !env.is_empty() {
                            let mut env_tab = toml::map::Map::new();
                            for (k, v) in env {
                                env_tab.insert(k.clone(), toml::Value::String(v.clone()));
                            }
                            server_tab.insert("env".to_string(), toml::Value::Table(env_tab));
                        }
                    }
                    Transport::StreamableHttp { url, .. } | Transport::Sse { url } => {
                        server_tab.insert("url".to_string(), toml::Value::String(url.clone()));
                    }
                }

                mcp_table.insert(entry.id.clone(), toml::Value::Table(server_tab));
            }

            create_backup(&loc.path, self.id())?;
            let output = toml::to_string_pretty(&root)? + "\n";
            atomic_write(&loc.path, &output)?;
            Ok(())
        } else {
            write_mcp_servers_to_json(&loc.path, "mcpServers", self.id(), entries)
        }
    }

    fn supports(&self) -> TransportSupport {
        TransportSupport::stdio_and_http()
    }

    fn backup_path(&self, _loc: &ConfigLocation) -> PathBuf {
        crate::backup::default_backup_dir().join(format!("{}.bak", self.id()))
    }
}
