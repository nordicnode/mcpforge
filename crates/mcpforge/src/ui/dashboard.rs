use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::secrets::{is_secret_key, redact_secret};
use crate::ui::layout::{create_app_layout, create_split_main_layout};
use crate::ui::theme::Theme;
use mcp_core::types::{HealthStatus, Transport};

pub fn render_dashboard(f: &mut Frame, app: &App) {
    let theme = Theme::default();
    let layout = create_app_layout(f.area());

    // 1. Header with Tab Bar
    crate::ui::clients::render_header_tabs(f, app, layout.header, &theme, 0);

    // 2. Main split
    let (left_rect, right_rect) = create_split_main_layout(layout.main);
    render_server_list(f, app, left_rect, &theme);
    render_server_details(f, app, right_rect, &theme);

    // 3. Footer
    render_footer(f, app, layout.footer, &theme);
}

fn render_server_list(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let filtered = app.filtered_servers();
    let total = filtered.len();
    let visible_height = (area.height as usize).saturating_sub(2).max(1);
    let (start, end) =
        crate::ui::layout::calculate_scroll_window(total, app.selected_index, visible_height);

    let items: Vec<ListItem> = filtered[start..end]
        .iter()
        .enumerate()
        .map(|(offset, server)| {
            let real_idx = start + offset;
            let status = app
                .health_cache
                .get(&server.id)
                .cloned()
                .unwrap_or(HealthStatus::Unknown);

            let (icon, icon_style) = match status {
                HealthStatus::Healthy { .. } => ("●", theme.status_healthy),
                HealthStatus::Degraded { .. } => ("▲", theme.status_degraded),
                HealthStatus::Broken { .. } => ("✖", theme.status_broken),
                HealthStatus::Disabled => ("○", theme.status_disabled),
                HealthStatus::Unknown => ("?", theme.muted),
            };

            let is_selected = real_idx == app.selected_index;
            let cursor = if is_selected { "▶ " } else { "  " };

            let name_style = if is_selected {
                theme.selected
            } else if !server.enabled {
                theme.status_disabled
            } else {
                Style::default().fg(Color::White)
            };

            let client_count = server.clients.len();
            let line = Line::from(vec![
                Span::styled(
                    cursor,
                    if is_selected {
                        theme.title
                    } else {
                        theme.muted
                    },
                ),
                Span::styled(format!("{} ", icon), icon_style),
                Span::styled(&server.id, name_style),
                Span::raw(" "),
                Span::styled(
                    format!("[{}]", server.transport.transport_type_str()),
                    Style::default().fg(Color::Rgb(139, 233, 253)),
                ),
                Span::raw(" "),
                Span::styled(format!("({} clients)", client_count), theme.muted),
            ]);

            ListItem::new(line)
        })
        .collect();

    let title = if total > 0 {
        format!(
            " CONFIGURED SERVERS ({}/{}) ",
            app.selected_index + 1,
            total
        )
    } else {
        " CONFIGURED SERVERS (0/0) ".to_string()
    };

    let list_block = Block::default()
        .title(title)
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(theme.border);

    let list = List::new(items)
        .block(list_block)
        .highlight_style(theme.selected);

    f.render_widget(list, area);
}

