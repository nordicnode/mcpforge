use crate::manager::AdapterManager;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredHarness {
    pub id: String,
    pub display_name: String,
    pub config_path: PathBuf,
    pub is_running: bool,
    pub is_installed: bool,
    pub server_count: usize,
}

pub struct DiscoveryEngine {
    manager: AdapterManager,
}

impl Default for DiscoveryEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscoveryEngine {
    pub fn new() -> Self {
        Self {
            manager: AdapterManager::new(),
        }
    }

    pub fn scan_running_processes() -> HashSet<String> {
        let mut running = HashSet::new();

        // 1. Try reading /proc on Linux
        #[cfg(target_os = "linux")]
        {
            if let Ok(entries) = std::fs::read_dir("/proc") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }
                    let comm_path = path.join("comm");
                    if let Ok(comm) = std::fs::read_to_string(&comm_path) {
                        let comm = comm.trim().to_lowercase();
                        Self::map_comm_to_client(&comm, &mut running);
                    }
                }
            }
        }

        // 2. Fallback or augment with pgrep / ps command check
        if running.is_empty() {
            if let Ok(output) = std::process::Command::new("pgrep")
                .args([
                    "-l",
                    "-i",
                    "code|cursor|claude|windsurf|zed|antigravity|freebuff",
                ])
                .output()
            {
                if let Ok(out_str) = String::from_utf8(output.stdout) {
                    for line in out_str.lines() {
                        let lower = line.to_lowercase();
                        Self::map_comm_to_client(&lower, &mut running);
                    }
                }
            }
        }

        running
    }

    fn map_comm_to_client(name: &str, set: &mut HashSet<String>) {
        if name.contains("cursor") {
            set.insert("cursor".to_string());
        }
        if name.contains("claude") {
            set.insert("claude-code".to_string());
            set.insert("claude-desktop".to_string());
        }
        if name.contains("code") || name.contains("codium") {
            set.insert("vscode".to_string());
            set.insert("cline".to_string());
        }
        if name.contains("windsurf") {
            set.insert("windsurf".to_string());
        }
        if name.contains("zed") {
            set.insert("zed".to_string());
        }
        if name.contains("antigravity") {
            set.insert("antigravity".to_string());
        }
        if name.contains("continue") {
            set.insert("continue".to_string());
        }
        if name.contains("freebuff") {
            set.insert("freebuff".to_string());
        }
    }

    pub fn discover_all(&self) -> Vec<DiscoveredHarness> {
        let running_processes = Self::scan_running_processes();
        let mut results = Vec::new();

        for adapter in self.manager.adapters() {
            for loc in adapter.detect() {
                let is_running = running_processes.contains(adapter.id())
                    || (adapter.id() == "cline" && running_processes.contains("vscode"));

                let server_count = if loc.exists {
                    adapter.read_servers(&loc).map(|s| s.len()).unwrap_or(0)
                } else {
                    0
                };

                results.push(DiscoveredHarness {
                    id: loc.client_id.clone(),
                    display_name: loc.display_name.clone(),
                    config_path: loc.path.clone(),
                    is_running,
                    is_installed: loc.exists,
                    server_count,
                });
            }
        }

        // Also scan current workspace and subdirectories for project configs
        Self::scan_workspace_configs(&mut results, &running_processes);

        results
    }

    fn scan_workspace_configs(
        results: &mut Vec<DiscoveredHarness>,
        running_processes: &HashSet<String>,
    ) {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Check cwd and immediate subdirectories
        let candidate_paths = [
            cwd.join(".mcp.json"),
            cwd.join(".cursor").join("mcp.json"),
            cwd.join(".vscode").join("mcp.json"),
        ];

        for path in candidate_paths {
            if path.exists() && !results.iter().any(|r| r.config_path == path) {
                let client_id = if path.to_string_lossy().contains(".cursor") {
                    "cursor"
                } else if path.to_string_lossy().contains(".vscode") {
                    "vscode"
                } else {
                    "claude-code"
                };

                let is_running = running_processes.contains(client_id);
                results.push(DiscoveredHarness {
                    id: client_id.to_string(),
                    display_name: format!("Workspace ({})", path.display()),
                    config_path: path,
                    is_running,
                    is_installed: true,
                    server_count: 0,
                });
            }
        }
    }
}
