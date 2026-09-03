use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::ui::theme::Theme;
use mcp_core::types::HealthStatus;
use mcpforge_adapters::DiscoveryEngine;

pub fn render_clients_view(f: &mut Frame, app: &App) {
    let theme = Theme::default();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Tab bar
            Constraint::Min(10),   // Content split
            Constraint::Length(2), // Footer (Legend + Key hints)
        ])
        .split(f.area());

    render_header_tabs(f, app, chunks[0], &theme, 1);
    render_clients_split(f, app, chunks[1], &theme);
    render_clients_footer(f, chunks[2], &theme);
}

pub fn render_header_tabs(f: &mut Frame, app: &App, area: Rect, theme: &Theme, active_tab: usize) {
    let total_servers = app.servers.len();
    let installed_clients = app
        .discovered_clients
        .iter()
        .filter(|c| c.is_installed)
        .count();
    let running_count = app
        .discovered_clients
        .iter()
        .filter(|c| c.is_running)
        .count();

    let tab1_style = if active_tab == 0 {
        theme.tab_active
    } else {
        theme.tab_inactive
    };

    let tab2_style = if active_tab == 1 {
        theme.tab_active
    } else {
        theme.tab_inactive
    };

    let healthy_count = app
        .health_cache
        .values()
        .filter(|h| matches!(h, HealthStatus::Healthy { .. }))
        .count();

    let health_badge =
        if !app.health_cache.is_empty() && healthy_count == total_servers && total_servers > 0 {
            Span::styled(
                format!(" ● {}/{} Healthy ", healthy_count, total_servers),
                theme.status_healthy,
            )
        } else if total_servers > 0 && healthy_count > 0 {
            Span::styled(
                format!(" ▲ {}/{} Healthy ", healthy_count, total_servers),
                theme.status_degraded,
            )
        } else {
            Span::styled(" ● All Systems Ready ", theme.status_healthy)
        };

    let mut spans = vec![
        Span::styled(
            " MCPFORGE ",
            Style::default()
                .fg(Color::Rgb(139, 233, 253))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("v0.1.0 ", theme.muted),
        Span::raw("  "),
        Span::styled(format!(" 1: Servers ({}) ", total_servers), tab1_style),
        Span::raw(" "),
        Span::styled(
            format!(
                " 2: Clients ({}/{} Installed · {} Active) ",
                installed_clients,
                app.discovered_clients.len(),
                running_count,
            ),
            tab2_style,
        ),
        Span::raw("  "),
        health_badge,
    ];

    if let Some(ref msg) = app.status_message {
        spans.push(Span::raw(" │ "));
        spans.push(Span::styled(
            msg,
            Style::default()
                .fg(Color::Rgb(139, 233, 253))
                .add_modifier(Modifier::BOLD),
        ));
    }

    let p = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(theme.border_type)
            .border_style(theme.border),
    );
    f.render_widget(p, area);
}

fn render_clients_split(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(46), Constraint::Percentage(54)])
        .split(area);

    render_client_list(f, app, main_chunks[0], theme);
    render_client_details(f, app, main_chunks[1], theme);
}

