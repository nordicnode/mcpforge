use crate::backup::{atomic_write, create_backup};
use crate::traits::{ClientAdapter, ConfigLocation, TransportSupport};
use anyhow::Result;
use mcp_core::types::{ClientRef, Scope, ServerEntry, Transport};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Default)]
pub struct OpenCodeAdapter;

impl OpenCodeAdapter {
    pub fn new() -> Self {
        Self
    }

    fn possible_paths() -> Vec<(PathBuf, Scope, &'static str)> {
        let mut paths = Vec::new();

        if let Some(config_dir) = dirs::config_dir() {
            paths.push((
                config_dir.join("opencode").join("opencode.jsonc"),
                Scope::Global,
                "OpenCode (Config jsonc)",
            ));
            paths.push((
                config_dir.join("opencode").join("opencode.json"),
                Scope::Global,
                "OpenCode (Config json)",
            ));
        }

        paths.push((
            PathBuf::from("opencode.json"),
            Scope::Project,
            "OpenCode (Project opencode.json)",
        ));
        paths.push((
            PathBuf::from(".opencode").join("opencode.json"),
            Scope::Project,
            "OpenCode (Project .opencode)",
        ));

        paths
    }
}

impl ClientAdapter for OpenCodeAdapter {
    fn id(&self) -> &'static str {
        "opencode"
    }

    fn display_name(&self) -> &'static str {
        "OpenCode"
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
        let clean_json = strip_jsonc_comments(&content);
        let root: Value = match serde_json::from_str(&clean_json) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut entries = Vec::new();

        let mcp_obj = root.get("mcp").or_else(|| root.get("mcpServers"));
        if let Some(obj) = mcp_obj.and_then(|o| o.as_object()) {
            for (id, val) in obj {
                let enabled = val.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);
                let client_ref = ClientRef {
                    client_id: self.id().to_string(),
                    display_name: loc.display_name.clone(),
                    scope: loc.scope,
                    config_path: loc.path.clone(),
                };

                let server_type = val.get("type").and_then(|t| t.as_str()).unwrap_or("local");
                if server_type == "remote" {
                    if let Some(url) = val.get("url").and_then(|u| u.as_str()) {
                        let mut headers = BTreeMap::new();
                        if let Some(h_obj) = val.get("headers").and_then(|h| h.as_object()) {
                            for (hk, hv) in h_obj {
                                if let Some(s) = hv.as_str() {
                                    headers.insert(hk.clone(), s.to_string());
                                }
                            }
                        }
                        entries.push(ServerEntry {
                            id: id.clone(),
                            transport: Transport::StreamableHttp {
                                url: url.to_string(),
                                headers,
                            },
                            enabled,
                            clients: vec![client_ref],
                            tags: Vec::new(),
                            notes: None,
                        });
                    }
                } else {
                    // Local server: "command" can be an array [cmd, arg1, arg2...] (OpenCode spec) or string
                    let (cmd, args) =
                        if let Some(arr) = val.get("command").and_then(|c| c.as_array()) {
                            let mut list: Vec<String> = arr
                                .iter()
                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                .collect();
                            if list.is_empty() {
                                continue;
                            }
                            let command = list.remove(0);
                            (command, list)
                        } else if let Some(c_str) = val.get("command").and_then(|c| c.as_str()) {
                            let a = val
                                .get("args")
                                .and_then(|a| a.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                        .collect()
                                })
                                .unwrap_or_default();
                            (c_str.to_string(), a)
                        } else {
                            continue;
                        };

                    let mut env = BTreeMap::new();
                    let env_val = val.get("environment").or_else(|| val.get("env"));
                    if let Some(e_obj) = env_val.and_then(|e| e.as_object()) {
                        for (ek, ev) in e_obj {
                            if let Some(s) = ev.as_str() {
                                env.insert(ek.clone(), s.to_string());
                            }
                        }
                    }

                    entries.push(ServerEntry {
                        id: id.clone(),
                        transport: Transport::Stdio {
                            command: cmd,
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
            let clean = strip_jsonc_comments(&content);
            serde_json::from_str(&clean).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({
                "$schema": "https://opencode.ai/config.json"
            })
        };

        if !root.is_object() {
            root = serde_json::json!({
                "$schema": "https://opencode.ai/config.json"
            });
        }

        let mut mcp_map = Map::new();
        for entry in entries {
            if !entry.enabled {
                continue;
            }

            match &entry.transport {
                Transport::Stdio { command, args, env } => {
                    let mut obj = Map::new();
                    obj.insert("type".to_string(), Value::String("local".to_string()));

                    // OpenCode official schema: command is array of [executable, ...args]
                    let mut cmd_array = vec![Value::String(command.clone())];
                    for arg in args {
                        cmd_array.push(Value::String(arg.clone()));
                    }
                    obj.insert("command".to_string(), Value::Array(cmd_array));

                    if !env.is_empty() {
                        let mut env_map = Map::new();
                        for (k, v) in env {
                            env_map.insert(k.clone(), Value::String(v.clone()));
                        }
                        obj.insert("environment".to_string(), Value::Object(env_map));
                    }
                    obj.insert("enabled".to_string(), Value::Bool(true));
                    mcp_map.insert(entry.id.clone(), Value::Object(obj));
                }
                Transport::StreamableHttp { url, headers } => {
                    let mut obj = Map::new();
                    obj.insert("type".to_string(), Value::String("remote".to_string()));
                    obj.insert("url".to_string(), Value::String(url.clone()));
                    if !headers.is_empty() {
                        let mut h_map = Map::new();
                        for (k, v) in headers {
                            h_map.insert(k.clone(), Value::String(v.clone()));
                        }
                        obj.insert("headers".to_string(), Value::Object(h_map));
                    }
                    obj.insert("enabled".to_string(), Value::Bool(true));
                    mcp_map.insert(entry.id.clone(), Value::Object(obj));
                }
                Transport::Sse { url } => {
                    let mut obj = Map::new();
                    obj.insert("type".to_string(), Value::String("remote".to_string()));
                    obj.insert("url".to_string(), Value::String(url.clone()));
                    obj.insert("enabled".to_string(), Value::Bool(true));
                    mcp_map.insert(entry.id.clone(), Value::Object(obj));
                }
            }
        }

        let root_obj = root.as_object_mut().unwrap();
        root_obj.insert("mcp".to_string(), Value::Object(mcp_map));

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

fn strip_jsonc_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_string = false;
    let mut in_escape = false;
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        if in_escape {
            out.push(ch);
            in_escape = false;
            i += 1;
            continue;
        }

        if ch == '\\' && in_string {
            in_escape = true;
            out.push(ch);
            i += 1;
            continue;
        }

        if ch == '"' {
            in_string = !in_string;
            out.push(ch);
            i += 1;
            continue;
        }

        if !in_string && i + 1 < chars.len() && ch == '/' && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        if !in_string && i + 1 < chars.len() && ch == '/' && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }

        out.push(ch);
        i += 1;
    }

    out
}
