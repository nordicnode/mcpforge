use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerEntry {
    pub id: String,
    pub transport: Transport,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub clients: Vec<ClientRef>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

fn default_enabled() -> bool {
    true
}

impl ServerEntry {
    pub fn new_stdio(
        id: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
        env: BTreeMap<String, String>,
    ) -> Self {
        Self {
            id: id.into(),
            transport: Transport::Stdio {
                command: command.into(),
                args,
                env,
            },
            enabled: true,
            clients: Vec::new(),
            tags: Vec::new(),
            notes: None,
        }
    }

    pub fn new_http(
        id: impl Into<String>,
        url: impl Into<String>,
        headers: BTreeMap<String, String>,
    ) -> Self {
        Self {
            id: id.into(),
            transport: Transport::StreamableHttp {
                url: url.into(),
                headers,
            },
            enabled: true,
            clients: Vec::new(),
            tags: Vec::new(),
            notes: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Transport {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: BTreeMap<String, String>,
    },
    StreamableHttp {
        url: String,
        #[serde(default)]
        headers: BTreeMap<String, String>,
    },
    Sse {
        url: String,
    },
}

impl Transport {
    pub fn transport_type_str(&self) -> &'static str {
        match self {
            Transport::Stdio { .. } => "stdio",
            Transport::StreamableHttp { .. } => "streamable_http",
            Transport::Sse { .. } => "sse (legacy)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientRef {
    pub client_id: String,
    pub display_name: String,
    pub scope: Scope,
    pub config_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Global,
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum HealthStatus {
    #[default]
    Unknown,
    Healthy {
        latency_ms: u64,
        tool_count: usize,
        server_name: String,
        server_version: String,
    },
    Degraded {
        reason: String,
        latency_ms: Option<u64>,
    },
    Broken {
        error: String,
    },
    Disabled,
}

impl HealthStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            HealthStatus::Healthy { .. } => "●",
            HealthStatus::Degraded { .. } => "▲",
            HealthStatus::Broken { .. } => "✖",
            HealthStatus::Disabled => "○",
            HealthStatus::Unknown => "?",
        }
    }

    pub fn status_text(&self) -> String {
        match self {
            HealthStatus::Healthy {
                latency_ms,
                tool_count,
                server_name,
                server_version,
            } => {
                format!(
                    "Healthy: {} v{} ({} tools, {}ms)",
                    server_name, server_version, tool_count, latency_ms
                )
            }
            HealthStatus::Degraded { reason, latency_ms } => {
                if let Some(ms) = latency_ms {
                    format!("Degraded: {} ({}ms)", reason, ms)
                } else {
                    format!("Degraded: {}", reason)
                }
            }
            HealthStatus::Broken { error } => format!("Broken: {}", error),
            HealthStatus::Disabled => "Disabled".to_string(),
            HealthStatus::Unknown => "Unknown (not checked)".to_string(),
        }
    }
}
