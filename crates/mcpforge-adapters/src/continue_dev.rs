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
                let id = s_val
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("continue-server-{}", idx + 1));

                let client_ref = ClientRef {
                    client_id: self.id().to_string(),
                    display_name: self.display_name().to_string(),
                    scope: loc.scope,
                    config_path: loc.path.clone(),
                };

                let transport_obj = s_val.get("transport");
                if let Some(t) = transport_obj {
                    // Check for SSE / HTTP URL
                    if let Some(url) = t
                        .get("url")
                        .or_else(|| s_val.get("url"))
                        .and_then(|u| u.as_str())
                    {
                        entries.push(ServerEntry {
                            id,
                            transport: Transport::Sse {
                                url: url.to_string(),
                            },
                            enabled: true,
                            clients: vec![client_ref],
                            tags: Vec::new(),
                            notes: None,
                        });
                        continue;
                    }

                    // Check for stdio command
                    let cmd = t
                        .get("command")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();

                    if !cmd.is_empty() {
                        let args = t
                            .get("args")
                            .and_then(|a| a.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();

                        let mut env = BTreeMap::new();
                        let env_val = t.get("env").or_else(|| s_val.get("env"));
                        if let Some(e_obj) = env_val.and_then(|e| e.as_object()) {
                            for (k, v) in e_obj {
                                if let Some(s) = v.as_str() {
                                    env.insert(k.clone(), s.to_string());
                                } else if let Some(n) = v.as_i64() {
                                    env.insert(k.clone(), n.to_string());
                                } else if let Some(b) = v.as_bool() {
                                    env.insert(k.clone(), b.to_string());
                                }
                            }
                        }

                        entries.push(ServerEntry {
                            id,
                            transport: Transport::Stdio {
                                command: cmd,
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
            }
        }

        // 2. Also check top-level mcpServers if present
        if let Some(obj) = root.get("mcpServers").and_then(|m| m.as_object()) {
            for (k, v) in obj {
                let client_ref = ClientRef {
                    client_id: self.id().to_string(),
                    display_name: self.display_name().to_string(),
                    scope: loc.scope,
                    config_path: loc.path.clone(),
                };

                if let Some(url) = v.get("url").and_then(|u| u.as_str()) {
                    entries.push(ServerEntry {
                        id: k.clone(),
                        transport: Transport::Sse {
                            url: url.to_string(),
                        },
                        enabled: true,
                        clients: vec![client_ref],
                        tags: Vec::new(),
                        notes: None,
                    });
                } else if let Some(cmd) = v.get("command").and_then(|c| c.as_str()) {
                    let args = v
                        .get("args")
                        .and_then(|a| a.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|s| s.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    let mut env = BTreeMap::new();
                    if let Some(e_obj) = v.get("env").and_then(|e| e.as_object()) {
                        for (ek, ev) in e_obj {
                            if let Some(s) = ev.as_str() {
                                env.insert(ek.clone(), s.to_string());
                            } else if let Some(n) = ev.as_i64() {
                                env.insert(ek.clone(), n.to_string());
                            } else if let Some(b) = ev.as_bool() {
                                env.insert(ek.clone(), b.to_string());
                            }
                        }
                    }

                    entries.push(ServerEntry {
                        id: k.clone(),
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
            match &e.transport {
                Transport::Stdio { command, args, env } => {
                    let mut transport_map = serde_json::Map::new();
                    transport_map.insert("type".to_string(), serde_json::json!("stdio"));
                    transport_map.insert("command".to_string(), serde_json::json!(command));
                    transport_map.insert("args".to_string(), serde_json::json!(args));
                    if !env.is_empty() {
                        let mut env_map = serde_json::Map::new();
                        for (k, v) in env {
                            env_map.insert(k.clone(), serde_json::json!(v));
                        }
                        transport_map.insert("env".to_string(), Value::Object(env_map));
                    }
                    servers_arr.push(serde_json::json!({
                        "name": e.id,
                        "transport": Value::Object(transport_map)
                    }));
                }
                Transport::StreamableHttp { url, .. } | Transport::Sse { url } => {
                    servers_arr.push(serde_json::json!({
                        "name": e.id,
                        "transport": {
                            "type": "sse",
                            "url": url
                        }
                    }));
                }
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
        TransportSupport::stdio_and_http()
    }

    fn backup_path(&self, _loc: &ConfigLocation) -> PathBuf {
        crate::backup::default_backup_dir().join(format!("{}.bak", self.id()))
    }
}