fn render_client_list(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let total = app.discovered_clients.len();
    let visible_height = (area.height as usize).saturating_sub(2).max(1);
    let (start, end) = crate::ui::layout::calculate_scroll_window(
        total,
        app.selected_client_index,
        visible_height,
    );

    let items: Vec<ListItem> = app.discovered_clients[start..end]
        .iter()
        .enumerate()
        .map(|(offset, client)| {
            let real_idx = start + offset;
            let is_selected = real_idx == app.selected_client_index;

            let (status_badge, badge_style) = if client.is_running && client.is_installed {
                ("● ACTIVE", theme.pill_active)
            } else if client.is_running {
                (
                    "● RUNNING",
                    Style::default()
                        .fg(Color::Rgb(139, 233, 253))
                        .add_modifier(Modifier::BOLD),
                )
            } else if client.is_installed {
                ("○ READY ", theme.pill_ready)
            } else {
                ("· AVAIL ", theme.pill_avail)
            };

            let prefix = if is_selected { "▶ " } else { "  " };

            let cat_style = match client.category.as_str() {
                "Agent" => Style::default().fg(Color::Rgb(189, 147, 249)),
                "CLI" => Style::default().fg(Color::Rgb(241, 250, 140)),
                "IDE" => Style::default().fg(Color::Rgb(139, 233, 253)),
                "Chat" => Style::default().fg(Color::Rgb(80, 250, 123)),
                _ => theme.muted,
            };

            let count_span = if client.server_count > 0 {
                Span::styled(
                    format!(" [{} servers]", client.server_count),
                    Style::default().fg(Color::Rgb(241, 250, 140)),
                )
            } else {
                Span::styled(" [0 servers]", theme.muted)
            };

            let line = Line::from(vec![
                Span::styled(
                    prefix,
                    if is_selected {
                        theme.title
                    } else {
                        theme.muted
                    },
                ),
                Span::styled(format!("[{}] ", status_badge), badge_style),
                Span::styled(format!("[{:<5}] ", client.category), cat_style),
                Span::styled(
                    format!("{:<20}", client.display_name),
                    if is_selected {
                        theme.selected
                    } else if client.is_installed {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::Rgb(150, 155, 175))
                    },
                ),
                count_span,
            ]);

            let mut li = ListItem::new(line);
            if is_selected {
                li = li.style(theme.selected);
            }
            li
        })
        .collect();

    let title = if total > 0 {
        format!(
            " CLIENTS & AGENT HARNESSES ({}/{}) ",
            app.selected_client_index + 1,
            total
        )
    } else {
        " CLIENTS & AGENT HARNESSES (0/0) ".to_string()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(theme.border_type)
            .border_style(theme.border)
            .title(title)
            .title_style(theme.title),
    );

    f.render_widget(list, area);
}

