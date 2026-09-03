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
use std::io::stdout;
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
                    CurrentView::Dashboard => match key.code {
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
                            app.status_message =
                                Some(format!("Auto-synced {} servers across all clients!", count));
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
                                        server.clients.iter().any(|sc| sc.config_path == c.path)
                                    })
                                    .cloned()
                                    .collect();
                                let _ = app.manager.write_server_to_locations(&server, &locs);
                                app.refresh_servers();
                            }
                        }
                        KeyCode::Char('t') => {
                            if let Some(server) = app.selected_server().cloned() {
                                app.tool_explorer_state = Some(crate::app::ToolExplorerState {
                                    server_id: server.id.clone(),
                                    tools: Vec::new(),
                                    selected_index: 0,
                                    is_loading: true,
                                    execution_result: None,
                                    error_message: None,
                                });
                                app.current_view = CurrentView::ToolExplorer;
                                terminal.draw(|f| ui::render_ui(f, app))?;

                                match mcp_core::client::list_server_tools(&server, 10).await {
                                    Ok(tools) => {
                                        if let Some(ref mut state) = app.tool_explorer_state {
                                            state.is_loading = false;
                                            state.tools = tools;
                                        }
                                    }
                                    Err(e) => {
                                        if let Some(ref mut state) = app.tool_explorer_state {
                                            state.is_loading = false;
                                            state.error_message = Some(e.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('T') => {
                            if let Some(server) = app.selected_server().cloned() {
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
                                            server_name, server_version, tool_count, elapsed_ms
                                        ));
                                    }
                                    mcp_core::types::HealthStatus::Degraded { reason, .. } => {
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
                        KeyCode::Char('v') | KeyCode::Enter if app.selected_server().is_some() => {
                            app.current_view = CurrentView::ViewSnippet;
                        }
                        _ => {}
                    },

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
                            KeyCode::Enter => {
                                if let (Some(server), Some(ref state)) =
                                    (app.selected_server().cloned(), &app.tool_explorer_state)
                                {
                                    if let Some(tool) = state.tools.get(state.selected_index) {
                                        let tool_name = tool.name.clone();
                                        if let Some(ref mut s) = app.tool_explorer_state {
                                            s.execution_result = Some(
                                            "Executing tool test with default parameters ({})..."
                                                .to_string(),
                                        );
                                            s.error_message = None;
                                        }
                                        terminal.draw(|f| ui::render_ui(f, app))?;

                                        let call_res = mcp_core::client::call_server_tool(
                                            &server,
                                            &tool_name,
                                            serde_json::json!({}),
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
                }
            }
        }
    }

    Ok(())
}
