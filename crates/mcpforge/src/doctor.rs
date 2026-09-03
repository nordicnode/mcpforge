use anyhow::Result;
use mcp_core::client::check_server_health;
use mcp_core::types::{HealthStatus, ServerEntry};
use std::collections::BTreeMap;
use tokio::task::JoinSet;

pub struct DoctorReport {
    pub results: BTreeMap<String, HealthStatus>,
}

impl DoctorReport {
    pub async fn run(servers: &[ServerEntry], timeout_secs: u64) -> Self {
        let mut set = JoinSet::new();

        for server in servers {
            let server_clone = server.clone();
            set.spawn(async move {
                let status = check_server_health(&server_clone, timeout_secs).await;
                (server_clone.id, status)
            });
        }

        let mut results = BTreeMap::new();
        while let Some(res) = set.join_next().await {
            if let Ok((id, status)) = res {
                results.insert(id, status);
            }
        }

        Self { results }
    }

    pub fn print_table(&self) -> bool {
        let mut all_healthy = true;
        println!("\n{:<20} {:<8} {:<40}", "SERVER", "STATUS", "DIAGNOSTICS");
        println!("{}", "-".repeat(75));

        for (id, status) in &self.results {
            let (icon, detail) = match status {
                HealthStatus::Healthy {
                    latency_ms,
                    tool_count,
                    server_name,
                    server_version,
                } => (
                    "● OK",
                    format!(
                        "{} v{} ({} tools, {}ms)",
                        server_name, server_version, tool_count, latency_ms
                    ),
                ),
                HealthStatus::Degraded { reason, latency_ms } => {
                    all_healthy = false;
                    let ms_str = latency_ms.map_or(String::new(), |m| format!(" [{}ms]", m));
                    ("▲ WARN", format!("{}{}", reason, ms_str))
                }
                HealthStatus::Broken { error } => {
                    all_healthy = false;
                    ("✖ FAIL", error.clone())
                }
                HealthStatus::Disabled => ("○ OFF", "Disabled in mcpforge".to_string()),
                HealthStatus::Unknown => ("? UNK", "Not checked".to_string()),
            };

            println!("{:<20} {:<8} {:<40}", id, icon, detail);
        }
        println!();
        all_healthy
    }

    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(&self.results)?)
    }
}
