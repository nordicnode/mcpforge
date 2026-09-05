use anyhow::Result;
use mcp_core::client::check_server_health;
use mcp_core::types::{HealthStatus, ServerEntry, Transport};
use mcpforge_adapters::AdapterManager;
use mcpforge_registry::Registry;
use std::collections::BTreeMap;
use tokio::task::JoinSet;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiagnosticPrescription {
    pub server_id: String,
    pub issue: String,
    pub root_cause: String,
    pub prescription: String,
    pub copy_paste_fix: Option<String>,
    pub can_autofix: bool,
    pub missing_env_key: Option<String>,
    pub discovered_env_val: Option<String>,
}

pub struct DoctorReport {
    pub results: BTreeMap<String, HealthStatus>,
    pub prescriptions: Vec<DiagnosticPrescription>,
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

        let registry = Registry::default();
        let resolver = crate::resolver::EnvResolver::new();
        let mut prescriptions = Vec::new();

        for server in servers {
            if let Some(
                HealthStatus::Broken { error } | HealthStatus::Degraded { reason: error, .. },
            ) = results.get(&server.id)
            {
                let cat_entry = registry.find_by_id(&server.id);

                // 1. Check for missing required environment variables
                let mut missing_env = None;
                if let Some(entry) = cat_entry.as_ref() {
                    if let Transport::Stdio { ref env, .. } = server.transport {
                        for req in &entry.required_env {
                            let is_missing = match env.get(req) {
                                Some(val) => val.trim().is_empty(),
                                None => true,
                            };
                            if is_missing {
                                missing_env = Some(req.clone());
                                break;
                            }
                        }
                    }
                }

                if let Some(ref req_key) = missing_env {
                    let (resolved_env, _) =
                        resolver.resolve_for_keys(std::slice::from_ref(req_key));
                    let host_val = resolved_env.get(req_key).cloned();
                    let (can_fix, prescription, copy_fix) = if host_val.is_some() {
                        (
                            true,
                            format!("Discovered '{}' in environment/secrets store. Auto-healing can inject this credential.", req_key),
                            Some("mcpforge doctor --fix".to_string()),
                        )
                    } else {
                        let doc_link = cat_entry
                            .as_ref()
                            .and_then(|c| c.source_url.as_deref())
                            .unwrap_or("https://github.com/modelcontextprotocol/servers");
                        (
                            false,
                            format!("Missing required environment variable '{}'. Upstream documentation: {}", req_key, doc_link),
                            Some(format!("export {}=\"<your-api-key>\" (or run 'mcpforge secret set {}')", req_key, req_key)),
                        )
                    };

                    prescriptions.push(DiagnosticPrescription {
                        server_id: server.id.clone(),
                        issue: "Missing Authentication / Configuration Credential".to_string(),
                        root_cause: format!(
                            "Required environment variable '{}' is not configured",
                            req_key
                        ),
                        prescription,
                        copy_paste_fix: copy_fix,
                        can_autofix: can_fix,
                        missing_env_key: Some(req_key.clone()),
                        discovered_env_val: host_val,
                    });
                    continue;
                }

                // 2. Classify binary not found in PATH
                if error.contains("not found in PATH") {
                    let cmd = match &server.transport {
                        Transport::Stdio { command, .. } => command.as_str(),
                        _ => "executable",
                    };

                    let (remedy, copy_fix) = match cmd {
                                "npx" | "node" => (
                                    "Node.js runtime is not installed or not in PATH".to_string(),
                                    Some("curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash - && sudo apt-get install -y nodejs".to_string()),
                                ),
                                "uvx" | "uv" => (
                                    "Astral uv Python package manager is not installed in PATH".to_string(),
                                    Some("curl -LsSf https://astral.sh/uv/install.sh | sh".to_string()),
                                ),
                                "docker" => (
                                    "Docker CLI is not found or daemon is not active".to_string(),
                                    Some("sudo systemctl start docker".to_string()),
                                ),
                                _ => (
                                    format!("Command '{}' is not installed or directory is not in PATH", cmd),
                                    Some(format!("which {} || export PATH=\"$PATH:/usr/local/bin:$HOME/.local/bin\"", cmd)),
                                ),
                            };

                    prescriptions.push(DiagnosticPrescription {
                        server_id: server.id.clone(),
                        issue: format!("Missing Executable Runtime ('{}')", cmd),
                        root_cause: error.clone(),
                        prescription: remedy,
                        copy_paste_fix: copy_fix,
                        can_autofix: false,
                        missing_env_key: None,
                        discovered_env_val: None,
                    });
                    continue;
                }

                // 3. Process stream closed / crash
                if error.contains("Process stream closed") {
                    prescriptions.push(DiagnosticPrescription {
                                server_id: server.id.clone(),
                                issue: "Process Terminated Prematurely".to_string(),
                                root_cause: "Server process exited or failed initialization handshake".to_string(),
                                prescription: "Test running the server command directly with 'mcpforge test' or verify standard input/output".to_string(),
                                copy_paste_fix: Some(format!("mcpforge test {}", server.id)),
                                can_autofix: false,
                                missing_env_key: None,
                                discovered_env_val: None,
                            });
                    continue;
                }

                // 4. HTTP / Network errors
                if error.contains("401") || error.contains("Unauthorized") {
                    prescriptions.push(DiagnosticPrescription {
                        server_id: server.id.clone(),
                        issue: "HTTP 401 Unauthorized".to_string(),
                        root_cause:
                            "Remote server rejected request headers or bearer authorization token"
                                .to_string(),
                        prescription: "Update authentication headers with valid credentials"
                            .to_string(),
                        copy_paste_fix: None,
                        can_autofix: false,
                        missing_env_key: None,
                        discovered_env_val: None,
                    });
                    continue;
                }

                // Fallback generic prescription
                prescriptions.push(DiagnosticPrescription {
                    server_id: server.id.clone(),
                    issue: "Server Communication Error".to_string(),
                    root_cause: error.clone(),
                    prescription: "Inspect detailed handshake traces using 'mcpforge test'"
                        .to_string(),
                    copy_paste_fix: Some(format!("mcpforge test {}", server.id)),
                    can_autofix: false,
                    missing_env_key: None,
                    discovered_env_val: None,
                });
            }
        }

