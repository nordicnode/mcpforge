use crate::backup::{atomic_write, create_backup};
use crate::traits::ConfigLocation;
use anyhow::{Context, Result};
use mcp_core::types::{ClientRef, ServerEntry, Transport};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::Path;

pub fn read_mcp_servers_from_json(
    path: &Path,
    servers_key: &str,
    loc: &ConfigLocation,
) -> Result<Vec<ServerEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file at {:?}", path))?;
    let root: Value = match serde_json::from_str(&content) {
        Ok(val) => val,
        Err(_) => {
            let clean = strip_jsonc_comments(&content);
            match serde_json::from_str(&clean) {
                Ok(val) => val,
                Err(e) => {
                    tracing::warn!("Failed to parse JSON in {:?}: {}", path, e);
                    return Ok(Vec::new());
                }
            }
        }
    };

    let servers_obj = match root.get(servers_key).and_then(|v| v.as_object()) {
        Some(obj) => obj,
        None => return Ok(Vec::new()),
    };

    let mut entries = Vec::new();
    for (name, val) in servers_obj {
        if let Some(entry) = parse_single_server_value(name, val, loc) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

fn parse_single_server_value(name: &str, val: &Value, loc: &ConfigLocation) -> Option<ServerEntry> {
    let client_ref = ClientRef {
        client_id: loc.client_id.clone(),
        display_name: loc.display_name.clone(),
        scope: loc.scope,
        config_path: loc.path.clone(),
    };

    // Check for HTTP / SSE URL first
    if let Some(url) = val.get("url").and_then(|u| u.as_str()) {
        let is_sse =
            val.get("type").and_then(|t| t.as_str()) == Some("sse") || url.contains("/sse");
        let transport = if is_sse {
            Transport::Sse {
                url: url.to_string(),
            }
        } else {
            let mut headers = BTreeMap::new();
            if let Some(h_obj) = val.get("headers").and_then(|h| h.as_object()) {
                for (k, v) in h_obj {
                    if let Some(s) = v.as_str() {
                        headers.insert(k.clone(), s.to_string());
                    }
                }
            }
            Transport::StreamableHttp {
                url: url.to_string(),
                headers,
            }
        };

        return Some(ServerEntry {
            id: name.to_string(),
            transport,
            enabled: true,
            clients: vec![client_ref],
            tags: Vec::new(),
            notes: None,
        });
    }

    // Check for command (stdio)
    if let Some(cmd) = val.get("command").and_then(|c| c.as_str()) {
        let args = match val.get("args").and_then(|a| a.as_array()) {
            Some(arr) => arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            None => Vec::new(),
        };

        let mut env = BTreeMap::new();
        if let Some(env_obj) = val.get("env").and_then(|e| e.as_object()) {
            for (k, v) in env_obj {
                if let Some(s) = v.as_str() {
                    env.insert(k.clone(), s.to_string());
                } else if let Some(n) = v.as_i64() {
                    env.insert(k.clone(), n.to_string());
                } else if let Some(b) = v.as_bool() {
                    env.insert(k.clone(), b.to_string());
                }
            }
        }

        return Some(ServerEntry {
            id: name.to_string(),
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

    None
}

pub fn write_mcp_servers_to_json(
    path: &Path,
    servers_key: &str,
    client_id: &str,
    entries: &[ServerEntry],
) -> Result<()> {
    // 1. Read existing root or create empty object
    let mut root: Value = if path.exists() {
        let content = std::fs::read_to_string(path)?;
        match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => {
                let clean = strip_jsonc_comments(&content);
                serde_json::from_str(&clean).unwrap_or_else(|_| Value::Object(Map::new()))
            }
        }
    } else {
        Value::Object(Map::new())
    };

    let root_map = match root.as_object_mut() {
        Some(m) => m,
        None => {
            root = Value::Object(Map::new());
            root.as_object_mut().unwrap()
        }
    };

    // 2. Access or create servers_key (e.g. "mcpServers")
    if !root_map.contains_key(servers_key) {
        root_map.insert(servers_key.to_string(), Value::Object(Map::new()));
    }

    let servers_val = root_map.get_mut(servers_key).unwrap();
    if !servers_val.is_object() {
        *servers_val = Value::Object(Map::new());
    }
    let servers_map = servers_val.as_object_mut().unwrap();

    // Retain only servers that are present in the provided entries
    let desired_ids: std::collections::HashSet<String> = entries
        .iter()
        .filter(|e| e.enabled)
        .map(|e| e.id.clone())
        .collect();
    servers_map.retain(|k, _| desired_ids.contains(k));

    // 3. Upsert entries
    for entry in entries {
        if !entry.enabled {
            servers_map.remove(&entry.id);
            continue;
        }

        let mut server_json = Map::new();
        match &entry.transport {
            Transport::Stdio { command, args, env } => {
                server_json.insert("command".to_string(), Value::String(command.clone()));
                server_json.insert(
                    "args".to_string(),
                    Value::Array(args.iter().map(|a| Value::String(a.clone())).collect()),
                );
                if !env.is_empty() {
                    let mut env_map = Map::new();
                    for (k, v) in env {
                        env_map.insert(k.clone(), Value::String(v.clone()));
                    }
                    server_json.insert("env".to_string(), Value::Object(env_map));
                }
            }
            Transport::StreamableHttp { url, headers } => {
                server_json.insert("type".to_string(), Value::String("http".to_string()));
                server_json.insert("url".to_string(), Value::String(url.clone()));
                if !headers.is_empty() {
                    let mut headers_map = Map::new();
                    for (k, v) in headers {
                        headers_map.insert(k.clone(), Value::String(v.clone()));
                    }
                    server_json.insert("headers".to_string(), Value::Object(headers_map));
                }
            }
            Transport::Sse { url } => {
                server_json.insert("type".to_string(), Value::String("sse".to_string()));
                server_json.insert("url".to_string(), Value::String(url.clone()));
            }
        }

        servers_map.insert(entry.id.clone(), Value::Object(server_json));
    }

    // 4. Create backup before write
    create_backup(path, client_id)?;

    // 5. Serialize and atomic write
    let output = serde_json::to_string_pretty(&root)? + "\n";
    atomic_write(path, &output)?;

    Ok(())
}

pub fn strip_jsonc_comments(input: &str) -> String {
    let comment_stripped = strip_comments_internal(input);
    remove_trailing_commas(&comment_stripped)
}

fn strip_comments_internal(input: &str) -> String {
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

fn remove_trailing_commas(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut in_string = false;
    let mut in_escape = false;
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

        if !in_string && ch == ',' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                i += 1;
                continue;
            }
        }

        out.push(ch);
        i += 1;
    }

    out
}