fn render_server_details(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let block = Block::default()
        .title(" SERVER INSPECTOR & CLIENT STATUS ")
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(theme.border);

    let server = match app.selected_server() {
        Some(s) => s,
        None => {
            let empty = Paragraph::new("\n  No servers configured or none match search filter.\n\n  • Press [a] to launch the Add Server Wizard.\n  • Press [Tab] to view discovered Clients & Harnesses.")
                .block(block)
                .style(theme.muted);
            f.render_widget(empty, area);
            return;
        }
    };

    let mut lines = Vec::new();

    // 1. Overview Card
    let (status_badge, status_style) = if server.enabled {
        ("ENABLED", theme.status_healthy)
    } else {
        ("DISABLED", theme.status_disabled)
    };

    lines.push(Line::from(vec![
        Span::styled("● ", status_style),
        Span::styled(
            &server.id,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(format!("[{}]", status_badge), status_style),
        Span::raw("  "),
        Span::styled(
            format!("Transport: {}", server.transport.transport_type_str()),
            Style::default().fg(Color::Rgb(189, 147, 249)),
        ),
    ]));
    lines.push(Line::raw(""));

    // 2. Command & Execution Configuration
    lines.push(Line::from(vec![Span::styled(
        "EXECUTION CONFIGURATION",
        theme.header,
    )]));

    match &server.transport {
        Transport::Stdio { command, args, env } => {
            lines.push(Line::from(vec![
                Span::styled(
                    "  Command: ",
                    Style::default().fg(Color::Rgb(139, 233, 253)),
                ),
                Span::styled(command, Style::default().fg(Color::White)),
            ]));
            if !args.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled(
                        "  Args:    ",
                        Style::default().fg(Color::Rgb(139, 233, 253)),
                    ),
                    Span::styled(args.join(" "), Style::default().fg(Color::White)),
                ]));
            }
            if !env.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    "  Env Vars:",
                    Style::default().fg(Color::Rgb(139, 233, 253)),
                )]));
                for (k, v) in env {
                    let display_v = if is_secret_key(k) {
                        redact_secret(v)
                    } else {
                        v.clone()
                    };
                    lines.push(Line::from(vec![
                        Span::raw("    • "),
                        Span::styled(k, Style::default().fg(Color::Rgb(241, 250, 140))),
                        Span::raw(" = "),
                        Span::styled(display_v, Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
        }
        Transport::StreamableHttp { url, headers } => {
            lines.push(Line::from(vec![
                Span::styled(
                    "  Endpoint URL: ",
                    Style::default().fg(Color::Rgb(139, 233, 253)),
                ),
                Span::styled(url, Style::default().fg(Color::White)),
            ]));
            if !headers.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    "  Headers:      ",
                    Style::default().fg(Color::Rgb(139, 233, 253)),
                )]));
                for (k, v) in headers {
                    let display_v = if is_secret_key(k) {
                        redact_secret(v)
                    } else {
                        v.clone()
                    };
                    lines.push(Line::from(vec![
                        Span::raw("    • "),
                        Span::styled(k, Style::default().fg(Color::Rgb(241, 250, 140))),
                        Span::raw(": "),
                        Span::styled(display_v, Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
        }
        Transport::Sse { url } => {
            lines.push(Line::from(vec![
                Span::styled(
                    "  Endpoint URL: ",
                    Style::default().fg(Color::Rgb(139, 233, 253)),
                ),
                Span::styled(url, Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled("(Legacy SSE)", theme.status_degraded),
            ]));
        }
    }
    lines.push(Line::raw(""));

    // 3. Client Installation Matrix
    lines.push(Line::from(vec![Span::styled(
        "INSTALLED IN CLIENTS & HARNESSES",
        theme.header,
    )]));

    let mut rendered_any = false;
    for client in &app.detected_clients {
        let is_installed = server.clients.iter().any(|c| c.config_path == client.path);

        if !client.exists && !is_installed {
            continue;
        }

        rendered_any = true;
        let (check_icon, check_style) = if is_installed {
            ("✓", theme.status_healthy)
        } else {
            ("✗", theme.muted)
        };

        let is_running = app.running_processes.contains(&client.client_id);
        let mut client_spans = vec![
            Span::raw("  "),
            Span::styled(format!("[{}] ", check_icon), check_style),
            Span::styled(&client.display_name, Style::default().fg(Color::White)),
        ];

        if is_running {
            client_spans.push(Span::styled(" [ACTIVE]", theme.pill_active));
        }

        client_spans.push(Span::styled(
            format!(" · {}", client.path.display()),
            theme.muted,
        ));
        lines.push(Line::from(client_spans));
    }

    if !rendered_any {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "None (server not yet installed in any client. Press [u] to sync).",
                theme.muted,
            ),
        ]));
    }
    lines.push(Line::raw(""));

    // 4. Runtime Health Check & Telemetry
    lines.push(Line::from(vec![Span::styled(
        "RUNTIME HEALTH & TELEMETRY",
        theme.header,
    )]));

    let health = app
        .health_cache
        .get(&server.id)
        .cloned()
        .unwrap_or(HealthStatus::Unknown);

    let (health_icon, health_text, health_style) = match health {
        HealthStatus::Healthy {
            latency_ms,
            tool_count,
            server_name,
            server_version,
        } => (
            "●",
            format!(
                "Connected & Operational · {} v{} · {} tool(s) exposed · {}ms latency",
                server_name, server_version, tool_count, latency_ms
            ),
            theme.status_healthy,
        ),
        HealthStatus::Degraded { reason, latency_ms } => {
            let ms = latency_ms.map_or(String::new(), |m| format!(" · {}ms", m));
            (
                "▲",
                format!("Degraded: {}{}", reason, ms),
                theme.status_degraded,
            )
        }
        HealthStatus::Broken { error } => ("✖", format!("Error: {}", error), theme.status_broken),
        HealthStatus::Disabled => (
            "○",
            "Server currently disabled by user toggle".to_string(),
            theme.status_disabled,
        ),
        HealthStatus::Unknown => (
            "?",
            "Not checked yet. Press [r] to run instant health check.".to_string(),
            theme.muted,
        ),
    };

    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("{} ", health_icon), health_style),
        Span::styled(health_text, health_style),
    ]));

    if let Some(ref notes) = server.notes {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("Notes: ", Style::default().fg(Color::Rgb(189, 147, 249))),
            Span::raw(notes),
        ]));
    }

    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(para, area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let line = if app.is_searching {
        Line::from(vec![
            Span::styled(" SEARCH: ", theme.header),
            Span::styled(
                &app.search_query,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("█", Style::default().fg(Color::Cyan)),
            Span::styled(" (Press Enter to apply, Esc to clear)", theme.muted),
        ])
    } else if let Some(ref msg) = app.status_message {
        Line::from(vec![
            Span::styled(" STATUS: ", theme.key_shortcut),
            Span::styled(
                msg,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │  ", theme.muted),
            Span::styled(
                "[Tab] Views  [a] Add  [u] Sync  [r] Check Health  [?] Help  [q] Quit",
                theme.key_hint,
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("[Tab]", theme.key_shortcut),
            Span::raw(" Views  "),
            Span::styled("[a]", theme.key_shortcut),
            Span::raw(" Add  "),
            Span::styled("[t]", theme.key_shortcut),
            Span::raw(" Tools  "),
            Span::styled("[T]", theme.key_shortcut),
            Span::raw(" Handshake  "),
            Span::styled("[u]", theme.key_shortcut),
            Span::raw(" Sync  "),
            Span::styled("[Space]", theme.key_shortcut),
            Span::raw(" Toggle  "),
            Span::styled("[d]", theme.key_shortcut),
            Span::raw(" Del  "),
            Span::styled("[v]", theme.key_shortcut),
            Span::raw(" Snippet  "),
            Span::styled("[r]", theme.key_shortcut),
            Span::raw(" Health  "),
            Span::styled("[/]", theme.key_shortcut),
            Span::raw(" Search  "),
            Span::styled("[?]", theme.key_shortcut),
            Span::raw(" Help  "),
            Span::styled("[q]", theme.key_shortcut),
            Span::raw(" Quit"),
        ])
    };

    let p = Paragraph::new(line);
    f.render_widget(p, area);
}

pub fn render_snippet_modal(f: &mut Frame, app: &App) {
    let server = match app.selected_server() {
        Some(s) => s,
        None => return,
    };

    let theme = Theme::default();
    let area = crate::ui::layout::centered_rect(72, 65, f.area());
    f.render_widget(ratatui::widgets::Clear, area);

    let title = format!(" CANONICAL CONFIGURATION SNIPPET: {} ", server.id);
    let block = Block::default()
        .title(title)
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(theme.border);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let snippet_json = match &server.transport {
        Transport::Stdio { command, args, env } => {
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
        Transport::StreamableHttp { url, headers } => {
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
        Transport::Sse { url } => serde_json::to_string_pretty(&serde_json::json!({
            server.id.clone(): {
                "type": "sse",
                "url": url,
            }
        }))
        .unwrap_or_default(),
    };

    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Min(5),
            ratatui::layout::Constraint::Length(2),
        ])
        .split(inner);

    let code = Paragraph::new(snippet_json)
        .style(Style::default().fg(Color::Rgb(248, 248, 242)))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(code, chunks[0]);

    let footer_line = Line::from(vec![
        Span::styled(" [Esc] / [v] / [Enter] ", theme.key_shortcut),
        Span::raw(" Close Snippet Modal"),
    ]);
    f.render_widget(Paragraph::new(footer_line), chunks[1]);
}
