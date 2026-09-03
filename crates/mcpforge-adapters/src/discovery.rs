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

        // 2. Query pgrep with strict pattern
        if let Ok(output) = std::process::Command::new("pgrep")
            .args(["-a", "freebuff|codebuff|grok|jcode|opencode|codex|claude|cursor|windsurf|zed|antigravity|goose|librechat|mcphub|anythingllm|idea|pycharm"])
            .output()
        {
            if let Ok(out_str) = String::from_utf8(output.stdout) {
                for line in out_str.lines() {
                    let lower = line.to_lowercase();
                    Self::map_comm_to_client(&lower, &mut running);
                }
            }
        }

        running
    }

    pub fn is_client_installed(client_id: &str) -> bool {
        let bins: &[&str] = match client_id {
            "vscode" => &["code", "code-insiders", "codium"],
            "cursor" => &["cursor"],
            "claude-code" => &["claude"],
            "claude-desktop" => &["claude-desktop"],
            "windsurf" => &["windsurf"],
            "zed" => &["zed", "zed-editor"],
            "freebuff" => &["freebuff", "freebuff-desktop"],
            "grok" => &["grok"],
            "jcode" => &["jcode"],
            "opencode" => &["opencode"],
            "codex" => &["codex"],
            "manicode" => &["manicode"],
            "goose" => &["goose"],
            "librechat" => &["librechat"],
            "mcphub" => &["mcp-hub", "nvim"],
            "anythingllm" => &["anythingllm", "anythingllm-desktop"],
            "jetbrains" => &["idea", "pycharm", "webstorm", "clion", "rustrover"],
            _ => &[],
        };

        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                for bin in bins {
                    if dir.join(bin).is_file() {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn map_comm_to_client(name: &str, set: &mut HashSet<String>) {
        // Match specific clients first before any substrings!
        if name.contains("freebuff") || name.contains("codebuff") {
            set.insert("freebuff".to_string());
            return;
        }
        if name.contains("opencode") {
            set.insert("opencode".to_string());
            return;
        }
        if name.contains("jcode") {
            set.insert("jcode".to_string());
            return;
        }
        if name.contains("claude") {
            set.insert("claude-code".to_string());
            set.insert("claude-desktop".to_string());
            return;
        }
        if name.contains("codex") {
            set.insert("codex".to_string());
            return;
        }
        if name.contains("grok") {
            set.insert("grok".to_string());
            return;
        }
        if name.contains("cursor") {
            set.insert("cursor".to_string());
            return;
        }
        if name.contains("windsurf") {
            set.insert("windsurf".to_string());
            return;
        }
        if name.contains("zed") {
            set.insert("zed".to_string());
            return;
        }
        if name.contains("antigravity") {
            set.insert("antigravity".to_string());
            return;
        }
        if name.contains("continue") {
            set.insert("continue".to_string());
            return;
        }
        if name.contains("roo") {
            set.insert("roo-code".to_string());
            return;
        }
        if name.contains("manicode") {
            set.insert("manicode".to_string());
            return;
        }
        if name.contains("goose") {
            set.insert("goose".to_string());
            return;
        }
        if name.contains("librechat") {
            set.insert("librechat".to_string());
            return;
        }
        if name.contains("mcphub") || name.contains("mcp-hub") {
            set.insert("mcphub".to_string());
            return;
        }
        if name.contains("anythingllm") {
            set.insert("anythingllm".to_string());
            return;
        }
        if name.contains("idea") || name.contains("pycharm") || name.contains("webstorm") {
            set.insert("jetbrains".to_string());
            return;
        }

        // Strict check for real VS Code / Codium:
        // Must NOT match codebuff, opencode, jcode, or other agents
        let is_real_vscode = name == "code"
            || name == "code-insiders"
            || name == "codium"
            || name.ends_with("/code")
            || name.ends_with("/code-insiders")
            || name.ends_with("/codium");

        if is_real_vscode {
            set.insert("vscode".to_string());
            set.insert("cline".to_string());
        }
    }

    pub fn discover_all(&self) -> Vec<DiscoveredHarness> {
        let running_processes = Self::scan_running_processes();
        let mut results = Vec::new();

        for adapter in self.manager.adapters() {
            for loc in adapter.detect() {
                // A client can only be running if it exists on disk or its binary is on PATH
                let is_installed = loc.exists || Self::is_client_installed(adapter.id());
                let is_running = is_installed
                    && (running_processes.contains(adapter.id())
                        || (adapter.id() == "cline" && running_processes.contains("vscode")));

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
                    is_installed,
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

                let is_installed = Self::is_client_installed(client_id);
                let is_running = is_installed && running_processes.contains(client_id);

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
