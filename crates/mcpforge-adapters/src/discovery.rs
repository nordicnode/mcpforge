use crate::manager::AdapterManager;
use crate::traits::ConfigLocation;
use mcp_core::types::Scope;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredHarness {
    pub id: String,
    pub display_name: String,
    pub category: String,
    pub config_path: PathBuf,
    pub all_locations: Vec<ConfigLocation>,
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
            .args(["-a", "pi|freebuff|codebuff|grok|jcode|opencode|codex|claude|cursor|windsurf|zed|antigravity|goose|librechat|mcphub|anythingllm|idea|pycharm|hermes|openclaw|deepseek|dsh|prime|letta|memgpt"])
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
            "hermes" => &["hermes"],
            "openclaw" => &["openclaw"],
            "deepseek" => &["dsh", "deepseek"],
            "prime" => &["prime", "prime-agent"],
            "letta" => &["letta", "memgpt"],
            "pi" => &["pi", "pi-agent", "pi-coding-agent"],
            _ => &[],
        };

        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                for bin in bins {
                    if dir.join(bin).is_file() {
                        return true;
                    }
                    #[cfg(windows)]
                    {
                        for ext in [".cmd", ".exe", ".bat"] {
                            if dir.join(format!("{}{}", bin, ext)).is_file() {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            let app_bundles: &[&str] = match client_id {
                "vscode" => &["Visual Studio Code.app", "VSCodium.app"],
                "cursor" => &["Cursor.app"],
                "claude-desktop" => &["Claude.app"],
                "windsurf" => &["Windsurf.app"],
                "zed" => &["Zed.app"],
                "anythingllm" => &["AnythingLLM.app"],
                "librechat" => &["LibreChat.app"],
                "jetbrains" => &[
                    "IntelliJ IDEA.app",
                    "PyCharm.app",
                    "WebStorm.app",
                    "RustRover.app",
                    "CLion.app",
                ],
                _ => &[],
            };
            for bundle in app_bundles {
                if std::path::Path::new("/Applications").join(bundle).exists() {
                    return true;
                }
                if let Some(home) = dirs::home_dir() {
                    if home.join("Applications").join(bundle).exists() {
                        return true;
                    }
                }
            }
        }

        #[cfg(windows)]
        {
            if let Some(local_app_data) = dirs::data_local_dir() {
                let programs = local_app_data.join("Programs");
                for bin in bins {
                    if programs.join(bin).join(format!("{}.exe", bin)).is_file() {
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
        if name.contains("hermes") {
            set.insert("hermes".to_string());
            return;
        }
        if name.contains("openclaw") {
            set.insert("openclaw".to_string());
            return;
        }
        if name.contains("deepseek") || name == "dsh" {
            set.insert("deepseek".to_string());
            return;
        }
        if name.contains("prime") {
            set.insert("prime".to_string());
            return;
        }
        if name.contains("letta") || name.contains("memgpt") {
            set.insert("letta".to_string());
            return;
        }
        if name == "pi" || name.starts_with("pi-") || name.contains("pi-agent") {
            set.insert("pi".to_string());
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
            let id = adapter.id().to_string();
            let display_name = adapter.display_name().to_string();
            let locations = adapter.detect();

            if id == "custom" && locations.is_empty() {
                continue;
            }

            let category = match id.as_str() {
                "freebuff" | "goose" | "hermes" | "openclaw" | "deepseek" | "prime" | "letta"
                | "pi" => "Agent".to_string(),
                "claude-code" | "codex" | "opencode" | "antigravity" | "jcode" | "manicode"
                | "grok" => "CLI".to_string(),
                "cursor" | "vscode" | "windsurf" | "zed" | "jetbrains" | "mcphub" | "cline"
                | "roo-code" | "continue" => "IDE".to_string(),
                "claude-desktop" | "librechat" | "anythingllm" => "Chat".to_string(),
                _ => "Other".to_string(),
            };

            let is_on_path = Self::is_client_installed(&id);
            let has_existing_config = locations.iter().any(|l| l.exists);
            let is_installed = is_on_path || has_existing_config;

            let is_running = is_installed
                && (running_processes.contains(&id)
                    || (id == "cline" && running_processes.contains("vscode")));

            // Primary config path: prefer first that exists, else first global, else first
            let config_path = locations
                .iter()
                .find(|l| l.exists)
                .or_else(|| locations.iter().find(|l| l.scope == Scope::Global))
                .or_else(|| locations.first())
                .map(|l| l.path.clone())
                .unwrap_or_else(|| PathBuf::from("unconfigured"));

            // Count unique servers configured in any existing location for this client
            let mut unique_servers = HashSet::new();
            for loc in &locations {
                if loc.exists {
                    if let Ok(servers) = adapter.read_servers(loc) {
                        for s in servers {
                            unique_servers.insert(s.id);
                        }
                    }
                }
            }
            let server_count = unique_servers.len();

            results.push(DiscoveredHarness {
                id,
                display_name,
                category,
                config_path,
                all_locations: locations,
                is_running,
                is_installed,
                server_count,
            });
        }

        // Sort results:
        // Tier 0: ACTIVE (running && installed)
        // Tier 1: RUNNING (running && !installed)
        // Tier 2: READY (installed)
        // Tier 3: AVAILABLE (unconfigured)
        results.sort_by_key(|h| {
            let tier = if h.is_running && h.is_installed {
                0u8
            } else if h.is_running {
                1u8
            } else if h.is_installed {
                2u8
            } else {
                3u8
            };
            (tier, h.category.clone(), h.display_name.clone())
        });

        results
    }
}
