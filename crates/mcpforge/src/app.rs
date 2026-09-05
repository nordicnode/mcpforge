use anyhow::Result;
use mcp_core::types::{HealthStatus, Scope, ServerEntry, Transport};
use mcpforge_adapters::{AdapterManager, ConfigLocation, DiscoveredHarness, DiscoveryEngine};
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
    ViewSnippet,
    ToolExplorer,
    ToolOutputPager,
    BackupManager,
    ViewClientConfig,
}

#[derive(Debug, Clone)]
pub struct FormField {
    pub name: String,
    pub field_type: String,
    pub is_required: bool,
    pub description: String,
    pub value: String,
    pub enum_options: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BackupManagerState {
    pub backups: Vec<mcpforge_adapters::BackupInfo>,
    pub selected_index: usize,
    pub diff_preview: String,
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

#[derive(Debug, Clone)]
pub struct ToolExplorerState {
    pub server_id: String,
    pub tools: Vec<mcp_core::protocol::ToolDefinition>,
    pub selected_index: usize,
    pub is_loading: bool,
    pub execution_result: Option<String>,
    pub error_message: Option<String>,
    pub params_input: String,
    pub is_editing_params: bool,
    pub is_form_mode: bool,
    pub form_fields: Vec<FormField>,
    pub form_active_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusedPane {
    #[default]
    ServersList,
    ServerDetails,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetailTab {
    #[default]
    Overview,
    Clients,
    Environment,
    Telemetry,
    ConfigJson,
}

impl DetailTab {
    pub fn all() -> &'static [DetailTab] {
        &[
            DetailTab::Overview,
            DetailTab::Clients,
            DetailTab::Environment,
            DetailTab::Telemetry,
            DetailTab::ConfigJson,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            DetailTab::Overview => "Overview",
            DetailTab::Clients => "Clients",
            DetailTab::Environment => "Env",
            DetailTab::Telemetry => "Telemetry",
            DetailTab::ConfigJson => "Config JSON",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            DetailTab::Overview => 0,
            DetailTab::Clients => 1,
            DetailTab::Environment => 2,
            DetailTab::Telemetry => 3,
            DetailTab::ConfigJson => 4,
        }
    }

    pub fn from_index(idx: usize) -> Self {
        match idx % 5 {
            0 => DetailTab::Overview,
            1 => DetailTab::Clients,
            2 => DetailTab::Environment,
            3 => DetailTab::Telemetry,
            _ => DetailTab::ConfigJson,
        }
    }
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
    pub focused_pane: FocusedPane,
    pub detail_tab: DetailTab,
    pub detail_scroll: usize,
    pub wizard_state: Option<WizardState>,
    pub delete_state: Option<DeleteState>,
    pub tool_explorer_state: Option<ToolExplorerState>,
    pub backup_state: Option<BackupManagerState>,
    pub pager_scroll: usize,
    pub should_quit: bool,
    pub status_message: Option<String>,
    pub running_processes: std::collections::HashSet<String>,
    pub discovered_clients: Vec<DiscoveredHarness>,
    pub selected_client_index: usize,
    pub client_config_modal: Option<(String, String)>,
    pub client_config_scroll: usize,
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
            focused_pane: FocusedPane::ServersList,
            detail_tab: DetailTab::Overview,
            detail_scroll: 0,
            wizard_state: None,
            delete_state: None,
            tool_explorer_state: None,
            backup_state: None,
            pager_scroll: 0,
            should_quit: false,
            status_message: None,
            running_processes,
            discovered_clients,
            selected_client_index: 0,
            client_config_modal: None,
            client_config_scroll: 0,
        })
    }

