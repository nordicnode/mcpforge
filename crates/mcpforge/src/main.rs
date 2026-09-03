use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::BTreeMap;
use std::io::{self, stdout, Read};
use std::time::Duration;

mod app;
mod cli;
mod doctor;
mod secrets;
mod ui;

use app::{App, CurrentView, WizardSource, WizardStep};
use cli::{Cli, Commands};
use doctor::DoctorReport;
use mcp_core::client::check_server_health;
use mcp_core::types::ServerEntry;
use mcpforge_adapters::{AdapterManager, ConfigLocation};
use mcpforge_registry::Registry;
use secrets::{is_secret_key, redact_secret};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        return handle_cli_command(cmd).await;
    }

    // Launch interactive TUI
    run_tui(cli.doctor).await
}

async fn handle_cli_command(cmd: Commands) -> Result<()> {
    let manager = AdapterManager::new();
    let registry = Registry::load().unwrap_or_default();

    match cmd {
        Commands::List { client, json } => {
            let mut servers = manager.read_all_servers()?;
            if let Some(ref c) = client {
                servers.retain(|s| s.clients.iter().any(|cr| cr.client_id == *c));
            }

            if json {
                println!("{}", serde_json::to_string_pretty(&servers)?);
            } else {
                println!(
                    "\n{:<22} {:<16} {:<30}",
                    "SERVER", "TRANSPORT", "INSTALLED IN"
                );
                println!("{}", "-".repeat(70));
                for s in &servers {
                    let mut installed: Vec<String> =
                        s.clients.iter().map(|c| c.display_name.clone()).collect();
                    installed.dedup();
                    println!(
                        "{:<22} {:<16} {:<30}",
                        s.id,
                        s.transport.transport_type_str(),
                        installed.join(", ")
                    );
                }
                println!();
            }
        }

        Commands::Doctor { json, timeout } => {
            let servers = manager.read_all_servers()?;
            println!(
                "Running doctor checks on {} configured servers...",
                servers.len()
            );
            let report = DoctorReport::run(&servers, timeout).await;

            if json {
                println!("{}", report.to_json()?);
            } else {
                let ok = report.print_table();
                if !ok {
                    std::process::exit(1);
                }
            }
        }

        Commands::Add {
            name,
            from_registry,
            stdin,
            command,
            args,
            to,
        } => {
            let all_locations = manager.detect_all();
            let targets: Vec<ConfigLocation> = if to.is_empty() {
                all_locations.into_iter().filter(|l| l.exists).collect()
            } else {
                all_locations
                    .into_iter()
                    .filter(|l| to.contains(&l.client_id))
                    .collect()
            };

            if targets.is_empty() {
                eprintln!("Error: No matching client targets found.");
                std::process::exit(1);
            }

            let server: ServerEntry = if from_registry {
                let id = name.context("Server name required when using --from-registry")?;
                let cat_entry = registry
                    .find_by_id(&id)
                    .with_context(|| format!("Server '{}' not found in registry", id))?;
                cat_entry.to_server_entry(BTreeMap::new())
            } else if stdin {
                let mut buf = String::new();
                io::stdin().read_to_string(&mut buf)?;
                let id = name.unwrap_or_else(|| "custom-server".to_string());
                let val: serde_json::Value = serde_json::from_str(&buf)?;
                let obj = val.as_object().context("Expected JSON object from stdin")?;
                let cmd = obj
                    .get("command")
                    .and_then(|c| c.as_str())
                    .context("Missing 'command'")?;
                let s_args = obj
                    .get("args")
                    .and_then(|a| a.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                ServerEntry::new_stdio(id, cmd, s_args, BTreeMap::new())
            } else {
                let id = name.context("Server name required")?;
                let cmd = command.context("--command is required")?;
                ServerEntry::new_stdio(id, cmd, args, BTreeMap::new())
            };

            manager.write_server_to_locations(&server, &targets)?;
            println!(
                "Successfully added server '{}' to {} client(s).",
                server.id,
                targets.len()
            );
        }

        Commands::Sync { target, from } => {
            let all_locs = manager.detect_all();
            let src_loc = all_locs
                .iter()
                .find(|l| l.client_id == from)
                .with_context(|| format!("Source client '{}' not found", from))?;
            let tgt_loc = all_locs
                .iter()
                .find(|l| l.client_id == target)
                .with_context(|| format!("Target client '{}' not found", target))?;

            let mut src_servers = Vec::new();
            for adapter in manager.adapters() {
                if adapter.id() == from {
                    src_servers = adapter.read_servers(src_loc)?;
                    break;
                }
            }

            for s in &src_servers {
                manager.write_server_to_locations(s, std::slice::from_ref(tgt_loc))?;
            }

            println!(
                "Synced {} servers from '{}' to '{}'.",
                src_servers.len(),
                from,
                target
            );
        }

        Commands::Export {
            output,
            include_secrets,
        } => {
            let mut servers = manager.read_all_servers()?;
            if !include_secrets {
                for s in &mut servers {
                    if let mcp_core::types::Transport::Stdio { env, .. } = &mut s.transport {
                        for (k, v) in env.iter_mut() {
                            if is_secret_key(k) {
                                *v = redact_secret(v);
                            }
                        }
                    }
                }
            }

            let json = serde_json::to_string_pretty(&servers)?;
            if let Some(path) = output {
                std::fs::write(&path, json)?;
                println!("Exported {} servers to {:?}", servers.len(), path);
            } else {
                println!("{}", json);
            }
        }

        Commands::Import { input, to } => {
            let content = std::fs::read_to_string(&input)?;
            let servers: Vec<ServerEntry> = serde_json::from_str(&content)?;

            let all_locations = manager.detect_all();
            let targets: Vec<ConfigLocation> = if let Some(ref client_ids) = to {
                all_locations
                    .into_iter()
                    .filter(|l| client_ids.contains(&l.client_id))
                    .collect()
            } else {
                all_locations.into_iter().filter(|l| l.exists).collect()
            };

            for s in &servers {
                manager.write_server_to_locations(s, &targets)?;
            }

            println!(
                "Imported {} servers into {} client(s).",
                servers.len(),
                targets.len()
            );
        }
    }

    Ok(())
}

async fn run_tui(run_doctor_init: bool) -> Result<()> {
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
                        KeyCode::Char('a') => {
                            app.start_wizard();
                        }
                        KeyCode::Char('d') => {
                            if let Some(server) = app.selected_server().cloned() {
                                let locs: Vec<ConfigLocation> = app
                                    .detected_clients
                                    .iter()
                                    .filter(|c| {
                                        server.clients.iter().any(|sc| sc.config_path == c.path)
                                    })
                                    .cloned()
                                    .collect();
                                let _ = app.manager.remove_server_from_locations(&server.id, &locs);
                                app.refresh_servers();
                                app.status_message = Some(format!("Removed '{}'", server.id));
                            }
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
                        _ => {}
                    },

                    CurrentView::AddWizard => {
                        let mut step_action = None;
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
                                            if wizard.registry_cursor > 0 {
                                                wizard.registry_cursor -= 1;
                                            }
                                        }
                                        KeyCode::Down | KeyCode::Char('j') => {
                                            let count = app.registry.entries().len();
                                            if wizard.registry_cursor + 1 < count {
                                                wizard.registry_cursor += 1;
                                            }
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
                                    KeyCode::Char(' ') => {
                                        if let Some((_, selected)) =
                                            wizard.target_locations.first_mut()
                                        {
                                            *selected = !*selected;
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
                }
            }
        }
    }

    Ok(())
}
