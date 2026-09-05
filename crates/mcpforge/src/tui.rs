use crate::app::{App, CurrentView, WizardSource, WizardStep};
use crate::ui;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use mcp_core::client::check_server_health;
use mcpforge_adapters::ConfigLocation;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{stdout, Write};
use std::time::Duration;

pub async fn run(run_doctor_init: bool) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;

    if run_doctor_init && !app.servers.is_empty() {
        for server in &app.servers {
            let status = check_server_health(server, 5).await;
            app.health_cache.insert(server.id.clone(), status);
        }
    }

    let res = main_loop(&mut terminal, &mut app).await;

    // Restore terminal cleanly
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("Application error: {:#}", e);
    }

    Ok(())
}

async fn main_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::render_ui(f, app))?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                if app.is_searching {
                    match key.code {
                        KeyCode::Esc => {
                            app.is_searching = false;
                            app.search_query.clear();
                        }
                        KeyCode::Enter => {
                            app.is_searching = false;
                        }
                        KeyCode::Backspace => {
                            app.search_query.pop();
                        }
                        KeyCode::Char(c) => {
                            app.search_query.push(c);
                        }
                        _ => {}
                    }
                    continue;
                }

                match app.current_view {
                    CurrentView::Dashboard => {
                        if app.focused_pane == crate::app::FocusedPane::ServerDetails {
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left => {
                                    app.focus_servers();
                                }
                                KeyCode::Char('1') => {
                                    app.set_detail_tab(crate::app::DetailTab::Overview);
                                }
                                KeyCode::Char('2') => {
                                    app.set_detail_tab(crate::app::DetailTab::Clients);
                                }
                                KeyCode::Char('3') => {
                                    app.set_detail_tab(crate::app::DetailTab::Environment);
                                }
                                KeyCode::Char('4') => {
                                    app.set_detail_tab(crate::app::DetailTab::Telemetry);
                                }
                                KeyCode::Char('5') => {
                                    app.set_detail_tab(crate::app::DetailTab::ConfigJson);
                                }
                                KeyCode::Tab | KeyCode::Char(']') => {
                                    app.next_detail_tab();
                                }
                                KeyCode::BackTab | KeyCode::Char('[') => {
                                    app.prev_detail_tab();
                                }
                                KeyCode::Char('j') | KeyCode::Down => {
                                    app.scroll_detail_down(1);
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    app.scroll_detail_up(1);
                                }
                                KeyCode::PageDown => {
                                    app.scroll_detail_down(8);
                                }
                                KeyCode::PageUp => {
                                    app.scroll_detail_up(8);
                                }
                                KeyCode::Char('c') => {
                                    if let Some(server) = app.selected_server() {
                                        let snippet = App::generate_canonical_snippet(server);
                                        let b64 = base64_encode(snippet.as_bytes());
                                        let osc52 = format!("\x1b]52;c;{}\x07", b64);
                                        let _ = std::io::stdout().write_all(osc52.as_bytes());
                                        let _ = std::io::stdout().flush();
                                        app.status_message = Some(format!(
                                            "Copied '{}' configuration to clipboard!",
                                            server.id
                                        ));
                                    }
                                }
                                KeyCode::Char('t') => {
                                    if let Some(mut server) = app.selected_server().cloned() {
                                        let resolver = crate::resolver::EnvResolver::new();
                                        resolver.enrich_server_entry(&mut server, &app.registry);

                                        app.tool_explorer_state =
                                            Some(crate::app::ToolExplorerState {
                                                server_id: server.id.clone(),
                                                tools: Vec::new(),
                                                selected_index: 0,
                                                is_loading: true,
                                                execution_result: None,
                                                error_message: None,
                                                params_input: "{}".to_string(),
                                                is_editing_params: false,
                                                is_form_mode: false,
                                                form_fields: Vec::new(),
                                                form_active_index: 0,
                                            });
                                        app.current_view = CurrentView::ToolExplorer;
                                        terminal.draw(|f| ui::render_ui(f, app))?;

                                        match mcp_core::client::list_server_tools(&server, 10).await
                                        {
                                            Ok(tools) => {
                                                if let Some(ref mut state) = app.tool_explorer_state
                                                {
                                                    state.is_loading = false;
                                                    state.tools = tools;
                                                }
                                                app.update_params_for_selected_tool();
                                            }
                                            Err(e) => {
                                                if let Some(ref mut state) = app.tool_explorer_state
                                                {
                                                    state.is_loading = false;
                                                    state.error_message = Some(e.to_string());
                                                }
                                            }
                                        }
                                    }
                                }
                                KeyCode::Char('T') => {
                                    if let Some(mut server) = app.selected_server().cloned() {
                                        let resolver = crate::resolver::EnvResolver::new();
                                        resolver.enrich_server_entry(&mut server, &app.registry);

                                        app.status_message = Some(format!(
                                            "Handshake test: Spawning & negotiating with '{}'...",
                                            server.id
                                        ));
                                        terminal.draw(|f| ui::render_ui(f, app))?;
                                        let start = std::time::Instant::now();
                                        let status = check_server_health(&server, 8).await;
                                        let elapsed_ms = start.elapsed().as_millis();
                                        app.health_cache.insert(server.id.clone(), status.clone());
                                        match status {
                                            mcp_core::types::HealthStatus::Healthy {
                                                tool_count,
                                                server_name,
                                                server_version,
                                                ..
                                            } => {
                                                app.status_message = Some(format!(
                                                    "✓ Handshake SUCCESS: {} v{} ({} tools, {}ms)",
                                                    server_name,
                                                    server_version,
                                                    tool_count,
                                                    elapsed_ms
                                                ));
                                            }
                                            mcp_core::types::HealthStatus::Degraded {
                                                reason,
                                                ..
                                            } => {
                                                app.status_message = Some(format!(
                                                    "▲ Handshake DEGRADED: {} ({}ms)",
                                                    reason, elapsed_ms
                                                ));
                                            }
                                            mcp_core::types::HealthStatus::Broken { error } => {
                                                app.status_message = Some(format!(
                                                    "✖ Handshake FAILED: {} ({}ms)",
                                                    error, elapsed_ms
                                                ));
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                KeyCode::Char(' ') => {
                                    if let Some(mut server) = app.selected_server().cloned() {
                                        server.enabled = !server.enabled;
                                        let locs: Vec<ConfigLocation> = app
                                            .detected_clients
                                            .iter()
                                            .filter(|c| {
                                                server
                                                    .clients
                                                    .iter()
                                                    .any(|sc| sc.config_path == c.path)
                                            })
                                            .cloned()
                                            .collect();
                                        let _ =
                                            app.manager.write_server_to_locations(&server, &locs);
                                        app.refresh_servers();
                                    }
                                }
                                KeyCode::Char('a') => {
                                    app.start_wizard();
                                }
                                KeyCode::Char('q') => {
                                    app.should_quit = true;
                                }
                                KeyCode::Char('?') => {
                                    app.current_view = CurrentView::Help;
                                }
                                _ => {}
                            }
                        } else {
                            match key.code {
                                KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => {
                                    if app.selected_server().is_some() {
                                        app.focus_details();
                                    }
                                }
                                KeyCode::Tab | KeyCode::BackTab | KeyCode::Char('2') => {
                                    app.refresh_discovery();
                                    app.current_view = CurrentView::Clients;
                                }
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    app.should_quit = true;
                                }
                                KeyCode::Char('?') => {
                                    app.current_view = CurrentView::Help;
                                }
                                KeyCode::Char('j') | KeyCode::Down => {
                                    app.select_next();
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    app.select_prev();
                                }
                                KeyCode::Char('/') => {
                                    app.is_searching = true;
                                }
                                KeyCode::Char('r') => {
                                    if let Some(server) = app.selected_server().cloned() {
                                        app.status_message =
                                            Some(format!("Checking health for '{}'...", server.id));
                                        terminal.draw(|f| ui::render_ui(f, app))?;
                                        let status = check_server_health(&server, 5).await;
                                        app.health_cache.insert(server.id.clone(), status);
                                        app.status_message =
                                            Some(format!("Updated health for '{}'", server.id));
                                    }
                                }
                                KeyCode::Char('u') => {
                                    let count = app.auto_sync_all().unwrap_or(0);
                                    app.status_message = Some(format!(
                                        "Auto-synced {} servers across all clients!",
                                        count
                                    ));
                                }
                                KeyCode::Char('a') => {
                                    app.start_wizard();
                                }
                                KeyCode::Char('d')
                                | KeyCode::Delete
                                | KeyCode::Backspace
                                | KeyCode::Char('x') => {
                                    app.start_delete();
                                }
                                KeyCode::Char(' ') => {
                                    if let Some(mut server) = app.selected_server().cloned() {
                                        server.enabled = !server.enabled;
                                        let locs: Vec<ConfigLocation> = app
                                            .detected_clients
                                            .iter()
                                            .filter(|c| {
                                                server
                                                    .clients
                                                    .iter()
                                                    .any(|sc| sc.config_path == c.path)
                                            })
                                            .cloned()
                                            .collect();
                                        let _ =
                                            app.manager.write_server_to_locations(&server, &locs);
                                        app.refresh_servers();
                                    }
                                }
                                KeyCode::Char('t') => {
                                    if let Some(mut server) = app.selected_server().cloned() {
                                        let resolver = crate::resolver::EnvResolver::new();
                                        resolver.enrich_server_entry(&mut server, &app.registry);

                                        app.tool_explorer_state =
                                            Some(crate::app::ToolExplorerState {
                                                server_id: server.id.clone(),
                                                tools: Vec::new(),
                                                selected_index: 0,
                                                is_loading: true,
                                                execution_result: None,
                                                error_message: None,
                                                params_input: "{}".to_string(),
                                                is_editing_params: false,
                                                is_form_mode: false,
                                                form_fields: Vec::new(),
                                                form_active_index: 0,
                                            });
                                        app.current_view = CurrentView::ToolExplorer;
                                        terminal.draw(|f| ui::render_ui(f, app))?;

                                        match mcp_core::client::list_server_tools(&server, 10).await
                                        {
                                            Ok(tools) => {
                                                if let Some(ref mut state) = app.tool_explorer_state
                                                {
                                                    state.is_loading = false;
                                                    state.tools = tools;
                                                }
                                                app.update_params_for_selected_tool();
                                            }
                                            Err(e) => {
                                                if let Some(ref mut state) = app.tool_explorer_state
                                                {
                                                    state.is_loading = false;
                                                    state.error_message = Some(e.to_string());
                                                }
                                            }
                                        }
                                    }
                                }
                                KeyCode::Char('T') => {
                                    if let Some(mut server) = app.selected_server().cloned() {
                                        let resolver = crate::resolver::EnvResolver::new();
                                        resolver.enrich_server_entry(&mut server, &app.registry);

                                        app.status_message = Some(format!(
                                            "Handshake test: Spawning & negotiating with '{}'...",
                                            server.id
                                        ));
                                        terminal.draw(|f| ui::render_ui(f, app))?;
                                        let start = std::time::Instant::now();
                                        let status = check_server_health(&server, 8).await;
                                        let elapsed_ms = start.elapsed().as_millis();
                                        app.health_cache.insert(server.id.clone(), status.clone());
                                        match status {
                                            mcp_core::types::HealthStatus::Healthy {
                                                tool_count,
                                                server_name,
                                                server_version,
                                                ..
                                            } => {
                                                app.status_message = Some(format!(
                                                    "✓ Handshake SUCCESS: {} v{} ({} tools, {}ms)",
                                                    server_name,
                                                    server_version,
                                                    tool_count,
                                                    elapsed_ms
                                                ));
                                            }
                                            mcp_core::types::HealthStatus::Degraded {
                                                reason,
                                                ..
                                            } => {
                                                app.status_message = Some(format!(
                                                    "▲ Handshake DEGRADED: {} ({}ms)",
                                                    reason, elapsed_ms
                                                ));
                                            }
                                            mcp_core::types::HealthStatus::Broken { error } => {
                                                app.status_message = Some(format!(
                                                    "✖ Handshake FAILED: {} ({}ms)",
                                                    error, elapsed_ms
                                                ));
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                KeyCode::Char('b') => {
                                    app.open_backup_manager();
                                }
                                KeyCode::Char('v') if app.selected_server().is_some() => {
                                    app.current_view = CurrentView::ViewSnippet;
                                }
                                _ => {}
                            }
                        }
                    }

                    CurrentView::Clients => match key.code {
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                        }
                        KeyCode::Esc | KeyCode::Tab | KeyCode::BackTab | KeyCode::Char('1') => {
                            app.current_view = CurrentView::Dashboard;
                        }
                        KeyCode::Char('?') => {
                            app.current_view = CurrentView::Help;
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.select_next_client();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.select_prev_client();
                        }
                        KeyCode::Char('r') => {
                            app.refresh_discovery();
                            app.status_message =
                                Some("Refreshed client & process discovery".to_string());
                        }
                        KeyCode::Char('u') => {
                            if let Some(client) = app.selected_client().cloned() {
                                let all_servers =
                                    app.manager.read_all_servers().unwrap_or_default();
                                if let Some(loc) = app
                                    .detected_clients
                                    .iter()
                                    .find(|l| l.path == client.config_path)
                                    .cloned()
                                {
                                    for s in &all_servers {
                                        let _ = app.manager.write_server_to_locations(
                                            s,
                                            std::slice::from_ref(&loc),
                                        );
                                    }
                                    app.refresh_discovery();
                                    app.status_message = Some(format!(
                                        "Synced {} servers into {}",
                                        all_servers.len(),
                                        client.display_name
                                    ));
                                }
                            }
                        }
                        KeyCode::Char('a') => {
                            app.start_wizard();
                        }
                        KeyCode::Char('v') => {
                            app.open_client_config_modal();
                        }
                        KeyCode::Char('m') => {
                            if let Some(client) = app.selected_client().cloned() {
                                app.status_message = Some(format!(
                                    "Testing matrix compatibility for '{}' (110 catalog servers)...",
                                    client.display_name
                                ));
                                terminal.draw(|f| ui::render_ui(f, app))?;
                                let verifier = crate::matrix::MatrixVerifier::new();
                                match verifier.run_matrix_audit(Some(&client.id)) {
                                    Ok(report) => {
                                        if report.is_success() {
                                            app.status_message = Some(format!(
                                                "✓ Matrix SUCCESS: 110/110 servers passed for '{}' ({}ms)",
                                                client.display_name, report.elapsed_ms
                                            ));
                                        } else {
                                            app.status_message = Some(format!(
                                                "✖ Matrix FAILED for '{}': {} failures detected",
                                                client.display_name,
                                                report.failures.len()
                                            ));
                                        }
                                    }
                                    Err(e) => {
                                        app.status_message = Some(format!(
                                            "✖ Matrix error for '{}': {:#}",
                                            client.display_name, e
                                        ));
                                    }
                                }
                            }
                        }
                        KeyCode::Char('d')
                        | KeyCode::Delete
                        | KeyCode::Backspace
                        | KeyCode::Char('x') => {
                            app.start_delete_for_current_client();
                        }
                        _ => {}
                    },

                    CurrentView::AddWizard => {
                        let mut step_action = None;
                        let mut cat_delta = 0i32;
                        let mut cat_set = None;
                        let mut item_delta = 0i32;

                        if let Some(ref mut wizard) = app.wizard_state {
                            match wizard.step {
                                WizardStep::SelectSource => match key.code {
                                    KeyCode::Up | KeyCode::Char('k') => {
                                        wizard.source = match wizard.source {
                                            WizardSource::FromRegistry => WizardSource::Manual,
                                            WizardSource::PasteJson => WizardSource::FromRegistry,
                                            WizardSource::Manual => WizardSource::PasteJson,
                                        };
                                    }
                                    KeyCode::Down | KeyCode::Char('j') => {
                                        wizard.source = match wizard.source {
                                            WizardSource::FromRegistry => WizardSource::PasteJson,
                                            WizardSource::PasteJson => WizardSource::Manual,
                                            WizardSource::Manual => WizardSource::FromRegistry,
                                        };
                                    }
                                    KeyCode::Enter => {
                                        step_action = Some(WizardStep::ConfigureServer);
                                    }
                                    KeyCode::Esc => {
                                        app.current_view = CurrentView::Dashboard;
                                        app.wizard_state = None;
                                    }
                                    _ => {}
                                },

                                WizardStep::ConfigureServer => match wizard.source {
                                    WizardSource::FromRegistry => match key.code {
                                        KeyCode::Up | KeyCode::Char('k') => {
                                            item_delta = -1;
                                        }
                                        KeyCode::Down | KeyCode::Char('j') => {
                                            item_delta = 1;
                                        }
                                        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                                            cat_delta = 1;
                                        }
                                        KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                                            cat_delta = -1;
                                        }
                                        KeyCode::Char(c) if ('1'..='8').contains(&c) => {
                                            cat_set = Some((c as usize) - ('1' as usize));
                                        }
                                        KeyCode::Enter => {
                                            step_action = Some(WizardStep::SelectTargets);
                                        }
                                        KeyCode::Esc => {
                                            step_action = Some(WizardStep::SelectSource);
                                        }
                                        _ => {}
                                    },
                                    _ => match key.code {
                                        KeyCode::Enter => {
                                            step_action = Some(WizardStep::SelectTargets);
                                        }
                                        KeyCode::Esc => {
                                            step_action = Some(WizardStep::SelectSource);
                                        }
                                        _ => {}
                                    },
                                },

                                WizardStep::SelectTargets => match key.code {
                                    KeyCode::Up | KeyCode::Char('k') => {
                                        if wizard.target_cursor > 0 {
                                            wizard.target_cursor -= 1;
                                        }
                                    }
                                    KeyCode::Down | KeyCode::Char('j') => {
                                        if wizard.target_cursor + 1 < wizard.target_locations.len()
                                        {
                                            wizard.target_cursor += 1;
                                        }
                                    }
                                    KeyCode::Char(' ') => {
                                        if let Some((_, selected)) =
                                            wizard.target_locations.get_mut(wizard.target_cursor)
                                        {
                                            *selected = !*selected;
                                        }
                                    }
                                    KeyCode::Char('a') => {
                                        for (_, selected) in &mut wizard.target_locations {
                                            *selected = true;
                                        }
                                    }
                                    KeyCode::Char('n') => {
                                        for (_, selected) in &mut wizard.target_locations {
                                            *selected = false;
                                        }
                                    }
                                    KeyCode::Enter => {
                                        step_action = Some(WizardStep::PreviewDiff);
                                    }
                                    KeyCode::Esc => {
                                        step_action = Some(WizardStep::ConfigureServer);
                                    }
                                    _ => {}
                                },

                                WizardStep::PreviewDiff => match key.code {
                                    KeyCode::Enter => {
                                        let _ = app.apply_wizard();
                                    }
                                    KeyCode::Esc => {
                                        step_action = Some(WizardStep::SelectTargets);
                                    }
                                    _ => {}
                                },
                            }
                        }

                        if cat_delta == 1 {
                            app.next_registry_category();
                        } else if cat_delta == -1 {
                            app.prev_registry_category();
                        } else if let Some(idx) = cat_set {
                            app.set_registry_category(idx);
                        }

                        if item_delta == 1 {
                            app.next_registry_item();
                        } else if item_delta == -1 {
                            app.prev_registry_item();
                        }

                        if let Some(next_step) = step_action {
                            if let Some(ref mut wizard) = app.wizard_state {
                                wizard.step = next_step;
                            }
                            if next_step == WizardStep::PreviewDiff {
                                app.compute_wizard_diff();
                            }
                        }
                    }

                    CurrentView::Help => match key.code {
                        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                            app.current_view = CurrentView::Dashboard;
                        }
                        _ => {}
                    },

                    CurrentView::DeleteConfirm => match key.code {
                        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                            let _ = app.confirm_delete();
                        }
                        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                            app.cancel_delete();
                        }
                        KeyCode::Tab | KeyCode::Char('m') => {
                            app.toggle_delete_mode();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.select_prev_delete_target();
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.select_next_delete_target();
                        }
                        KeyCode::Char(' ') => {
                            app.toggle_delete_target();
                        }
                        KeyCode::Char('a') => {
                            app.toggle_delete_all_targets(true);
                        }
                        KeyCode::Char('c') => {
                            app.toggle_delete_all_targets(false);
                        }
                        _ => {}
                    },
                    CurrentView::ViewSnippet => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('v') | KeyCode::Enter => {
                            app.current_view = CurrentView::Dashboard;
                        }
                        _ => {}
                    },
                    CurrentView::ToolExplorer => {
                        let is_form = app
                            .tool_explorer_state
                            .as_ref()
                            .is_some_and(|s| s.is_form_mode);

                        let is_editing = app
                            .tool_explorer_state
                            .as_ref()
                            .is_some_and(|s| s.is_editing_params);

                        if is_form {
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('f') => {
                                    app.toggle_form_mode();
                                    continue;
                                }
                                KeyCode::Tab | KeyCode::Down => {
                                    app.form_next_field();
                                    continue;
                                }
                                KeyCode::BackTab | KeyCode::Up => {
                                    app.form_prev_field();
                                    continue;
                                }
                                KeyCode::Char(' ') => {
                                    if let Some(ref mut s) = app.tool_explorer_state {
                                        if let Some(f) = s.form_fields.get_mut(s.form_active_index)
                                        {
                                            if f.field_type == "boolean" {
                                                if f.value == "true" {
                                                    f.value = "false".to_string();
                                                } else {
                                                    f.value = "true".to_string();
                                                }
                                            } else {
                                                f.value.push(' ');
                                            }
                                        }
                                    }
                                    continue;
                                }
                                KeyCode::Backspace => {
                                    if let Some(ref mut s) = app.tool_explorer_state {
                                        if let Some(f) = s.form_fields.get_mut(s.form_active_index)
                                        {
                                            f.value.pop();
                                        }
                                    }
                                    continue;
                                }
                                KeyCode::Char(c) => {
                                    if let Some(ref mut s) = app.tool_explorer_state {
                                        if let Some(f) = s.form_fields.get_mut(s.form_active_index)
                                        {
                                            if f.field_type != "boolean" {
                                                f.value.push(c);
                                            }
                                        }
                                    }
                                    continue;
                                }
                                KeyCode::Enter => {
                                    if let Some(ref mut s) = app.tool_explorer_state {
                                        s.params_input =
                                            crate::app::assemble_form_to_json(&s.form_fields);
                                    }
                                }
                                _ => continue,
                            }
                        } else if is_editing {
                            match key.code {
                                KeyCode::Esc => {
                                    if let Some(ref mut s) = app.tool_explorer_state {
                                        s.is_editing_params = false;
                                    }
                                }
                                KeyCode::Backspace => {
                                    if let Some(ref mut s) = app.tool_explorer_state {
                                        s.params_input.pop();
                                    }
                                }
                                KeyCode::Char(c) => {
                                    if let Some(ref mut s) = app.tool_explorer_state {
                                        s.params_input.push(c);
                                    }
                                }
                                KeyCode::Enter => {
                                    if let Some(ref mut s) = app.tool_explorer_state {
                                        s.is_editing_params = false;
                                    }
                                }
                                _ => {}
                            }
                            if key.code != KeyCode::Enter {
                                continue;
                            }
                        }

                        match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => {
                                app.current_view = CurrentView::Dashboard;
                                app.tool_explorer_state = None;
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                app.select_next_tool();
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                app.select_prev_tool();
                            }
                            KeyCode::Char('f') => {
                                app.toggle_form_mode();
                            }
                            KeyCode::Char('v') => {
                                if let Some(ref s) = app.tool_explorer_state {
                                    if s.execution_result.is_some() || s.error_message.is_some() {
                                        app.current_view = CurrentView::ToolOutputPager;
                                        app.pager_scroll = 0;
                                    }
                                }
                            }
                            KeyCode::Char('e') => {
                                if let Some(ref mut s) = app.tool_explorer_state {
                                    s.is_editing_params = true;
                                }
                            }
                            KeyCode::Char('r') => {
                                app.update_params_for_selected_tool();
                            }
                            KeyCode::Enter => {
                                if let (Some(server), Some(ref state)) =
                                    (app.selected_server().cloned(), &app.tool_explorer_state)
                                {
                                    if let Some(tool) = state.tools.get(state.selected_index) {
                                        let tool_name = tool.name.clone();
                                        let raw_params = state.params_input.clone();

                                        let parsed_args: serde_json::Value =
                                            match serde_json::from_str(&raw_params) {
                                                Ok(val) => val,
                                                Err(err) => {
                                                    if let Some(ref mut s) = app.tool_explorer_state
                                                    {
                                                        s.error_message = Some(format!(
                                                            "Invalid JSON parameters: {}",
                                                            err
                                                        ));
                                                        s.execution_result = None;
                                                    }
                                                    continue;
                                                }
                                            };

                                        if let Some(ref mut s) = app.tool_explorer_state {
                                            s.execution_result =
                                                Some(format!("Executing tool '{}'...", tool_name));
                                            s.error_message = None;
                                        }
                                        terminal.draw(|f| ui::render_ui(f, app))?;

                                        let mut enriched = server.clone();
                                        let resolver = crate::resolver::EnvResolver::new();
                                        resolver.enrich_server_entry(&mut enriched, &app.registry);

                                        let call_res = mcp_core::client::call_server_tool(
                                            &enriched,
                                            &tool_name,
                                            parsed_args,
                                            15,
                                        )
                                        .await;

                                        if let Some(ref mut s) = app.tool_explorer_state {
                                            match call_res {
                                                Ok(res) => {
                                                    let mut text_out = String::new();
                                                    for c in &res.content {
                                                        if let Some(ref t) = c.text {
                                                            text_out.push_str(t);
                                                            text_out.push('\n');
                                                        } else if let Some(ref d) = c.data {
                                                            text_out.push_str(&format!(
                                                                "[Binary data: {} bytes]\n",
                                                                d.len()
                                                            ));
                                                        }
                                                    }
                                                    if text_out.trim().is_empty() {
                                                        text_out = "✓ Tool call completed with empty output.".to_string();
                                                    }
                                                    s.execution_result = Some(text_out);
                                                }
                                                Err(e) => {
                                                    s.error_message = Some(e.to_string());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    CurrentView::ToolOutputPager => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.current_view = CurrentView::ToolExplorer;
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.pager_scroll = app.pager_scroll.saturating_add(1);
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.pager_scroll = app.pager_scroll.saturating_sub(1);
                        }
                        KeyCode::PageDown => {
                            app.pager_scroll = app.pager_scroll.saturating_add(15);
                        }
                        KeyCode::PageUp => {
                            app.pager_scroll = app.pager_scroll.saturating_sub(15);
                        }
                        KeyCode::Char('g') => {
                            app.pager_scroll = 0;
                        }
                        KeyCode::Char('G') => {
                            app.pager_scroll = usize::MAX / 2;
                        }
                        KeyCode::Char('c') => {
                            if let Some(ref s) = app.tool_explorer_state {
                                let text = s
                                    .execution_result
                                    .as_deref()
                                    .or(s.error_message.as_deref())
                                    .unwrap_or("");
                                use std::io::Write;
                                let b64 = base64_encode(text.as_bytes());
                                let osc52 = format!("\x1b]52;c;{}\x07", b64);
                                let _ = std::io::stdout().write_all(osc52.as_bytes());
                                let _ = std::io::stdout().flush();
                                app.status_message = Some(
                                    "✓ Output copied to system clipboard via OSC 52!".to_string(),
                                );
                            }
                        }
                        _ => {}
                    },

                    CurrentView::BackupManager => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.current_view = CurrentView::Dashboard;
                            app.backup_state = None;
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.select_next_backup();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.select_prev_backup();
                        }
                        KeyCode::Char('r') => {
                            if let Some(ref state) = app.backup_state {
                                if let Some(b) = state.backups.get(state.selected_index) {
                                    match mcpforge_adapters::restore_backup(
                                        &b.backup_path,
                                        &b.target_path,
                                    ) {
                                        Ok(_) => {
                                            app.status_message = Some(format!(
                                                "✓ Successfully restored snapshot for '{}'!",
                                                b.client_id
                                            ));
                                            app.refresh_servers();
                                            app.current_view = CurrentView::Dashboard;
                                            app.backup_state = None;
                                        }
                                        Err(e) => {
                                            app.status_message =
                                                Some(format!("Error restoring snapshot: {}", e));
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    },

                    CurrentView::ViewClientConfig => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter | KeyCode::Char('v') => {
                            app.close_client_config_modal();
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.scroll_client_config_down(1);
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.scroll_client_config_up(1);
                        }
                        KeyCode::PageDown => {
                            app.scroll_client_config_down(10);
                        }
                        KeyCode::PageUp => {
                            app.scroll_client_config_up(10);
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    Ok(())
}

fn base64_encode(input: &[u8]) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        out.push(CHARSET[(b0 >> 2) as usize] as char);
        out.push(CHARSET[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(CHARSET[(((b1 & 15) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(CHARSET[(b2 & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}
