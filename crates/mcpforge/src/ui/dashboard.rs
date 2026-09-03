use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, DetailTab, FocusedPane};
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
    let is_focused = app.focused_pane == FocusedPane::ServersList;
    let border_style = if is_focused {
        theme.border_focus
    } else {
        theme.border_inactive
    };

    let filtered = app.filtered_servers();
    let total = filtered.len();

    // If searching, split top for filter input
    let (filter_area, list_area) = if app.is_searching {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(5)])
            .split(area);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, area)
    };

    if let Some(fa) = filter_area {
        let filter_block = Block::default()
            .title(Span::styled(" 🔍 FILTER SERVERS ", theme.header))
            .borders(Borders::ALL)
            .border_type(theme.border_type)
            .border_style(theme.border_focus);

        let filter_line = Line::from(vec![
            Span::raw(" "),
            Span::styled(
                &app.search_query,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("█", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("  ({}/{} matches) · Esc to clear", total, app.servers.len()),
                theme.muted,
            ),
        ]);
        f.render_widget(Paragraph::new(filter_line).block(filter_block), fa);
    }

    let visible_height = (list_area.height as usize).saturating_sub(2).max(1);
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
                Span::styled(format!("{:<18}", server.id), name_style),
                Span::raw(" "),
                Span::styled(
                    format!("[{}]", server.transport.transport_type_str()),
                    theme.pill_transport,
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

    let focus_hint = if is_focused { " [Focused] " } else { "" };
    let list_block = Block::default()
        .title(Span::styled(
            format!("{}{}", title, focus_hint),
            if is_focused { theme.title } else { theme.muted },
        ))
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(border_style);

    let list = List::new(items)
        .block(list_block)
        .highlight_style(theme.selected);

    f.render_widget(list, list_area);
}

fn render_server_details(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let is_focused = app.focused_pane == FocusedPane::ServerDetails;
    let border_style = if is_focused {
        theme.border_focus
    } else {
        theme.border_inactive
    };

    let server = match app.selected_server() {
        Some(s) => s,
        None => {
            let empty_block = Block::default()
                .title(" SERVER INSPECTOR ")
                .borders(Borders::ALL)
                .border_type(theme.border_type)
                .border_style(border_style);
            let empty = Paragraph::new(
                "\n  No servers configured or none match search filter.\n\n  • Press [a] to launch the Add Server Wizard.\n  • Press [Tab] to view discovered Clients & Harnesses.",
            )
            .block(empty_block)
            .style(theme.muted);
            f.render_widget(empty, area);
            return;
        }
    };

    let title = format!(
        " SERVER INSPECTOR: {} {}",
        server.id,
        if is_focused { "[Active Focus]" } else { "" }
    );

    let outer_block = Block::default()
        .title(Span::styled(
            title,
            if is_focused { theme.title } else { theme.muted },
        ))
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(border_style);

    let inner = outer_block.inner(area);
    f.render_widget(outer_block, area);

    // Split inner into:
    // 0: Segmented Sub-Tab Bar (Length 2)
    // 1: Tab Content Pane (Min 5)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(5)])
        .split(inner);

    // 1. Sub-Tab Bar
    let env_count = match &server.transport {
        Transport::Stdio { env, .. } => env.len(),
        Transport::StreamableHttp { headers, .. } => headers.len(),
        _ => 0,
    };

    let tabs = [
        (DetailTab::Overview, "1: Overview".to_string()),
        (
            DetailTab::Clients,
            format!("2: Clients ({})", server.clients.len()),
        ),
        (DetailTab::Environment, format!("3: Env ({})", env_count)),
        (DetailTab::Telemetry, "4: Telemetry".to_string()),
        (DetailTab::ConfigJson, "5: Config JSON".to_string()),
    ];

    let mut tab_spans = Vec::new();
    for (tab_type, title) in &tabs {
        let is_active = *tab_type == app.detail_tab;
        let style = if is_active {
            theme.tab_active
        } else {
            theme.tab_inactive
        };
        tab_spans.push(Span::styled(format!(" {} ", title), style));
        tab_spans.push(Span::raw(" "));
    }

    let hint_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        theme.muted
    };
    tab_spans.push(Span::styled(
        " · [1-5 / Tab] Switch Tab · [h/Esc] Back ",
        hint_style,
    ));

    f.render_widget(Paragraph::new(Line::from(tab_spans)), chunks[0]);

    // 2. Tab Content Pane
    let content_area = chunks[1];
    match app.detail_tab {
        DetailTab::Overview => render_overview_tab(f, app, server, content_area, theme),
        DetailTab::Clients => render_clients_tab(f, app, server, content_area, theme),
        DetailTab::Environment => render_environment_tab(f, server, content_area, theme),
        DetailTab::Telemetry => render_telemetry_tab(f, app, server, content_area, theme),
        DetailTab::ConfigJson => render_config_json_tab(f, app, server, content_area, theme),
    }
}

