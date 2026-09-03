use crate::backup::{atomic_write, create_backup};
use crate::traits::{ClientAdapter, ConfigLocation, TransportSupport};
use anyhow::{Context, Result};
use mcp_core::types::{ClientRef, Scope, ServerEntry, Transport};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Default)]
pub struct GrokAdapter;

impl GrokAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl ClientAdapter for GrokAdapter {
    fn id(&self) -> &'static str {
        "grok"
    }

    fn display_name(&self) -> &'static str {
        "Grok Build"
    }

    fn detect(&self) -> Vec<ConfigLocation> {
        let mut locs = Vec::new();

        // 1. Global ~/.grok/config.toml
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".grok").join("config.toml");
            let exists = path.exists();
            locs.push(ConfigLocation {
                client_id: self.id().to_string(),
                display_name: format!("{} (Global)", self.display_name()),
                path,
                scope: Scope::Global,
                exists,
            });
        }

        // 2. Project .grok/config.toml
        let project_path = PathBuf::from(".grok").join("config.toml");
        let exists = project_path.exists();
        locs.push(ConfigLocation {
            client_id: self.id().to_string(),
            display_name: format!("{} (Project)", self.display_name()),
            path: project_path,
            scope: Scope::Project,
            exists,
        });

        locs
    }

    fn read_servers(&self, loc: &ConfigLocation) -> Result<Vec<ServerEntry>> {
        if !loc.path.exists() {
            return Ok(Vec::new());
        }

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
            let enabled = val.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);

            let client_ref = ClientRef {
                client_id: self.id().to_string(),
                display_name: loc.display_name.clone(),
                scope: loc.scope,
                config_path: loc.path.clone(),
            };

            if let Some(url) = val.get("url").and_then(|u| u.as_str()) {
                let mut headers = BTreeMap::new();
                if let Some(h_tab) = val.get("headers").and_then(|h| h.as_table()) {
                    for (hk, hv) in h_tab {
                        if let Some(s) = hv.as_str() {
                            headers.insert(hk.clone(), s.to_string());
                        }
                    }
                }
                entries.push(ServerEntry {
                    id: name.clone(),
                    transport: Transport::StreamableHttp {
                        url: url.to_string(),
                        headers,
                    },
                    enabled,
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
                    enabled,
                    clients: vec![client_ref],
                    tags: Vec::new(),
                    notes: None,
                });
            }
        }

        Ok(entries)
    }

    fn write_servers(&self, loc: &ConfigLocation, entries: &[ServerEntry]) -> Result<()> {
        let mut root: toml::Value = if loc.path.exists() {
            let content = std::fs::read_to_string(&loc.path)?;
            toml::from_str(&content).unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()))
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
                    server_tab.insert("command".to_string(), toml::Value::String(command.clone()));
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
                    server_tab.insert("enabled".to_string(), toml::Value::Boolean(true));
                }
                Transport::StreamableHttp { url, headers } => {
                    server_tab.insert("url".to_string(), toml::Value::String(url.clone()));
                    if !headers.is_empty() {
                        let mut h_tab = toml::map::Map::new();
                        for (k, v) in headers {
                            h_tab.insert(k.clone(), toml::Value::String(v.clone()));
                        }
                        server_tab.insert("headers".to_string(), toml::Value::Table(h_tab));
                    }
                    server_tab.insert("enabled".to_string(), toml::Value::Boolean(true));
                }
                Transport::Sse { url } => {
                    server_tab.insert("url".to_string(), toml::Value::String(url.clone()));
                    server_tab.insert("enabled".to_string(), toml::Value::Boolean(true));
                }
            }

            mcp_table.insert(entry.id.clone(), toml::Value::Table(server_tab));
        }

        create_backup(&loc.path, self.id())?;
        let output = toml::to_string_pretty(&root)? + "\n";
        atomic_write(&loc.path, &output)?;

        Ok(())
    }

    fn supports(&self) -> TransportSupport {
        TransportSupport::stdio_and_http()
    }

    fn backup_path(&self, _loc: &ConfigLocation) -> PathBuf {
        crate::backup::default_backup_dir().join(format!("{}.bak", self.id()))
    }
}
