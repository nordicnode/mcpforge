use crate::backup::{atomic_write, create_backup};
use crate::traits::{ClientAdapter, ConfigLocation, TransportSupport};
use anyhow::Result;
use mcp_core::types::{ClientRef, Scope, ServerEntry, Transport};
use serde_yaml::{Mapping, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Default)]
pub struct GooseAdapter;

impl GooseAdapter {
    pub fn new() -> Self {
        Self
    }

    fn possible_paths() -> Vec<(PathBuf, Scope, &'static str)> {
        let mut paths = Vec::new();

        if let Some(config_dir) = dirs::config_dir() {
            paths.push((
                config_dir.join("goose").join("config.yaml"),
                Scope::Global,
                "Goose (Global Config)",
            ));
        }

        paths.push((
            PathBuf::from(".goose").join("config.yaml"),
            Scope::Project,
            "Goose (Project .goose)",
        ));
        paths.push((
            PathBuf::from("goose.yaml"),
            Scope::Project,
            "Goose (Project goose.yaml)",
        ));

        paths
    }
}

impl ClientAdapter for GooseAdapter {
    fn id(&self) -> &'static str {
        "goose"
    }

    fn display_name(&self) -> &'static str {
        "Goose"
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

        if let Some(exts) = root.get("extensions").and_then(|e| e.as_mapping()) {
            for (k, v) in exts {
                let id = match k.as_str() {
                    Some(s) => s.to_string(),
                    None => continue,
                };

                let enabled = v.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);
                let ext_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("stdio");

                let client_ref = ClientRef {
                    client_id: self.id().to_string(),
                    display_name: loc.display_name.clone(),
                    scope: loc.scope,
                    config_path: loc.path.clone(),
                };

                if ext_type == "sse" || ext_type == "streamable-http" || ext_type == "remote" {
                    if let Some(url) = v
                        .get("uri")
                        .or_else(|| v.get("url"))
                        .and_then(|u| u.as_str())
                    {
                        entries.push(ServerEntry {
                            id,
                            transport: Transport::StreamableHttp {
                                url: url.to_string(),
                                headers: BTreeMap::new(),
                            },
                            enabled,
                            clients: vec![client_ref],
                            tags: Vec::new(),
                            notes: None,
                        });
                    }
                } else if let Some(cmd) = v
                    .get("cmd")
                    .or_else(|| v.get("command"))
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
                    let envs = v.get("envs").or_else(|| v.get("env"));
                    if let Some(e_map) = envs.and_then(|e| e.as_mapping()) {
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

        let mut exts_map = Mapping::new();

        for entry in entries {
            if !entry.enabled {
                continue;
            }

            let mut ext_obj = Mapping::new();
            ext_obj.insert(
                Value::String("name".to_string()),
                Value::String(entry.id.clone()),
            );

            match &entry.transport {
                Transport::Stdio { command, args, env } => {
                    ext_obj.insert(
                        Value::String("type".to_string()),
                        Value::String("stdio".to_string()),
                    );
                    ext_obj.insert(
                        Value::String("cmd".to_string()),
                        Value::String(command.clone()),
                    );
                    ext_obj.insert(
                        Value::String("args".to_string()),
                        Value::Sequence(args.iter().map(|a| Value::String(a.clone())).collect()),
                    );
                    if !env.is_empty() {
                        let mut env_map = Mapping::new();
                        for (k, v) in env {
                            env_map.insert(Value::String(k.clone()), Value::String(v.clone()));
                        }
                        ext_obj.insert(Value::String("envs".to_string()), Value::Mapping(env_map));
                    }
                }
                Transport::StreamableHttp { url, .. } | Transport::Sse { url } => {
                    ext_obj.insert(
                        Value::String("type".to_string()),
                        Value::String("sse".to_string()),
                    );
                    ext_obj.insert(Value::String("uri".to_string()), Value::String(url.clone()));
                }
            }

            ext_obj.insert(Value::String("enabled".to_string()), Value::Bool(true));
            exts_map.insert(Value::String(entry.id.clone()), Value::Mapping(ext_obj));
        }

        let root_map = root.as_mapping_mut().unwrap();
        root_map.insert(
            Value::String("extensions".to_string()),
            Value::Mapping(exts_map),
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
