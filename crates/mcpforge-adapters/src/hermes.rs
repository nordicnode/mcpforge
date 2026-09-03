use crate::backup::{atomic_write, create_backup};
use crate::traits::{ClientAdapter, ConfigLocation, TransportSupport};
use anyhow::Result;
use mcp_core::types::{ClientRef, Scope, ServerEntry, Transport};
use serde_yaml::{Mapping, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Default)]
pub struct HermesAdapter;

impl HermesAdapter {
    pub fn new() -> Self {
        Self
    }

    fn possible_paths() -> Vec<(PathBuf, Scope, &'static str)> {
        let mut paths = Vec::new();

        if let Some(home) = dirs::home_dir() {
            paths.push((
                home.join(".hermes").join("config.yaml"),
                Scope::Global,
                "Hermes Agent (Home Config)",
            ));
            paths.push((
                home.join(".config").join("hermes").join("config.yaml"),
                Scope::Global,
                "Hermes Agent (Config Dir)",
            ));
        }

        paths.push((
            PathBuf::from(".hermes").join("config.yaml"),
            Scope::Project,
            "Hermes Agent (Project .hermes)",
        ));
        paths.push((
            PathBuf::from("hermes.yaml"),
            Scope::Project,
            "Hermes Agent (Project hermes.yaml)",
        ));

        paths
    }
}

impl ClientAdapter for HermesAdapter {
    fn id(&self) -> &'static str {
        "hermes"
    }

    fn display_name(&self) -> &'static str {
        "Hermes Agent"
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
        if !loc.path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&loc.path)?;
        let root: Value = match serde_yaml::from_str(&content) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut entries = Vec::new();

        let servers_val = root.get("mcp_servers").or_else(|| root.get("mcpServers"));
        if let Some(servers) = servers_val.and_then(|s| s.as_mapping()) {
            for (k, v) in servers {
                let id = match k.as_str() {
                    Some(s) => s.to_string(),
                    None => continue,
                };

                let enabled = v.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);
                let client_ref = ClientRef {
                    client_id: self.id().to_string(),
                    display_name: loc.display_name.clone(),
                    scope: loc.scope,
                    config_path: loc.path.clone(),
                };

                if let Some(url) = v.get("url").and_then(|u| u.as_str()) {
                    let mut headers = BTreeMap::new();
                    if let Some(h_map) = v.get("headers").and_then(|h| h.as_mapping()) {
                        for (hk, hv) in h_map {
                            if let (Some(hk_str), Some(hv_str)) = (hk.as_str(), hv.as_str()) {
                                headers.insert(hk_str.to_string(), hv_str.to_string());
                            }
                        }
                    }
                    entries.push(ServerEntry {
                        id,
                        transport: Transport::StreamableHttp {
                            url: url.to_string(),
                            headers,
                        },
                        enabled,
                        clients: vec![client_ref],
                        tags: Vec::new(),
                        notes: None,
                    });
                } else if let Some(cmd) = v
                    .get("command")
                    .or_else(|| v.get("cmd"))
                    .and_then(|c| c.as_str())
                {
                    let args = v
                        .get("args")
                        .and_then(|a| a.as_sequence())
                        .map(|seq| {
                            seq.iter()
                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    let mut env = BTreeMap::new();
                    let env_val = v.get("env").or_else(|| v.get("envs"));
                    if let Some(e_map) = env_val.and_then(|e| e.as_mapping()) {
                        for (ek, ev) in e_map {
                            if let (Some(ek_str), Some(ev_str)) = (ek.as_str(), ev.as_str()) {
                                env.insert(ek_str.to_string(), ev_str.to_string());
                            }
                        }
                    }

                    entries.push(ServerEntry {
                        id,
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
        }

        Ok(entries)
    }

    fn write_servers(&self, loc: &ConfigLocation, entries: &[ServerEntry]) -> Result<()> {
        let mut root: Value = if loc.path.exists() {
            let content = std::fs::read_to_string(&loc.path)?;
            serde_yaml::from_str(&content).unwrap_or_else(|_| Value::Mapping(Mapping::new()))
        } else {
            Value::Mapping(Mapping::new())
        };

        if !root.is_mapping() {
            root = Value::Mapping(Mapping::new());
        }

        let mut servers_map = Mapping::new();

        for entry in entries {
            if !entry.enabled {
                continue;
            }

            let mut server_obj = Mapping::new();

            match &entry.transport {
                Transport::Stdio { command, args, env } => {
                    server_obj.insert(
                        Value::String("command".to_string()),
                        Value::String(command.clone()),
                    );
                    server_obj.insert(
                        Value::String("args".to_string()),
                        Value::Sequence(args.iter().map(|a| Value::String(a.clone())).collect()),
                    );
                    if !env.is_empty() {
                        let mut env_map = Mapping::new();
                        for (k, v) in env {
                            env_map.insert(Value::String(k.clone()), Value::String(v.clone()));
                        }
                        server_obj
                            .insert(Value::String("env".to_string()), Value::Mapping(env_map));
                    }
                }
                Transport::StreamableHttp { url, headers } => {
                    server_obj.insert(Value::String("url".to_string()), Value::String(url.clone()));
                    server_obj.insert(
                        Value::String("transport".to_string()),
                        Value::String("sse".to_string()),
                    );
                    if !headers.is_empty() {
                        let mut h_map = Mapping::new();
                        for (k, v) in headers {
                            h_map.insert(Value::String(k.clone()), Value::String(v.clone()));
                        }
                        server_obj
                            .insert(Value::String("headers".to_string()), Value::Mapping(h_map));
                    }
                }
                Transport::Sse { url } => {
                    server_obj.insert(Value::String("url".to_string()), Value::String(url.clone()));
                    server_obj.insert(
                        Value::String("transport".to_string()),
                        Value::String("sse".to_string()),
                    );
                }
            }

            server_obj.insert(Value::String("enabled".to_string()), Value::Bool(true));
            servers_map.insert(Value::String(entry.id.clone()), Value::Mapping(server_obj));
        }

        let root_map = root.as_mapping_mut().unwrap();
        root_map.insert(
            Value::String("mcp_servers".to_string()),
            Value::Mapping(servers_map),
        );

        create_backup(&loc.path, self.id())?;
        let output = serde_yaml::to_string(&root)?;
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