    pub fn toggle_focus(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::ServersList => FocusedPane::ServerDetails,
            FocusedPane::ServerDetails => FocusedPane::ServersList,
        };
    }

    pub fn focus_details(&mut self) {
        self.focused_pane = FocusedPane::ServerDetails;
    }

    pub fn focus_servers(&mut self) {
        self.focused_pane = FocusedPane::ServersList;
    }

    pub fn generate_canonical_snippet(server: &ServerEntry) -> String {
        match &server.transport {
            mcp_core::types::Transport::Stdio { command, args, env } => {
                let mut val = serde_json::json!({
                    "command": command,
                    "args": args,
                });
                if !env.is_empty() {
                    val.as_object_mut().unwrap().insert(
                        "env".to_string(),
                        serde_json::to_value(env).unwrap_or_default(),
                    );
                }
                serde_json::to_string_pretty(&serde_json::json!({
                    server.id.clone(): val
                }))
                .unwrap_or_default()
            }
            mcp_core::types::Transport::StreamableHttp { url, headers } => {
                let mut val = serde_json::json!({
                    "url": url,
                });
                if !headers.is_empty() {
                    val.as_object_mut().unwrap().insert(
                        "headers".to_string(),
                        serde_json::to_value(headers).unwrap_or_default(),
                    );
                }
                serde_json::to_string_pretty(&serde_json::json!({
                    server.id.clone(): val
                }))
                .unwrap_or_default()
            }
            mcp_core::types::Transport::Sse { url } => {
                serde_json::to_string_pretty(&serde_json::json!({
                    server.id.clone(): {
                        "type": "sse",
                        "url": url,
                    }
                }))
                .unwrap_or_default()
            }
        }
    }

    pub fn set_detail_tab(&mut self, tab: DetailTab) {
        self.detail_tab = tab;
        self.detail_scroll = 0;
    }

    pub fn next_detail_tab(&mut self) {
        self.detail_tab = DetailTab::from_index(self.detail_tab.index() + 1);
        self.detail_scroll = 0;
    }

    pub fn prev_detail_tab(&mut self) {
        let idx = if self.detail_tab.index() == 0 {
            4
        } else {
            self.detail_tab.index() - 1
        };
        self.detail_tab = DetailTab::from_index(idx);
        self.detail_scroll = 0;
    }

    pub fn scroll_detail_down(&mut self, delta: usize) {
        self.detail_scroll = self.detail_scroll.saturating_add(delta);
    }

    pub fn scroll_detail_up(&mut self, delta: usize) {
        self.detail_scroll = self.detail_scroll.saturating_sub(delta);
    }

    pub fn select_next_tool(&mut self) {
        if let Some(ref mut s) = self.tool_explorer_state {
            if !s.tools.is_empty() && s.selected_index + 1 < s.tools.len() {
                s.selected_index += 1;
                s.execution_result = None;
                s.error_message = None;
                s.is_editing_params = false;
            }
        }
        self.update_params_for_selected_tool();
    }

    pub fn select_prev_tool(&mut self) {
        if let Some(ref mut s) = self.tool_explorer_state {
            if s.selected_index > 0 {
                s.selected_index -= 1;
                s.execution_result = None;
                s.error_message = None;
                s.is_editing_params = false;
            }
        }
        self.update_params_for_selected_tool();
    }

    pub fn update_params_for_selected_tool(&mut self) {
        if let Some(ref mut s) = self.tool_explorer_state {
            if let Some(tool) = s.tools.get(s.selected_index) {
                let def_val = generate_default_args(tool.input_schema.as_ref());
                s.params_input =
                    serde_json::to_string(&def_val).unwrap_or_else(|_| "{}".to_string());
                if s.is_form_mode {
                    s.form_fields = init_form_fields_from_schema(tool.input_schema.as_ref());
                    s.form_active_index = 0;
                }
            }
        }
    }

    pub fn toggle_form_mode(&mut self) {
        if let Some(ref mut s) = self.tool_explorer_state {
            s.is_form_mode = !s.is_form_mode;
            if s.is_form_mode {
                s.is_editing_params = false;
                if let Some(tool) = s.tools.get(s.selected_index) {
                    s.form_fields = init_form_fields_from_schema(tool.input_schema.as_ref());
                    s.form_active_index = 0;
                }
            } else {
                s.params_input = assemble_form_to_json(&s.form_fields);
            }
        }
    }

    pub fn form_next_field(&mut self) {
        if let Some(ref mut s) = self.tool_explorer_state {
            if !s.form_fields.is_empty() {
                s.form_active_index = (s.form_active_index + 1) % s.form_fields.len();
            }
        }
    }

    pub fn form_prev_field(&mut self) {
        if let Some(ref mut s) = self.tool_explorer_state {
            if !s.form_fields.is_empty() {
                if s.form_active_index == 0 {
                    s.form_active_index = s.form_fields.len() - 1;
                } else {
                    s.form_active_index -= 1;
                }
            }
        }
    }

    pub fn open_backup_manager(&mut self) {
        let backups = mcpforge_adapters::list_backups().unwrap_or_default();
        let mut state = BackupManagerState {
            backups,
            selected_index: 0,
            diff_preview: String::new(),
        };
        state.compute_diff();
        self.backup_state = Some(state);
        self.current_view = CurrentView::BackupManager;
    }

    pub fn select_next_backup(&mut self) {
        if let Some(ref mut s) = self.backup_state {
            if !s.backups.is_empty() {
                s.selected_index = (s.selected_index + 1) % s.backups.len();
                s.compute_diff();
            }
        }
    }

    pub fn select_prev_backup(&mut self) {
        if let Some(ref mut s) = self.backup_state {
            if !s.backups.is_empty() {
                if s.selected_index == 0 {
                    s.selected_index = s.backups.len() - 1;
                } else {
                    s.selected_index -= 1;
                }
                s.compute_diff();
            }
        }
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

    pub fn open_client_config_modal(&mut self) {
        if let Some(client) = self.selected_client() {
            if let Some(loc) = client.all_locations.iter().find(|l| l.exists) {
                if let Ok(content) = std::fs::read_to_string(&loc.path) {
                    self.client_config_modal = Some((loc.path.display().to_string(), content));
                    self.client_config_scroll = 0;
                    self.current_view = CurrentView::ViewClientConfig;
                } else {
                    self.status_message = Some(format!(
                        "Could not read configuration at {}",
                        loc.path.display()
                    ));
                }
            } else {
                self.status_message = Some(format!(
                    "No active configuration file on disk for {}",
                    client.display_name
                ));
            }
        }
    }

    pub fn close_client_config_modal(&mut self) {
        self.client_config_modal = None;
        self.client_config_scroll = 0;
        self.current_view = CurrentView::Clients;
    }

    pub fn scroll_client_config_down(&mut self, lines: usize) {
        if let Some((_, ref content)) = self.client_config_modal {
            let total = content.lines().count();
            if total > 0 {
                self.client_config_scroll =
                    (self.client_config_scroll + lines).min(total.saturating_sub(1));
            }
        }
    }

    pub fn scroll_client_config_up(&mut self, lines: usize) {
        self.client_config_scroll = self.client_config_scroll.saturating_sub(lines);
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
            self.detail_scroll = 0;
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
            self.detail_scroll = 0;
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
        let active_client_id = if self.current_view == CurrentView::Clients {
            self.selected_client().map(|c| c.id.clone())
        } else {
            None
        };

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
                let should_select = if let Some(ref target_id) = active_client_id {
                    &h.id == target_id
                } else {
                    h.is_running
                };
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

        let (
            source,
            registry_cursor,
            server_id,
            command_str,
            args_str,
            pasted_json,
            selected_targets,
        ) = {
            let wizard = match self.wizard_state {
                Some(ref w) => w,
                None => return,
            };

            let targets: Vec<ConfigLocation> = wizard
                .target_locations
                .iter()
                .filter(|(_, sel)| *sel)
                .map(|(loc, _)| loc.clone())
                .collect();

            (
                wizard.source,
                wizard.registry_cursor,
                wizard.server_id.clone(),
                wizard.command.clone(),
                wizard.args.clone(),
                wizard.pasted_json.clone(),
                targets,
            )
        };

        let new_server = match source {
            WizardSource::FromRegistry => {
                if registry_cursor < entries.len() {
                    let cat_entry = &entries[registry_cursor];
                    let (env, _) = crate::resolver::EnvResolver::new()
                        .resolve_for_keys(&cat_entry.required_env);
                    cat_entry.to_server_entry(env)
                } else {
                    return;
                }
            }
            WizardSource::Manual => {
                let args = args_str.split_whitespace().map(|s| s.to_string()).collect();
                ServerEntry::new_stdio(&server_id, &command_str, args, BTreeMap::new())
            }
            WizardSource::PasteJson => {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&pasted_json) {
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
                            ServerEntry::new_stdio(&server_id, cmd, args, BTreeMap::new())
                        } else {
                            if let Some(ref mut w) = self.wizard_state {
                                w.error_message =
                                    Some("JSON must have 'command' property".to_string());
                            }
                            return;
                        }
                    } else {
                        if let Some(ref mut w) = self.wizard_state {
                            w.error_message = Some("Invalid JSON object".to_string());
                        }
                        return;
                    }
                } else {
                    if let Some(ref mut w) = self.wizard_state {
                        w.error_message = Some("Failed to parse JSON".to_string());
                    }
                    return;
                }
            }
        };

        let mut diffs = String::new();
        for loc in &selected_targets {
            match self.manager.preview_diff_for_server(&new_server, loc) {
                Ok(d) if !d.is_empty() => {
                    diffs.push_str(&format!(
                        "--- Target: {} ({})\n",
                        loc.display_name,
                        loc.path.display()
                    ));
                    diffs.push_str(&d);
                    diffs.push('\n');
                }
                Ok(_) => {}
                Err(e) => {
                    diffs.push_str(&format!(
                        "--- Target: {} ({})\n[Error generating diff: {}]\n\n",
                        loc.display_name,
                        loc.path.display(),
                        e
                    ));
                }
            }
        }

        if diffs.is_empty() {
            diffs = "No changes or all configurations are already up-to-date.".to_string();
        }

        if let Some(ref mut wizard) = self.wizard_state {
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
                    let (env, _) = crate::resolver::EnvResolver::new()
                        .resolve_for_keys(&cat_entry.required_env);
                    cat_entry.to_server_entry(env)
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

pub fn generate_default_args(schema: Option<&serde_json::Value>) -> serde_json::Value {
    let schema = match schema {
        Some(s) => s,
        None => return serde_json::json!({}),
    };

    let props = match schema.get("properties").and_then(|p| p.as_object()) {
        Some(p) => p,
        None => return serde_json::json!({}),
    };

    let required: std::collections::HashSet<&str> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let mut map = serde_json::Map::new();

    for (name, spec) in props {
        // Only generate for required fields or if few properties exist
        if !required.is_empty() && !required.contains(name.as_str()) && props.len() > 3 {
            continue;
        }

        if let Some(def) = spec.get("default") {
            map.insert(name.clone(), def.clone());
            continue;
        }

        if let Some(enums) = spec.get("enum").and_then(|e| e.as_array()) {
            if let Some(first) = enums.first() {
                map.insert(name.clone(), first.clone());
                continue;
            }
        }

        let type_str = if let Some(t) = spec.get("type").and_then(|t| t.as_str()) {
            t
        } else if let Some(arr) = spec.get("type").and_then(|t| t.as_array()) {
            arr.first().and_then(|v| v.as_str()).unwrap_or("string")
        } else {
            "string"
        };

        let val = match type_str {
            "string" => {
                let lower = name.to_lowercase();
                if lower.contains("path") || lower.contains("file") {
                    serde_json::Value::String("/tmp/test.txt".to_string())
                } else if lower.contains("url") || lower.contains("uri") {
                    serde_json::Value::String("https://example.com".to_string())
                } else if lower.contains("thought") {
                    serde_json::Value::String("Initial reasoning step".to_string())
                } else if lower.contains("query") || lower.contains("search") {
                    serde_json::Value::String("test query".to_string())
                } else if lower.contains("name") {
                    serde_json::Value::String("test_item".to_string())
                } else {
                    serde_json::Value::String("test".to_string())
                }
            }
            "integer" | "number" => {
                let min = spec.get("minimum").and_then(|m| m.as_i64()).unwrap_or(1);
                serde_json::Value::Number(serde_json::Number::from(min))
            }
            "boolean" => {
                let lower = name.to_lowercase();
                if lower.contains("needed") || lower.contains("enable") {
                    serde_json::Value::Bool(false)
                } else {
                    serde_json::Value::Bool(true)
                }
            }
            "array" => serde_json::Value::Array(Vec::new()),
            "object" => serde_json::Value::Object(serde_json::Map::new()),
            _ => serde_json::Value::String("test".to_string()),
        };

        map.insert(name.clone(), val);
    }

    serde_json::Value::Object(map)
}

impl BackupManagerState {
    pub fn compute_diff(&mut self) {
        if let Some(b) = self.backups.get(self.selected_index) {
            let backup_content = std::fs::read_to_string(&b.backup_path).unwrap_or_default();
            let current_content = if b.target_path.exists() {
                std::fs::read_to_string(&b.target_path).unwrap_or_default()
            } else {
                String::new()
            };
            self.diff_preview = mcpforge_adapters::compute_diff(
                &backup_content,
                &current_content,
                &b.original_file,
            );
        } else {
            self.diff_preview = "No backup snapshots available.".to_string();
        }
    }
}

pub fn init_form_fields_from_schema(schema: Option<&serde_json::Value>) -> Vec<FormField> {
    let schema = match schema {
        Some(s) => s,
        None => return Vec::new(),
    };

    let props = match schema.get("properties").and_then(|p| p.as_object()) {
        Some(p) => p,
        None => return Vec::new(),
    };

    let required: std::collections::HashSet<&str> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let mut fields = Vec::new();

    for (name, spec) in props {
        let is_required = required.contains(name.as_str());
        let desc = spec
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        let field_type = if let Some(t) = spec.get("type").and_then(|t| t.as_str()) {
            t.to_string()
        } else if let Some(arr) = spec.get("type").and_then(|t| t.as_array()) {
            arr.first()
                .and_then(|v| v.as_str())
                .unwrap_or("string")
                .to_string()
        } else {
            "string".to_string()
        };

        let mut enum_options = Vec::new();
        if let Some(arr) = spec.get("enum").and_then(|e| e.as_array()) {
            for item in arr {
                if let Some(s) = item.as_str() {
                    enum_options.push(s.to_string());
                }
            }
        }

        let initial_val = if let Some(def) = spec.get("default") {
            match def {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            }
        } else if let Some(first) = enum_options.first() {
            first.clone()
        } else {
            match field_type.as_str() {
                "boolean" => "false".to_string(),
                "integer" | "number" => {
                    let min = spec.get("minimum").and_then(|m| m.as_i64()).unwrap_or(1);
                    min.to_string()
                }
                _ => {
                    let lower = name.to_lowercase();
                    if lower.contains("path") || lower.contains("file") {
                        "/tmp/test.txt".to_string()
                    } else if lower.contains("url") || lower.contains("uri") {
                        "https://example.com".to_string()
                    } else if lower.contains("thought") {
                        "Initial reasoning step".to_string()
                    } else if is_required {
                        "test".to_string()
                    } else {
                        "".to_string()
                    }
                }
            }
        };

        fields.push(FormField {
            name: name.clone(),
            field_type,
            is_required,
            description: desc,
            value: initial_val,
            enum_options,
        });
    }

    fields.sort_by(|a, b| {
        b.is_required
            .cmp(&a.is_required)
            .then_with(|| a.name.cmp(&b.name))
    });

    fields
}

pub fn assemble_form_to_json(fields: &[FormField]) -> String {
    let mut map = serde_json::Map::new();

    for f in fields {
        let val_trim = f.value.trim();
        if val_trim.is_empty() && !f.is_required {
            continue;
        }

        let json_val = match f.field_type.as_str() {
            "boolean" => serde_json::Value::Bool(f.value.trim().eq_ignore_ascii_case("true")),
            "integer" => {
                let n: i64 = f.value.trim().parse().unwrap_or(1);
                serde_json::Value::Number(serde_json::Number::from(n))
            }
            "number" => {
                let n: f64 = f.value.trim().parse().unwrap_or(1.0);
                serde_json::Number::from_f64(n)
                    .map(serde_json::Value::Number)
                    .unwrap_or_else(|| serde_json::Value::Number(serde_json::Number::from(1)))
            }
            "array" => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(val_trim) {
                    v
                } else {
                    serde_json::Value::Array(Vec::new())
                }
            }
            "object" => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(val_trim) {
                    v
                } else {
                    serde_json::Value::Object(serde_json::Map::new())
                }
            }
            _ => serde_json::Value::String(f.value.clone()),
        };

        map.insert(f.name.clone(), json_val);
    }

    serde_json::to_string(&serde_json::Value::Object(map)).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deepseek_diff() {
        let mut app = App::new().unwrap();
        app.start_wizard();
        if let Some(ref mut w) = app.wizard_state {
            for (loc, sel) in &mut w.target_locations {
                if loc.client_id == "deepseek" {
                    *sel = true;
                }
            }
            w.registry_cursor = 25; // different server
        }
        app.compute_wizard_diff();
        if let Some(ref w) = app.wizard_state {
            println!("=== DIFF RESULT ===\n{}", w.diff_preview);
            assert!(w.diff_preview.contains("--- Target: DeepSeek Harness"));
            assert!(!w.diff_preview.contains("-      \"command\": \"npx\","));
            assert!(!w.diff_preview.contains("-      \"args\": ["));
        }
    }

    #[test]
    fn test_generate_default_args_for_sequentialthinking() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "thought": { "type": "string", "description": "Your current thinking step" },
                "nextThoughtNeeded": { "type": ["boolean", "string"] },
                "thoughtNumber": { "type": "integer", "minimum": 1 },
                "totalThoughts": { "type": "integer", "minimum": 1 }
            },
            "required": ["thought", "nextThoughtNeeded", "thoughtNumber", "totalThoughts"]
        });

        let args = generate_default_args(Some(&schema));
        assert!(args.get("thought").unwrap().is_string());
        assert!(args.get("nextThoughtNeeded").unwrap().is_boolean());
        assert_eq!(args.get("thoughtNumber").unwrap().as_i64(), Some(1));
        assert_eq!(args.get("totalThoughts").unwrap().as_i64(), Some(1));
    }

    #[test]
    fn test_form_builder_schema_roundtrip() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "thought": { "type": "string", "description": "Thinking step" },
                "nextThoughtNeeded": { "type": "boolean" },
                "thoughtNumber": { "type": "integer", "minimum": 1 }
            },
            "required": ["thought", "nextThoughtNeeded"]
        });

        let mut fields = init_form_fields_from_schema(Some(&schema));
        assert_eq!(fields.len(), 3);
        // Required fields first
        assert!(fields[0].is_required);
        assert!(fields[1].is_required);

        // Mutate a field
        for f in &mut fields {
            if f.name == "thought" {
                f.value = "Custom deep thought".to_string();
            }
        }

        let json_str = assemble_form_to_json(&fields);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            parsed.get("thought").and_then(|v| v.as_str()),
            Some("Custom deep thought")
        );
        assert_eq!(
            parsed.get("nextThoughtNeeded").and_then(|v| v.as_bool()),
            Some(false)
        );
    }
}
