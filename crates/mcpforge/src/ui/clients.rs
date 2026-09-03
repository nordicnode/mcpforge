use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::ui::theme::Theme;
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
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let tab2_style = if active_tab == 1 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mut spans = vec![
        Span::styled(" MCPForge ", theme.title),
        Span::raw(" "),
        Span::styled(format!(" [1] Servers ({}) ", total_servers), tab1_style),
        Span::raw(" "),
        Span::styled(
            format!(
                " [2] Clients & Harnesses ({} installed, {} active) ",
                installed_clients, running_count
            ),
            tab2_style,
        ),
    ];

    if let Some(ref msg) = app.status_message {
        spans.push(Span::raw(" │ "));
        spans.push(Span::styled(msg, Style::default().fg(Color::LightCyan)));
    }

    let p = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border),
    );
    f.render_widget(p, area);
}

fn render_clients_split(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
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
                (
                    "● ACTIVE ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
            } else if client.is_running {
                (
                    "● RUNNING",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            } else if client.is_installed {
                ("○ READY  ", Style::default().fg(Color::LightBlue))
            } else {
                ("· AVAIL  ", Style::default().fg(Color::DarkGray))
            };

            let prefix = if is_selected { "▶ " } else { "  " };

            let count_span = if client.server_count > 0 {
                Span::styled(
                    format!(" [{} servers]", client.server_count),
                    Style::default().fg(Color::Yellow),
                )
            } else {
                Span::styled(" [0 servers]", Style::default().fg(Color::DarkGray))
            };

            let line = Line::from(vec![
                Span::styled(
                    prefix,
                    if is_selected {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(status_badge, badge_style),
                Span::styled(
                    format!(" {:<20}", client.display_name),
                    if is_selected {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else if client.is_installed {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::Gray)
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
            " Clients & Harnesses ({}/{}) ",
            app.selected_client_index + 1,
            total
        )
    } else {
        " Clients & Harnesses (0/0) ".to_string()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border)
            .title(title),
    );

    f.render_widget(list, area);
}

fn render_client_details(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border)
        .title(" Client Inspector ");

    let client = match app.selected_client() {
        Some(c) => c,
        None => {
            let empty = Paragraph::new("No client selected.")
                .block(block)
                .style(theme.muted);
            f.render_widget(empty, area);
            return;
        }
    };

    let mut lines = Vec::new();

    // Title & ID
    lines.push(Line::from(vec![
        Span::styled(
            &client.display_name,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(format!("id: {}", client.id), theme.muted),
    ]));
    lines.push(Line::raw(""));

    // Status Row with Explicit Breakdown
    let (status_badge, status_title, status_style, meaning_text, next_action) = if client.is_running
        && client.is_installed
    {
        (
                "● ACTIVE",
                "Live Process Running & Configuration Active",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                "The client application is actively running on this OS and reading this configuration. Servers synced here can be used in your live session.",
                "Press [u] to sync all servers to this client's active session.",
            )
    } else if client.is_running {
        (
                "● RUNNING (UNCONFIGURED)",
                "Live Process Detected, but No MCP Config File Yet",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                "The application is actively running on your machine, but hasn't had an MCP configuration file created yet.",
                "Press [u] to initialize the config and sync your MCP servers into it immediately.",
            )
    } else if client.is_installed {
        (
                "○ READY (CONFIGURED)",
                "MCP Configuration Exists on Disk (Process Idle)",
                Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD),
                "The application has an MCP configuration file on disk, but is not currently running. Servers will be loaded automatically on next launch.",
                "Press [u] to update or sync servers ahead of your next session.",
            )
    } else {
        (
                "· AVAILABLE",
                "Adapter Supported by MCPForge (Unconfigured)",
                Style::default().fg(Color::DarkGray),
                "MCPForge has built-in support for this AI harness, but neither an active process nor an MCP configuration file was detected on this system.",
                "Press [u] or select this client in the Add Wizard to provision its configuration file.",
            )
    };

    lines.push(Line::from(vec![
        Span::styled(
            "Status:   ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("[{}] ", status_badge), status_style),
        Span::styled(status_title, Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Meaning:  ", Style::default().fg(Color::Yellow)),
        Span::styled(meaning_text, Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Action:   ", Style::default().fg(Color::Yellow)),
        Span::styled(next_action, Style::default().fg(Color::LightCyan)),
    ]));
    lines.push(Line::raw(""));

    // Configuration Path
    lines.push(Line::from(vec![
        Span::styled("Config:   ", Style::default().fg(Color::Yellow)),
        Span::styled(
            client.config_path.display().to_string(),
            Style::default().fg(Color::White),
        ),
    ]));

    let on_path = DiscoveryEngine::is_client_installed(&client.id);
    lines.push(Line::from(vec![
        Span::styled("Binary:   ", Style::default().fg(Color::Yellow)),
        if on_path {
            Span::styled(
                "✓ Executable detected on $PATH",
                Style::default().fg(Color::Green),
            )
        } else {
            Span::styled("✗ Not found on $PATH", Style::default().fg(Color::DarkGray))
        },
    ]));
    lines.push(Line::raw(""));

    // Configured Servers Header
    lines.push(Line::from(vec![Span::styled(
        format!(
            "Configured Servers in this Client ({}/{}):",
            client.server_count,
            app.servers.len()
        ),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )]));

    // Find servers that belong to this client config
    let matching_servers: Vec<_> = app
        .servers
        .iter()
        .filter(|s| {
            s.clients
                .iter()
                .any(|c| c.config_path == client.config_path)
        })
        .collect();

    if matching_servers.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "No servers configured in this client. Press [u] to sync all servers into this client!",
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
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("[{}]", s.transport.transport_type_str()),
                    Style::default().fg(Color::LightBlue),
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
        Span::styled(
            "Legend: ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "[● ACTIVE] ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Running + Configured  ", Style::default().fg(Color::White)),
        Span::styled(
            "[○ READY] ",
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Configured (Idle)  ", Style::default().fg(Color::White)),
        Span::styled("[· AVAIL] ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "Supported Adapter (Unconfigured)",
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let key_hints = Line::from(vec![
        Span::styled(
            "[Tab/1] Servers View   [j/k] Navigate   [u] Sync to Client   [r] Rescan   [?] Help   [q] Quit",
            theme.key_hint,
        ),
    ]);

    let p = Paragraph::new(vec![legend, key_hints]);
    f.render_widget(p, area);
}
