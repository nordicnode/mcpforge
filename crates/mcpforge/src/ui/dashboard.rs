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

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(idx, server)| {
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

            let is_selected = idx == app.selected_index;
            let name_style = if is_selected {
                theme.selected
            } else if !server.enabled {
                theme.status_disabled
            } else {
                Style::default().fg(Color::White)
            };

            let line = Line::from(vec![
                Span::styled(format!(" {} ", icon), icon_style),
                Span::styled(&server.id, name_style),
                Span::raw(" "),
                Span::styled(
                    format!("({})", server.transport.transport_type_str()),
                    theme.muted,
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list_block = Block::default()
        .title(" SERVERS ")
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_style(theme.border);

    let list = List::new(items)
        .block(list_block)
        .highlight_style(theme.selected);

    f.render_widget(list, area);
}

fn render_server_details(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let block = Block::default()
        .title(" DETAILS & CLIENT STATUS ")
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_style(theme.border);

    let server = match app.selected_server() {
        Some(s) => s,
        None => {
            let empty = Paragraph::new("No servers configured or none match search filter.\nPress [a] to add an MCP server.")
                .block(block)
                .style(theme.muted);
            f.render_widget(empty, area);
            return;
        }
    };

    let mut lines = Vec::new();

    // 1. Title & ID
    lines.push(Line::from(vec![
        Span::styled(
            "Server: ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &server.id,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::raw(""));

    // 2. Transport Details
    match &server.transport {
        Transport::Stdio { command, args, env } => {
            lines.push(Line::from(vec![
                Span::styled("Transport: ", Style::default().fg(Color::Cyan)),
                Span::raw("stdio"),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Command: ", Style::default().fg(Color::LightBlue)),
                Span::styled(command, Style::default().fg(Color::White)),
            ]));
            if !args.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("  Args:    ", Style::default().fg(Color::LightBlue)),
                    Span::raw(args.join(" ")),
                ]));
            }
            if !env.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    "  Env:     ",
                    Style::default().fg(Color::LightBlue),
                )]));
                for (k, v) in env {
                    let display_v = if is_secret_key(k) {
                        redact_secret(v)
                    } else {
                        v.clone()
                    };
                    lines.push(Line::from(vec![
                        Span::raw("    • "),
                        Span::styled(k, Style::default().fg(Color::LightCyan)),
                        Span::raw("="),
                        Span::styled(display_v, Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
        }
        Transport::StreamableHttp { url, headers } => {
            lines.push(Line::from(vec![
                Span::styled("Transport: ", Style::default().fg(Color::Cyan)),
                Span::raw("Streamable HTTP"),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  URL:     ", Style::default().fg(Color::LightBlue)),
                Span::styled(url, Style::default().fg(Color::White)),
            ]));
            if !headers.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    "  Headers: ",
                    Style::default().fg(Color::LightBlue),
                )]));
                for (k, v) in headers {
                    let display_v = if is_secret_key(k) {
                        redact_secret(v)
                    } else {
                        v.clone()
                    };
                    lines.push(Line::from(vec![
                        Span::raw("    • "),
                        Span::styled(k, Style::default().fg(Color::LightCyan)),
                        Span::raw(": "),
                        Span::styled(display_v, Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
        }
        Transport::Sse { url } => {
            lines.push(Line::from(vec![
                Span::styled("Transport: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    "SSE (Legacy - Deprecated)",
                    Style::default().fg(Color::Yellow),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  URL:     ", Style::default().fg(Color::LightBlue)),
                Span::styled(url, Style::default().fg(Color::White)),
            ]));
        }
    }
    lines.push(Line::raw(""));

    // 3. Client Installation Matrix
    lines.push(Line::from(vec![Span::styled(
        "Installed In: ",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )]));

    let mut rendered_any = false;
    for client in &app.detected_clients {
        let is_installed = server.clients.iter().any(|c| c.config_path == client.path);

        if !client.exists && !is_installed {
            continue;
        }

        rendered_any = true;
        let (check_icon, check_style) = if is_installed {
            (
                "✓",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            ("✗", Style::default().fg(Color::DarkGray))
        };

        let is_running = app.running_processes.contains(&client.client_id);
        let mut client_spans = vec![
            Span::raw("  "),
            Span::styled(format!("[{}] ", check_icon), check_style),
            Span::styled(&client.display_name, Style::default().fg(Color::White)),
        ];

        if is_running {
            client_spans.push(Span::styled(
                " [RUNNING]",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        client_spans.push(Span::styled(
            format!(" ({})", client.path.display()),
            theme.muted,
        ));
        lines.push(Line::from(client_spans));
    }

    if !rendered_any {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "None (server not installed in any detected client)",
                theme.muted,
            ),
        ]));
    }
    lines.push(Line::raw(""));

    // 4. Health Status
    lines.push(Line::from(vec![Span::styled(
        "Health Check: ",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
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
            "✓",
            format!(
                "Healthy · {} v{} · {} tools · {}ms",
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
        HealthStatus::Disabled => ("○", "Disabled".to_string(), theme.status_disabled),
        HealthStatus::Unknown => (
            "?",
            "Not checked yet. Press [r] to run health check.".to_string(),
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
            Span::styled("Notes: ", Style::default().fg(Color::LightMagenta)),
            Span::raw(notes),
        ]));
    }

    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(para, area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let text = if app.is_searching {
        format!(
            "Search: {}_ (Enter to apply, Esc to cancel)",
            app.search_query
        )
    } else {
        "[Tab / 2] Clients View  [a]dd  [u] auto-sync  [d]elete  [r] health  [/]search  [?]help  [q]uit".to_string()
    };

    let p = Paragraph::new(text).style(theme.key_hint);
    f.render_widget(p, area);
}