fn render_overview_tab(
    f: &mut Frame,
    app: &App,
    server: &mcp_core::types::ServerEntry,
    area: Rect,
    theme: &Theme,
) {
    let mut lines = Vec::new();

    // 1. Status Summary Card
    let (status_badge, status_style) = if server.enabled {
        ("ENABLED", theme.status_healthy)
    } else {
        ("DISABLED", theme.status_disabled)
    };

    let health = app
        .health_cache
        .get(&server.id)
        .cloned()
        .unwrap_or(HealthStatus::Unknown);

    lines.push(Line::from(vec![
        Span::styled("  Status:        ", theme.key_shortcut),
        Span::styled(format!("[{}]", status_badge), status_style),
        Span::raw("     "),
        Span::styled("Transport:   ", theme.key_shortcut),
        Span::styled(
            format!("[{}]", server.transport.transport_type_str()),
            theme.pill_transport,
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Performance:   ", theme.key_shortcut),
        Span::styled(
            format!("[{}]", health.performance_badge()),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled("Exposed:     ", theme.key_shortcut),
        Span::styled(
            match health {
                HealthStatus::Healthy { tool_count, .. } => format!("{} tools active", tool_count),
                _ => "Tools unprobed".to_string(),
            },
            theme.muted,
        ),
    ]));
    lines.push(Line::raw(""));

    // 2. Execution Command
    lines.push(Line::from(Span::styled(
        "EXECUTION COMMAND & PARAMETERS",
        theme.card_header,
    )));
    match &server.transport {
        Transport::Stdio { command, args, .. } => {
            lines.push(Line::from(vec![
                Span::styled("  Command:   ", theme.key_shortcut),
                Span::styled(
                    command,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            if !args.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("  Arguments: ", theme.key_shortcut),
                    Span::styled(args.join(" "), Style::default().fg(Color::White)),
                ]));
            }
        }
        Transport::StreamableHttp { url, .. } => {
            lines.push(Line::from(vec![
                Span::styled("  Endpoint:  ", theme.key_shortcut),
                Span::styled(url, Style::default().fg(Color::White)),
            ]));
        }
        Transport::Sse { url } => {
            lines.push(Line::from(vec![
                Span::styled("  Endpoint:  ", theme.key_shortcut),
                Span::styled(url, Style::default().fg(Color::White)),
                Span::styled(" (Legacy SSE)", theme.status_degraded),
            ]));
        }
    }
    lines.push(Line::raw(""));

    // 3. Upstream Registry Provenance (if catalog server)
    if let Some(cat_entry) = app.registry.find_by_id(&server.id) {
        lines.push(Line::from(Span::styled(
            "UPSTREAM REGISTRY PROVENANCE",
            theme.card_header,
        )));
        let maintainer = cat_entry.maintainer.as_deref().unwrap_or("Community");
        let last_verified = cat_entry.last_verified.as_deref().unwrap_or("Unverified");
        let source_url = cat_entry.source_url.as_deref().unwrap_or("N/A");

        lines.push(Line::from(vec![
            Span::styled("  Maintainer: ", theme.key_shortcut),
            Span::styled(maintainer, Style::default().fg(Color::White)),
            Span::raw("   "),
            Span::styled("Category:   ", theme.key_shortcut),
            Span::styled(&cat_entry.category, Style::default().fg(Color::Cyan)),
            Span::raw("   "),
            Span::styled("Verified:   ", theme.key_shortcut),
            Span::styled(last_verified, theme.status_healthy),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Source:     ", theme.key_shortcut),
            Span::styled(source_url, Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::raw(""));
    }

    // 4. Notes (if any)
    if let Some(ref notes) = server.notes {
        lines.push(Line::from(Span::styled(
            "OPERATOR NOTES",
            theme.card_header,
        )));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(notes, Style::default().fg(Color::White)),
        ]));
        lines.push(Line::raw(""));
    }

    // 5. Quick action hints
    lines.push(Line::from(vec![
        Span::styled("Quick Actions: ", theme.muted),
        Span::styled("[t] ", theme.key_shortcut),
        Span::raw("Playground & Form Builder  "),
        Span::styled("[T] ", theme.key_shortcut),
        Span::raw("Handshake Ping  "),
        Span::styled("[u] ", theme.key_shortcut),
        Span::raw("Auto-Sync Clients"),
    ]));

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn render_clients_tab(
    f: &mut Frame,
    app: &App,
    server: &mcp_core::types::ServerEntry,
    area: Rect,
    theme: &Theme,
) {
    let mut lines = vec![
        Line::from(Span::styled(
            "CONFIGURED IN THE FOLLOWING CLIENT HARNESSES:",
            theme.card_header,
        )),
        Line::raw(""),
    ];

    if server.clients.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "This server is not configured in any client yet.",
                theme.muted,
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "Press [u] to automatically sync this server across all detected clients.",
                Style::default().fg(Color::Yellow),
            ),
        ]));
    } else {
        for client in &server.clients {
            let is_running = app.running_processes.contains(&client.client_id);
            let (status_chip, status_style) = if is_running {
                ("[ACTIVE]", theme.pill_active)
            } else {
                ("[READY] ", theme.pill_ready)
            };

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(status_chip, status_style),
                Span::raw(" "),
                Span::styled(
                    format!("{:<28}", client.display_name),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" {}", client.config_path.display()), theme.muted),
            ]));
        }

        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Tip: ", theme.key_shortcut),
            Span::raw("Press "),
            Span::styled("[u]", theme.key_shortcut),
            Span::raw(" to mirror changes across all other active client harnesses."),
        ]));
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn render_environment_tab(
    f: &mut Frame,
    server: &mcp_core::types::ServerEntry,
    area: Rect,
    theme: &Theme,
) {
    let mut lines = Vec::new();

    match &server.transport {
        Transport::Stdio { env, .. } => {
            lines.push(Line::from(Span::styled(
                format!("ENVIRONMENT VARIABLES ({} CONFIGURED):", env.len()),
                theme.card_header,
            )));
            lines.push(Line::raw(""));

            if env.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "No custom environment variables required or configured.",
                        theme.muted,
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{:<30} {:<30} STATUS", "VARIABLE", "VALUE"),
                        theme.muted,
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("─".repeat(70), theme.border_inactive),
                ]));

                for (k, v) in env {
                    let is_sec = is_secret_key(k);
                    let display_v = if is_sec {
                        redact_secret(v)
                    } else if v.is_empty() {
                        "(empty string)".to_string()
                    } else {
                        v.clone()
                    };

                    let (status_text, status_style) = if v.is_empty() {
                        ("MISSING", theme.status_broken)
                    } else if is_sec {
                        ("PROTECTED", theme.status_healthy)
                    } else {
                        ("SET", Style::default().fg(Color::White))
                    };

                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            format!("{:<30}", k),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("{:<30}", display_v),
                            Style::default().fg(Color::White),
                        ),
                        Span::raw(" "),
                        Span::styled(status_text, status_style),
                    ]));
                }
            }
        }
        Transport::StreamableHttp { headers, .. } => {
            lines.push(Line::from(Span::styled(
                format!("HTTP HEADERS ({} CONFIGURED):", headers.len()),
                theme.card_header,
            )));
            lines.push(Line::raw(""));
            if headers.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("No custom HTTP headers configured.", theme.muted),
                ]));
            } else {
                for (k, v) in headers {
                    let display_v = if is_secret_key(k) {
                        redact_secret(v)
                    } else {
                        v.clone()
                    };
                    lines.push(Line::from(vec![
                        Span::raw("  • "),
                        Span::styled(
                            format!("{}: ", k),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(display_v, Style::default().fg(Color::White)),
                    ]));
                }
            }
        }
        Transport::Sse { .. } => {
            lines.push(Line::from(Span::styled(
                "SSE endpoint does not utilize local environment variables.",
                theme.muted,
            )));
        }
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn render_telemetry_tab(
    f: &mut Frame,
    app: &App,
    server: &mcp_core::types::ServerEntry,
    area: Rect,
    theme: &Theme,
) {
    let health = app
        .health_cache
        .get(&server.id)
        .cloned()
        .unwrap_or(HealthStatus::Unknown);

    let mut lines = vec![
        Line::from(Span::styled(
            "RUNTIME HEALTH & PROTOCOL TELEMETRY",
            theme.card_header,
        )),
        Line::raw(""),
    ];

    let perf_badge = health.performance_badge();
    match health {
        HealthStatus::Healthy {
            latency_ms,
            tool_count,
            ref server_name,
            ref server_version,
        } => {
            lines.push(Line::from(vec![
                Span::styled("  Health Status:         ", theme.key_shortcut),
                Span::styled("● Connected & Operational", theme.status_healthy),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Performance Grade:     ", theme.key_shortcut),
                Span::styled(
                    format!("{} ({}ms roundtrip latency)", perf_badge, latency_ms),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Server Implementation: ", theme.key_shortcut),
                Span::styled(
                    format!("{} v{}", server_name, server_version),
                    Style::default().fg(Color::White),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Exposed Tools:         ", theme.key_shortcut),
                Span::styled(
                    format!(
                        "{} tools available (Press [t] to test in Playground)",
                        tool_count
                    ),
                    Style::default().fg(Color::Cyan),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Protocol Wire:         ", theme.key_shortcut),
                Span::styled("JSON-RPC 2.0 via async Stdio / HTTP", theme.muted),
            ]));
        }
        HealthStatus::Degraded { reason, latency_ms } => {
            lines.push(Line::from(vec![
                Span::styled("  Health Status:         ", theme.key_shortcut),
                Span::styled("▲ Degraded", theme.status_degraded),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Diagnosis:             ", theme.key_shortcut),
                Span::styled(reason, Style::default().fg(Color::White)),
            ]));
            if let Some(ms) = latency_ms {
                lines.push(Line::from(vec![
                    Span::styled("  Latency:               ", theme.key_shortcut),
                    Span::styled(format!("{}ms", ms), theme.status_degraded),
                ]));
            }
        }
        HealthStatus::Broken { error } => {
            lines.push(Line::from(vec![
                Span::styled("  Health Status:         ", theme.key_shortcut),
                Span::styled("✖ Broken / Failed Handshake", theme.status_broken),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Error Diagnostics:     ", theme.key_shortcut),
                Span::styled(error, Style::default().fg(Color::Red)),
            ]));
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Tip: ", theme.key_shortcut),
                Span::raw("Run "),
                Span::styled("mcpforge doctor --diagnose", theme.title),
                Span::raw(" to automatically detect missing packages or dependencies."),
            ]));
        }
        HealthStatus::Disabled => {
            lines.push(Line::from(vec![
                Span::styled("  Health Status:         ", theme.key_shortcut),
                Span::styled(
                    "○ Server is currently disabled by user toggle",
                    theme.status_disabled,
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  Press "),
                Span::styled("[Space]", theme.key_shortcut),
                Span::raw(" to re-enable this server across all clients."),
            ]));
        }
        HealthStatus::Unknown => {
            lines.push(Line::from(vec![
                Span::styled("  Health Status:         ", theme.key_shortcut),
                Span::styled("? Unchecked", theme.muted),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  Press "),
                Span::styled("[r]", theme.key_shortcut),
                Span::raw(" to perform an instant health check, or "),
                Span::styled("[T]", theme.key_shortcut),
                Span::raw(" for an in-depth JSON-RPC handshake test."),
            ]));
        }
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn render_config_json_tab(
    f: &mut Frame,
    app: &App,
    server: &mcp_core::types::ServerEntry,
    area: Rect,
    _theme: &Theme,
) {
    let snippet_json = App::generate_canonical_snippet(server);

    let total_lines = snippet_json.lines().count();
    let visible_lines = area.height as usize;
    let scroll = app
        .detail_scroll
        .min(total_lines.saturating_sub(visible_lines));

    let display_lines: Vec<Line> = snippet_json
        .lines()
        .skip(scroll)
        .take(visible_lines)
        .map(|l| {
            Line::from(vec![
                Span::raw("  "),
                Span::styled(l, Style::default().fg(Color::Rgb(248, 248, 242))),
            ])
        })
        .collect();

    f.render_widget(Paragraph::new(display_lines), area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(36)])
        .split(area);

    let left_line = if app.is_searching {
        Line::from(vec![
            Span::styled(" FILTER: ", theme.header),
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
        ])
    } else if app.focused_pane == FocusedPane::ServerDetails {
        Line::from(vec![
            Span::styled("[h / Esc]", theme.key_shortcut),
            Span::raw(" Back  "),
            Span::styled("[1-5 / Tab]", theme.key_shortcut),
            Span::raw(" Tabs  "),
            Span::styled("[j/k]", theme.key_shortcut),
            Span::raw(" Scroll  "),
            Span::styled("[c]", theme.key_shortcut),
            Span::raw(" Copy Config  "),
            Span::styled("[t]", theme.key_shortcut),
            Span::raw(" Playground"),
        ])
    } else {
        Line::from(vec![
            Span::styled("[Enter / l]", theme.key_shortcut),
            Span::raw(" Inspect  "),
            Span::styled("[Space]", theme.key_shortcut),
            Span::raw(" Toggle  "),
            Span::styled("[t]", theme.key_shortcut),
            Span::raw(" Playground  "),
            Span::styled("[/]", theme.key_shortcut),
            Span::raw(" Filter  "),
            Span::styled("[a]", theme.key_shortcut),
            Span::raw(" Add  "),
            Span::styled("[b]", theme.key_shortcut),
            Span::raw(" Backups  "),
            Span::styled("[?]", theme.key_shortcut),
            Span::raw(" Help"),
        ])
    };

    let installed_count = app
        .discovered_clients
        .iter()
        .filter(|c| c.is_installed)
        .count();

    let right_line = Line::from(vec![Span::styled(
        format!(
            "{} servers · {}/{} clients · v0.1.0 ",
            app.servers.len(),
            installed_count,
            app.discovered_clients.len()
        ),
        theme.muted,
    )]);

    f.render_widget(Paragraph::new(left_line), chunks[0]);
    f.render_widget(Paragraph::new(right_line), chunks[1]);
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

    let snippet_json = App::generate_canonical_snippet(server);

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
