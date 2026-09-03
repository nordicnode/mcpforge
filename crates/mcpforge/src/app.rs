use anyhow::Result;
use mcp_core::types::{HealthStatus, Scope, ServerEntry, Transport};
use mcpforge_adapters::{
    compute_diff, AdapterManager, ConfigLocation, DiscoveredHarness, DiscoveryEngine,
};
use mcpforge_registry::{CatalogEntry, Registry};
use std::collections::BTreeMap;

pub const REGISTRY_CATEGORIES: &[&str] = &[
    "All",
    "ai-agent",
    "dev tools",
    "data",
    "web",
    "git",
    "cloud",
    "productivity",
];

pub const CATEGORY_LABELS: &[&str] = &[
    "All (110)",
    "Agents",
    "Dev Tools",
    "Data & DBs",
    "Web",
    "Git & Code",
    "Cloud",
    "Productivity",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentView {
    Dashboard,
    Clients,
    AddWizard,
    Help,
    DeleteConfirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    SelectSource,
    ConfigureServer,
    SelectTargets,
    PreviewDiff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardSource {
    FromRegistry,
    PasteJson,
    Manual,
}

pub struct WizardState {
    pub step: WizardStep,
    pub source: WizardSource,
    pub registry_cursor: usize,
    pub registry_category_index: usize,
    pub server_id: String,
    pub command: String,
    pub args: String,
    pub pasted_json: String,
    pub target_locations: Vec<(ConfigLocation, bool)>,
    pub target_cursor: usize,
    pub diff_preview: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteState {
    pub server_id: String,
    pub target_locations: Vec<(ConfigLocation, bool)>,
    pub target_cursor: usize,
    pub remove_all_mode: bool,
}

pub struct App {
    pub manager: AdapterManager,
    pub registry: Registry,
    pub servers: Vec<ServerEntry>,
    pub health_cache: BTreeMap<String, HealthStatus>,
    pub detected_clients: Vec<ConfigLocation>,
    pub selected_index: usize,
    pub search_query: String,
    pub is_searching: bool,
    pub current_view: CurrentView,
    pub wizard_state: Option<WizardState>,
    pub delete_state: Option<DeleteState>,
    pub should_quit: bool,
    pub status_message: Option<String>,
    pub running_processes: std::collections::HashSet<String>,
    pub discovered_clients: Vec<DiscoveredHarness>,
    pub selected_client_index: usize,
}

impl App {
    pub fn new() -> Result<Self> {
        let manager = AdapterManager::new();
        let registry = Registry::load().unwrap_or_default();
        let detected_clients = manager.detect_all();
        let servers = manager.read_all_servers().unwrap_or_default();
        let running_processes = DiscoveryEngine::scan_running_processes();
        let discovered_clients = DiscoveryEngine::new().discover_all();

        Ok(Self {
            manager,
            registry,
            servers,
            health_cache: BTreeMap::new(),
            detected_clients,
            selected_index: 0,
            search_query: String::new(),
            is_searching: false,
            current_view: CurrentView::Dashboard,
            wizard_state: None,
            delete_state: None,
            should_quit: false,
            status_message: None,
            running_processes,
            discovered_clients,
            selected_client_index: 0,
        })
    }

    pub fn refresh_discovery(&mut self) {
        self.running_processes = DiscoveryEngine::scan_running_processes();
        self.discovered_clients = DiscoveryEngine::new().discover_all();
        self.detected_clients = self.manager.detect_all();
    }

    pub fn select_next_client(&mut self) {
        if !self.discovered_clients.is_empty() {
            self.selected_client_index =
                (self.selected_client_index + 1) % self.discovered_clients.len();
        }
    }

    pub fn select_prev_client(&mut self) {
        if !self.discovered_clients.is_empty() {
            if self.selected_client_index == 0 {
                self.selected_client_index = self.discovered_clients.len() - 1;
            } else {
                self.selected_client_index -= 1;
            }
        }
    }

    pub fn selected_client(&self) -> Option<&DiscoveredHarness> {
        self.discovered_clients.get(self.selected_client_index)
    }

    pub fn auto_sync_all(&mut self) -> Result<usize> {
        let all_servers = self.manager.read_all_servers()?;
        let all_targets: Vec<ConfigLocation> = self
            .detected_clients
            .iter()
            .filter(|l| l.exists)
            .cloned()
            .collect();

        for server in &all_servers {
            self.manager
                .write_server_to_locations(server, &all_targets)?;
        }

        let count = all_servers.len();
        self.refresh_servers();
        self.status_message = Some(format!(
            "Auto-synced {} servers across {} clients",
            count,
            all_targets.len()
        ));
        Ok(count)
    }

    pub fn filtered_servers(&self) -> Vec<&ServerEntry> {
        if self.search_query.trim().is_empty() {
            self.servers.iter().collect()
        } else {
            let q = self.search_query.to_lowercase();
            self.servers
                .iter()
                .filter(|s| {
                    s.id.to_lowercase().contains(&q)
                        || s.tags.iter().any(|t| t.to_lowercase().contains(&q))
                        || match &s.transport {
                            Transport::Stdio { command, args, .. } => {
                                command.to_lowercase().contains(&q)
                                    || args.iter().any(|a| a.to_lowercase().contains(&q))
                            }
                            Transport::StreamableHttp { url, .. } | Transport::Sse { url } => {
                                url.to_lowercase().contains(&q)
                            }
                        }
                })
                .collect()
        }
    }

    pub fn selected_server(&self) -> Option<&ServerEntry> {
        let filtered = self.filtered_servers();
        if filtered.is_empty() {
            None
        } else {
            let idx = self.selected_index.min(filtered.len().saturating_sub(1));
            Some(filtered[idx])
        }
    }

    pub fn select_next(&mut self) {
        let count = self.filtered_servers().len();
        if count > 0 {
            self.selected_index = (self.selected_index + 1) % count;
        }
    }

    pub fn select_prev(&mut self) {
        let count = self.filtered_servers().len();
        if count > 0 {
            if self.selected_index == 0 {
                self.selected_index = count - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    pub fn refresh_servers(&mut self) {
        if let Ok(servers) = self.manager.read_all_servers() {
            self.servers = servers;
            let count = self.filtered_servers().len();
            if self.selected_index >= count && count > 0 {
                self.selected_index = count - 1;
            }
            self.status_message = Some("Refreshed servers from clients".to_string());
        }
    }

    pub fn filtered_registry_entries(&self) -> Vec<CatalogEntry> {
        let all = self.registry.entries();
        if let Some(ref wizard) = self.wizard_state {
            let cat = REGISTRY_CATEGORIES
                .get(wizard.registry_category_index)
                .copied()
                .unwrap_or("All");
            if cat == "All" {
                all.to_vec()
            } else {
                all.iter().filter(|e| e.category == cat).cloned().collect()
            }
        } else {
            all.to_vec()
        }
    }

    pub fn next_registry_category(&mut self) {
        if let Some(ref mut wizard) = self.wizard_state {
            wizard.registry_category_index =
                (wizard.registry_category_index + 1) % REGISTRY_CATEGORIES.len();
            wizard.registry_cursor = 0;
        }
    }

    pub fn prev_registry_category(&mut self) {
        if let Some(ref mut wizard) = self.wizard_state {
            if wizard.registry_category_index == 0 {
                wizard.registry_category_index = REGISTRY_CATEGORIES.len() - 1;
            } else {
                wizard.registry_category_index -= 1;
            }
            wizard.registry_cursor = 0;
        }
    }

    pub fn set_registry_category(&mut self, index: usize) {
        if let Some(ref mut wizard) = self.wizard_state {
            if index < REGISTRY_CATEGORIES.len() {
                wizard.registry_category_index = index;
                wizard.registry_cursor = 0;
            }
        }
    }

    pub fn next_registry_item(&mut self) {
        let count = self.filtered_registry_entries().len();
        if let Some(ref mut wizard) = self.wizard_state {
            if wizard.registry_cursor + 1 < count {
                wizard.registry_cursor += 1;
            }
        }
    }

    pub fn prev_registry_item(&mut self) {
        if let Some(ref mut wizard) = self.wizard_state {
            if wizard.registry_cursor > 0 {
                wizard.registry_cursor -= 1;
            }
        }
    }

    pub fn start_wizard(&mut self) {
        let locations: Vec<(ConfigLocation, bool)> = self
            .discovered_clients
            .iter()
            .map(|h| {
                let loc = ConfigLocation {
                    client_id: h.id.clone(),
                    display_name: h.display_name.clone(),
                    path: h.config_path.clone(),
                    scope: Scope::Global,
                    exists: h.is_installed,
                };
                let should_select = h.is_installed;
                (loc, should_select)
            })
            .collect();

        self.wizard_state = Some(WizardState {
            step: WizardStep::SelectSource,
            source: WizardSource::FromRegistry,
            registry_cursor: 0,
            registry_category_index: 0,
            server_id: String::new(),
            command: String::new(),
            args: String::new(),
            pasted_json: String::new(),
            target_locations: locations,
            target_cursor: 0,
            diff_preview: String::new(),
            error_message: None,
        });
        self.current_view = CurrentView::AddWizard;
    }

    pub fn compute_wizard_diff(&mut self) {
        let entries = self.filtered_registry_entries();
        if let Some(ref mut wizard) = self.wizard_state {
            let mut diffs = String::new();
            let new_server = match wizard.source {
                WizardSource::FromRegistry => {
                    if wizard.registry_cursor < entries.len() {
                        let cat_entry = &entries[wizard.registry_cursor];
                        cat_entry.to_server_entry(BTreeMap::new())
                    } else {
                        return;
                    }
                }
                WizardSource::Manual => {
                    let args = wizard
                        .args
                        .split_whitespace()
                        .map(|s| s.to_string())
                        .collect();
                    ServerEntry::new_stdio(
                        &wizard.server_id,
                        &wizard.command,
                        args,
                        BTreeMap::new(),
                    )
                }
                WizardSource::PasteJson => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&wizard.pasted_json)
                    {
                        if let Some(obj) = val.as_object() {
                            if let Some(cmd) = obj.get("command").and_then(|c| c.as_str()) {
                                let args = obj
                                    .get("args")
                                    .and_then(|a| a.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                            .collect()
                                    })
                                    .unwrap_or_default();
                                ServerEntry::new_stdio(
                                    &wizard.server_id,
                                    cmd,
                                    args,
                                    BTreeMap::new(),
                                )
                            } else {
                                wizard.error_message =
                                    Some("JSON must have 'command' property".to_string());
                                return;
                            }
                        } else {
                            wizard.error_message = Some("Invalid JSON object".to_string());
                            return;
                        }
                    } else {
                        wizard.error_message = Some("Failed to parse JSON".to_string());
                        return;
                    }
                }
            };

            for (loc, selected) in &wizard.target_locations {
                if *selected {
                    let old_content = std::fs::read_to_string(&loc.path).unwrap_or_default();
                    let mut simulated_json: serde_json::Value = serde_json::from_str(&old_content)
                        .unwrap_or_else(|_| serde_json::json!({}));

                    if !simulated_json.is_object() {
                        simulated_json = serde_json::json!({});
                    }
                    if !simulated_json
                        .as_object()
                        .unwrap()
                        .contains_key("mcpServers")
                    {
                        simulated_json
                            .as_object_mut()
                            .unwrap()
                            .insert("mcpServers".to_string(), serde_json::json!({}));
                    }

                    let server_json = match &new_server.transport {
                        Transport::Stdio { command, args, env } => {
                            serde_json::json!({
                                "command": command,
                                "args": args,
                                "env": env,
                            })
                        }
                        Transport::StreamableHttp { url, headers } => {
                            serde_json::json!({
                                "url": url,
                                "headers": headers,
                            })
                        }
                        Transport::Sse { url } => {
                            serde_json::json!({
                                "type": "sse",
                                "url": url,
                            })
                        }
                    };

                    simulated_json
                        .get_mut("mcpServers")
                        .unwrap()
                        .as_object_mut()
                        .unwrap()
                        .insert(new_server.id.clone(), server_json);

                    let new_content =
                        serde_json::to_string_pretty(&simulated_json).unwrap_or_default() + "\n";
                    let file_name = loc
                        .path
                        .file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or("config.json");
                    let d = compute_diff(&old_content, &new_content, file_name);
                    diffs.push_str(&format!(
                        "--- Target: {} ({})\n",
                        loc.display_name,
                        loc.path.display()
                    ));
                    diffs.push_str(&d);
                    diffs.push('\n');
                }
            }

            wizard.diff_preview = diffs;
        }
    }

    pub fn apply_wizard(&mut self) -> Result<()> {
        let entries = self.filtered_registry_entries();
        let wizard = match self.wizard_state.take() {
            Some(w) => w,
            None => return Ok(()),
        };

        let new_server = match wizard.source {
            WizardSource::FromRegistry => {
                if wizard.registry_cursor < entries.len() {
                    let cat_entry = &entries[wizard.registry_cursor];
                    cat_entry.to_server_entry(BTreeMap::new())
                } else {
                    return Ok(());
                }
            }
            WizardSource::Manual => {
                let args = wizard
                    .args
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                ServerEntry::new_stdio(&wizard.server_id, &wizard.command, args, BTreeMap::new())
            }
            WizardSource::PasteJson => {
                let val: serde_json::Value = serde_json::from_str(&wizard.pasted_json)?;
                let obj = val.as_object().unwrap();
                let cmd = obj.get("command").and_then(|c| c.as_str()).unwrap();
                let args = obj
                    .get("args")
                    .and_then(|a| a.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                ServerEntry::new_stdio(&wizard.server_id, cmd, args, BTreeMap::new())
            }
        };

        let targets: Vec<ConfigLocation> = wizard
            .target_locations
            .into_iter()
            .filter(|(_, sel)| *sel)
            .map(|(loc, _)| loc)
            .collect();

        self.manager
            .write_server_to_locations(&new_server, &targets)?;
        self.refresh_servers();
        self.status_message = Some(format!(
            "Successfully installed '{}' to {} clients",
            new_server.id,
            targets.len()
        ));
        self.current_view = CurrentView::Dashboard;
        Ok(())
    }

    pub fn start_delete(&mut self) {
        if let Some(server) = self.selected_server().cloned() {
            let locs: Vec<(ConfigLocation, bool)> = self
                .detected_clients
                .iter()
                .filter(|c| server.clients.iter().any(|sc| sc.config_path == c.path))
                .map(|c| (c.clone(), true))
                .collect();

            if locs.is_empty() {
                self.status_message = Some(format!(
                    "Server '{}' is not installed in any client",
                    server.id
                ));
                return;
            }

            self.delete_state = Some(DeleteState {
                server_id: server.id,
                target_locations: locs,
                target_cursor: 0,
                remove_all_mode: true,
            });
            self.current_view = CurrentView::DeleteConfirm;
        }
    }

    pub fn start_delete_for_current_client(&mut self) {
        if let Some(client) = self.selected_client().cloned() {
            let servers_in_client: Vec<_> = self
                .servers
                .iter()
                .filter(|s| {
                    s.clients
                        .iter()
                        .any(|sc| sc.config_path == client.config_path)
                })
                .cloned()
                .collect();

            if servers_in_client.is_empty() {
                self.status_message =
                    Some(format!("No servers configured in {}", client.display_name));
                return;
            }

            if let Some(loc) = self
                .detected_clients
                .iter()
                .find(|l| l.path == client.config_path)
                .cloned()
            {
                let first_server = &servers_in_client[0];
                self.delete_state = Some(DeleteState {
                    server_id: first_server.id.clone(),
                    target_locations: vec![(loc, true)],
                    target_cursor: 0,
                    remove_all_mode: false,
                });
                self.current_view = CurrentView::DeleteConfirm;
            }
        }
    }

    pub fn confirm_delete(&mut self) -> Result<usize> {
        if let Some(state) = self.delete_state.take() {
            let targets: Vec<ConfigLocation> = if state.remove_all_mode {
                state.target_locations.into_iter().map(|(l, _)| l).collect()
            } else {
                state
                    .target_locations
                    .into_iter()
                    .filter(|(_, sel)| *sel)
                    .map(|(l, _)| l)
                    .collect()
            };

            let count = targets.len();
            if count > 0 {
                self.manager
                    .remove_server_from_locations(&state.server_id, &targets)?;
                self.refresh_servers();
                self.refresh_discovery();
                self.status_message = Some(format!(
                    "Removed server '{}' from {} client(s)",
                    state.server_id, count
                ));
            }
            self.current_view = CurrentView::Dashboard;
            Ok(count)
        } else {
            self.current_view = CurrentView::Dashboard;
            Ok(0)
        }
    }

    pub fn cancel_delete(&mut self) {
        self.delete_state = None;
        self.current_view = CurrentView::Dashboard;
    }

    pub fn toggle_delete_target(&mut self) {
        if let Some(ref mut state) = self.delete_state {
            if let Some((_, sel)) = state.target_locations.get_mut(state.target_cursor) {
                *sel = !*sel;
            }
        }
    }

    pub fn toggle_delete_all_targets(&mut self, select: bool) {
        if let Some(ref mut state) = self.delete_state {
            for (_, sel) in &mut state.target_locations {
                *sel = select;
            }
        }
    }

    pub fn select_next_delete_target(&mut self) {
        if let Some(ref mut state) = self.delete_state {
            if !state.target_locations.is_empty() {
                state.target_cursor = (state.target_cursor + 1) % state.target_locations.len();
            }
        }
    }

    pub fn select_prev_delete_target(&mut self) {
        if let Some(ref mut state) = self.delete_state {
            if !state.target_locations.is_empty() {
                if state.target_cursor == 0 {
                    state.target_cursor = state.target_locations.len() - 1;
                } else {
                    state.target_cursor -= 1;
                }
            }
        }
    }

    pub fn toggle_delete_mode(&mut self) {
        if let Some(ref mut state) = self.delete_state {
            state.remove_all_mode = !state.remove_all_mode;
        }
    }
}
