use crate::backup::{atomic_write, create_backup};
use crate::traits::{ClientAdapter, ConfigLocation, TransportSupport};
use anyhow::Result;
use mcp_core::types::{ClientRef, Scope, ServerEntry, Transport};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Default)]
pub struct ContinueAdapter;

impl ContinueAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl ClientAdapter for ContinueAdapter {
    fn id(&self) -> &'static str {
        "continue"
    }

    fn display_name(&self) -> &'static str {
        "Continue.dev"
    }

    fn detect(&self) -> Vec<ConfigLocation> {
        let mut locs = Vec::new();
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".continue").join("config.json");
            let exists = path.exists();
            locs.push(ConfigLocation {
                client_id: self.id().to_string(),
                display_name: self.display_name().to_string(),
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

        let content = std::fs::read_to_string(&loc.path)?;
        let root: Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut entries = Vec::new();

        // 1. Check experimental.modelContextProtocolServers array
        if let Some(servers_arr) = root
            .get("experimental")
            .and_then(|e| e.get("modelContextProtocolServers"))
            .and_then(|s| s.as_array())
        {
            for (idx, s_val) in servers_arr.iter().enumerate() {
                let transport_obj = s_val.get("transport");
                let (command, args) = match transport_obj {
                    Some(t) => {
                        let cmd = t
                            .get("command")
                            .and_then(|c| c.as_str())
                            .unwrap_or("")
                            .to_string();
                        let a = t
                            .get("args")
                            .and_then(|a| a.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();
                        (cmd, a)
                    }
                    None => continue,
                };

                if command.is_empty() {
                    continue;
                }

                let id = s_val
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("continue-server-{}", idx + 1));

                entries.push(ServerEntry {
                    id,
                    transport: Transport::Stdio {
                        command,
                        args,
                        env: BTreeMap::new(),
                    },
                    enabled: true,
                    clients: vec![ClientRef {
                        client_id: self.id().to_string(),
                        display_name: self.display_name().to_string(),
                        scope: loc.scope,
                        config_path: loc.path.clone(),
                    }],
                    tags: Vec::new(),
                    notes: None,
                });
            }
        }

        // 2. Also check top-level mcpServers if present
        if let Some(obj) = root.get("mcpServers").and_then(|m| m.as_object()) {
            for (k, v) in obj {
                if let Some(cmd) = v.get("command").and_then(|c| c.as_str()) {
                    let args = v
                        .get("args")
                        .and_then(|a| a.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|s| s.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    entries.push(ServerEntry {
                        id: k.clone(),
                        transport: Transport::Stdio {
                            command: cmd.to_string(),
                            args,
                            env: BTreeMap::new(),
                        },
                        enabled: true,
                        clients: vec![ClientRef {
                            client_id: self.id().to_string(),
                            display_name: self.display_name().to_string(),
                            scope: loc.scope,
                            config_path: loc.path.clone(),
                        }],
                        tags: Vec::new(),
                        notes: None,
                    });
                }
            }
        }

        Ok(entries)
    }

    fn write_servers(&self, loc: &ConfigLocation, entries: &[ServerEntry]) -> Result<()> {
        let mut root: Value = if loc.path.exists() {
            let content = std::fs::read_to_string(&loc.path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        if !root.is_object() {
            root = serde_json::json!({});
        }

        let mut servers_arr = Vec::new();
        for e in entries {
            if !e.enabled {
                continue;
            }
            if let Transport::Stdio { command, args, .. } = &e.transport {
                servers_arr.push(serde_json::json!({
                    "name": e.id,
                    "transport": {
                        "type": "stdio",
                        "command": command,
                        "args": args
                    }
                }));
            }
        }

        let root_map = root.as_object_mut().unwrap();
        if !root_map.contains_key("experimental") {
            root_map.insert("experimental".to_string(), serde_json::json!({}));
        }
        let exp = root_map.get_mut("experimental").unwrap();
        if !exp.is_object() {
            *exp = serde_json::json!({});
        }
        exp.as_object_mut().unwrap().insert(
            "modelContextProtocolServers".to_string(),
            Value::Array(servers_arr),
        );

        create_backup(&loc.path, self.id())?;
        let output = serde_json::to_string_pretty(&root)? + "\n";
        atomic_write(&loc.path, &output)?;

        Ok(())
    }

    fn supports(&self) -> TransportSupport {
        TransportSupport::stdio_only()
    }

    fn backup_path(&self, _loc: &ConfigLocation) -> PathBuf {
        crate::backup::default_backup_dir().join(format!("{}.bak", self.id()))
    }
}