fn render_client_details(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(theme.border)
        .title(" HARNESS SPECIFICATIONS & STATUS ")
        .title_style(theme.title);

    let client = match app.selected_client() {
        Some(c) => c,
        None => {
            let empty = Paragraph::new("\n  No client harness selected.")
                .block(block)
                .style(theme.muted);
            f.render_widget(empty, area);
            return;
        }
    };

    let mut lines = Vec::new();

    // 1. Overview & Identity
    let cat_desc = match client.category.as_str() {
        "Agent" => "Autonomous Agent",
        "CLI" => "Terminal & CLI Tool",
        "IDE" => "IDE / Code Editor",
        "Chat" => "Chat & Desktop Client",
        _ => "Custom Harness",
    };

    lines.push(Line::from(vec![
        Span::styled(
            &client.display_name,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("id: {}", client.id),
            Style::default().fg(Color::Rgb(139, 233, 253)),
        ),
        Span::raw("  "),
        Span::styled(
            format!("({})", cat_desc),
            Style::default().fg(Color::Rgb(189, 147, 249)),
        ),
    ]));
    lines.push(Line::raw(""));

    // 2. Status Row with Plain-English Explanation
    let (status_badge, status_title, status_style, meaning_text, next_action) = if client.is_running
        && client.is_installed
    {
        (
            "ACTIVE",
            "Live Process Running & Configuration Active",
            theme.pill_active,
            "Application is actively executing on this OS and reading its configuration. Servers are live.",
            "Press [u] to sync all servers to this harness's active session.",
        )
    } else if client.is_running {
        (
            "RUNNING",
            "Live Process Detected, but Config File Not Yet Created",
            Style::default().fg(Color::Rgb(139, 233, 253)).add_modifier(Modifier::BOLD),
            "Process is running on your machine, but hasn't had an MCP configuration file written yet.",
            "Press [u] to initialize configuration and sync all servers immediately.",
        )
    } else if client.is_installed {
        (
            "READY",
            "MCP Configuration Exists on Disk (Process Idle)",
            theme.pill_ready,
            "Configuration file exists on disk. Configured servers will be loaded on next launch.",
            "Press [u] to update or sync servers ahead of your next session.",
        )
    } else {
        (
            "AVAILABLE",
            "Built-in Adapter Supported (Unconfigured)",
            theme.pill_avail,
            "MCPForge provides native auto-configuration for this tool, but no installation was found.",
            "Press [u] or choose this client in the Add Wizard to provision its config file.",
        )
    };

    lines.push(Line::from(vec![
        Span::styled("Status:   ", theme.header),
        Span::styled(format!("[{}] ", status_badge), status_style),
        Span::styled(status_title, Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Meaning:  ", theme.header),
        Span::styled(meaning_text, Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Action:   ", theme.header),
        Span::styled(next_action, theme.key_shortcut),
    ]));
    lines.push(Line::raw(""));

    // 3. Known Configuration Locations
    lines.push(Line::from(vec![Span::styled(
        "CONFIGURATION LOCATIONS & PATHS",
        theme.header,
    )]));

    if client.all_locations.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("  • "),
            Span::styled(
                client.config_path.display().to_string(),
                Style::default().fg(Color::White),
            ),
        ]));
    } else {
        for loc in &client.all_locations {
            let (loc_icon, loc_style) = if loc.exists {
                ("✓", theme.status_healthy)
            } else {
                ("·", theme.muted)
            };

            let scope_str = match loc.scope {
                mcp_core::types::Scope::Global => "Global",
                mcp_core::types::Scope::Project => "Project",
            };

            let mut spans = vec![
                Span::raw("  "),
                Span::styled(format!("[{}] ", loc_icon), loc_style),
                Span::styled(
                    format!("{:<8}", scope_str),
                    Style::default().fg(Color::Rgb(139, 233, 253)),
                ),
                Span::styled(
                    loc.path.display().to_string(),
                    Style::default().fg(Color::White),
                ),
            ];

            if loc.exists {
                spans.push(Span::styled(" (active)", theme.status_healthy));
            }

            lines.push(Line::from(spans));
        }
    }
    lines.push(Line::raw(""));

    // 4. Executable / PATH Check
    let on_path = DiscoveryEngine::is_client_installed(&client.id);
    lines.push(Line::from(vec![
        Span::styled("Binary:   ", theme.header),
        if on_path {
            Span::styled(
                "✓ Executable detected on system $PATH",
                theme.status_healthy,
            )
        } else {
            Span::styled(
                "✗ Executable not found on $PATH (config-file based adapter)",
                theme.muted,
            )
        },
    ]));
    lines.push(Line::raw(""));

    // 5. Configured Servers in this Client
    lines.push(Line::from(vec![Span::styled(
        format!(
            "CONFIGURED SERVERS IN THIS CLIENT ({}/{}):",
            client.server_count,
            app.servers.len()
        ),
        theme.header,
    )]));

    let matching_servers: Vec<_> = app
        .servers
        .iter()
        .filter(|s| {
            s.clients.iter().any(|c| {
                client
                    .all_locations
                    .iter()
                    .any(|al| al.path == c.config_path)
            })
        })
        .collect();

    if matching_servers.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "No servers configured yet. Press [u] to sync all servers into this client!",
                theme.muted,
            ),
        ]));
    } else {
        for s in matching_servers {
            lines.push(Line::from(vec![
                Span::raw("  • "),
                Span::styled(
                    &s.id,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("[{}]", s.transport.transport_type_str()),
                    Style::default().fg(Color::Rgb(139, 233, 253)),
                ),
            ]));
        }
    }

    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn render_clients_footer(f: &mut Frame, area: Rect, theme: &Theme) {
    let legend = Line::from(vec![
        Span::styled("Legend: ", theme.header),
        Span::styled("[● ACTIVE] ", theme.pill_active),
        Span::styled("Running + Configured   ", Style::default().fg(Color::White)),
        Span::styled("[○ READY] ", theme.pill_ready),
        Span::styled("Configured (Idle)   ", Style::default().fg(Color::White)),
        Span::styled("[· AVAIL] ", theme.pill_avail),
        Span::styled("Supported Adapter (Unconfigured)", theme.muted),
    ]);

    let key_hints = Line::from(vec![
        Span::styled("[Tab/1]", theme.key_shortcut),
        Span::raw(" Servers   "),
        Span::styled("[j/k]", theme.key_shortcut),
        Span::raw(" Navigate   "),
        Span::styled("[u]", theme.key_shortcut),
        Span::raw(" Sync   "),
        Span::styled("[d]", theme.key_shortcut),
        Span::raw(" Remove   "),
        Span::styled("[r]", theme.key_shortcut),
        Span::raw(" Rescan   "),
        Span::styled("[?]", theme.key_shortcut),
        Span::raw(" Help   "),
        Span::styled("[q]", theme.key_shortcut),
        Span::raw(" Quit"),
    ]);

    let p = Paragraph::new(vec![legend, key_hints]);
    f.render_widget(p, area);
}