        Self {
            results,
            prescriptions,
        }
    }

    pub fn print_table(&self) -> bool {
        let mut all_healthy = true;
        println!("\n{:<20} {:<8} {:<45}", "SERVER", "STATUS", "DIAGNOSTICS");
        println!("{}", "-".repeat(78));

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

            println!("{:<20} {:<8} {:<45}", id, icon, detail);
        }
        println!();

        if !self.prescriptions.is_empty() {
            println!("DIAGNOSTIC PRESCRIPTIONS & REMEDIATIONS");
            println!("{}", "=".repeat(78));
            for p in &self.prescriptions {
                println!("▶ [{}] {}", p.server_id, p.issue);
                println!("  Root Cause:   {}", p.root_cause);
                println!("  Prescription: {}", p.prescription);
                if let Some(ref fix) = p.copy_paste_fix {
                    println!("  Recommended:  {}", fix);
                }
                if p.can_autofix {
                    println!(
                        "  Self-Healing: Available! Run 'mcpforge doctor --fix' to auto-resolve."
                    );
                }
                println!();
            }
        }

        all_healthy
    }

    pub fn auto_heal(&self, manager: &AdapterManager, servers: &[ServerEntry]) -> Result<usize> {
        let mut fixed = 0;
        let all_locations = manager.detect_all();

        for p in &self.prescriptions {
            if p.can_autofix {
                if let (Some(ref key), Some(ref val)) = (&p.missing_env_key, &p.discovered_env_val)
                {
                    if let Some(target_server) = servers.iter().find(|s| s.id == p.server_id) {
                        let mut updated = target_server.clone();
                        if let Transport::Stdio { ref mut env, .. } = updated.transport {
                            env.insert(key.clone(), val.clone());
                        }

                        // Write to all locations where this server was installed
                        let targets: Vec<_> = all_locations
                            .iter()
                            .filter(|l| {
                                target_server
                                    .clients
                                    .iter()
                                    .any(|c| c.client_id == l.client_id)
                            })
                            .cloned()
                            .collect();

                        if !targets.is_empty() {
                            manager.write_server_to_locations(&updated, &targets)?;
                            println!(
                                "✓ Auto-healed '{}': injected '{}' into {} client configuration(s)",
                                p.server_id,
                                key,
                                targets.len()
                            );
                            fixed += 1;
                        }
                    }
                }
            }
        }

        Ok(fixed)
    }

    pub fn to_json(&self) -> Result<String> {
        let json_data = serde_json::json!({
            "health": self.results,
            "prescriptions": self.prescriptions,
        });
        Ok(serde_json::to_string_pretty(&json_data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_prescription_serialization() {
        let p = DiagnosticPrescription {
            server_id: "test-server".to_string(),
            issue: "Missing Credential".to_string(),
            root_cause: "Variable FOO not found".to_string(),
            prescription: "Export FOO".to_string(),
            copy_paste_fix: Some("export FOO=bar".to_string()),
            can_autofix: true,
            missing_env_key: Some("FOO".to_string()),
            discovered_env_val: Some("bar".to_string()),
        };

        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"server_id\":\"test-server\""));
        assert!(json.contains("\"can_autofix\":true"));
    }

    #[tokio::test]
    async fn test_doctor_report_empty_servers() {
        let report = DoctorReport::run(&[], 1).await;
        assert!(report.results.is_empty());
        assert!(report.prescriptions.is_empty());
    }
}
