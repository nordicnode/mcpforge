use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, WizardSource, WizardStep, CATEGORY_LABELS};
use crate::ui::theme::Theme;

pub fn render_wizard(f: &mut Frame, app: &App) {
    let wizard = match &app.wizard_state {
        Some(w) => w,
        None => return,
    };

    let theme = Theme::default();
    let area = centered_rect(88, 88, f.area());
    f.render_widget(Clear, area);

    let (step_num, step_title) = match wizard.step {
        WizardStep::SelectSource => (1, "Choose Server Source"),
        WizardStep::ConfigureServer => (2, "Browse Catalog & Configure Parameters"),
        WizardStep::SelectTargets => (3, "Select Client Targets & Harnesses"),
        WizardStep::PreviewDiff => (4, "Preview Configuration Diff & Apply"),
    };

    let title = format!(" ADD MCP SERVER (Step {}/4: {}) ", step_num, step_title);

    let block = Block::default()
        .title(title)
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(theme.border_focus);

    let inner = block.inner(area);
    f.render_widget(block, area);

    match wizard.step {
        WizardStep::SelectSource => {
            let sources = [
                (
                    "1. Curated MCP Registry Catalog (Recommended)",
                    "Browse 50+ pre-tested, categorized official and community servers with instant auto-config",
                    WizardSource::FromRegistry,
                ),
                (
                    "2. Paste JSON Definition",
                    "Paste configuration JSON directly from documentation, GitHub READMEs, or other clients",
                    WizardSource::PasteJson,
                ),
                (
                    "3. Custom Command Line",
                    "Specify custom executable, CLI arguments, and environment variables manually",
                    WizardSource::Manual,
                ),
            ];

            let items: Vec<ListItem> = sources
                .iter()
                .enumerate()
                .map(|(i, (name, desc, src))| {
                    let is_selected = *src == wizard.source
                        || (i == 0 && matches!(wizard.source, WizardSource::FromRegistry));

                    let cursor = if is_selected { " ▶ [x] " } else { "   [ ] " };
                    let style = if is_selected {
                        theme.selected
                    } else {
                        Style::default().fg(Color::White)
                    };

                    let line = Line::from(vec![
                        Span::styled(
                            cursor,
                            if is_selected {
                                theme.title
                            } else {
                                theme.muted
                            },
                        ),
                        Span::styled(*name, style),
                    ]);

                    let sub_line = Line::from(vec![
                        Span::raw("         "),
                        Span::styled(*desc, theme.muted),
                    ]);

                    ListItem::new(vec![line, sub_line, Line::raw("")])
                })
                .collect();

            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(theme.border_type)
                    .border_style(theme.border)
                    .title(" Select Source [j/k or Up/Down: move cursor, Enter: continue, Esc: cancel] "),
            );
            f.render_widget(list, inner);
        }

        WizardStep::ConfigureServer => match wizard.source {
            WizardSource::FromRegistry => {
                let cat_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // Category Bar
                        Constraint::Min(8),    // Split List & Specs
                    ])
                    .split(inner);

                // 1. Category Tabs Bar
                let mut cat_spans = vec![Span::styled(" Categories: ", theme.header)];

                for (idx, label) in CATEGORY_LABELS.iter().enumerate() {
                    let is_active = idx == wizard.registry_category_index;
                    let tab_style = if is_active {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Rgb(139, 233, 253))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Rgb(150, 155, 175))
                    };

                    cat_spans.push(Span::styled(
                        format!(" [{}] {} ", idx + 1, label),
                        tab_style,
                    ));
                    cat_spans.push(Span::raw(" "));
                }

                let cat_bar = Paragraph::new(Line::from(cat_spans)).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(theme.border_type)
                        .border_style(theme.border)
                        .title(" Filter by Category [Press 1-8 or Tab/BackTab to switch] "),
                );
                f.render_widget(cat_bar, cat_chunks[0]);

                // 2. Main 2-Column Split: Catalog List (Left) + Inspector Card (Right)
                let split = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(44), Constraint::Percentage(56)])
                    .split(cat_chunks[1]);

                let entries = app.filtered_registry_entries();
                let total = entries.len();
                let visible_height = (split[0].height as usize).saturating_sub(2).max(1);
                let (start, end) = crate::ui::layout::calculate_scroll_window(
                    total,
                    wizard.registry_cursor,
                    visible_height,
                );

                let items: Vec<ListItem> = entries[start..end]
                    .iter()
                    .enumerate()
                    .map(|(offset, entry)| {
                        let real_idx = start + offset;
                        let is_selected = real_idx == wizard.registry_cursor;
                        let prefix = if is_selected { "▶ " } else { "  " };
                        let style = if is_selected {
                            theme.selected
                        } else {
                            Style::default().fg(Color::White)
                        };

                        let cat_pill_style = match entry.category.as_str() {
                            "ai-agent" => Style::default().fg(Color::Rgb(189, 147, 249)),
                            "dev tools" => Style::default().fg(Color::Rgb(139, 233, 253)),
                            "data" => Style::default().fg(Color::Rgb(241, 250, 140)),
                            "web" => Style::default().fg(Color::Rgb(80, 250, 123)),
                            "git" => Style::default().fg(Color::Rgb(255, 121, 198)),
                            "cloud" => Style::default().fg(Color::Rgb(255, 184, 108)),
                            _ => theme.muted,
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
                            Span::styled(format!("[{}] ", entry.category), cat_pill_style),
                            Span::styled(&entry.name, style),
                            Span::raw(" "),
                            Span::styled(
                                format!("[{}]", entry.id),
                                Style::default().fg(Color::Rgb(139, 233, 253)),
                            ),
                        ]);
                        ListItem::new(line)
                    })
                    .collect();

                let active_cat_name = CATEGORY_LABELS
                    .get(wizard.registry_category_index)
                    .copied()
                    .unwrap_or("All");

                let list_title = format!(
                    " {} ({}/{}) ",
                    active_cat_name,
                    if total > 0 {
                        wizard.registry_cursor + 1
                    } else {
                        0
                    },
                    total
                );

                let list = List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(theme.border_type)
                        .border_style(theme.border)
                        .title(list_title),
                );
                f.render_widget(list, split[0]);

                // Right: Server Inspector Card
                let selected_entry = entries.get(wizard.registry_cursor);
                let details_block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(theme.border_type)
                    .border_style(theme.border)
                    .title(" Server Specifications ");

                let mut details_lines = Vec::new();

                if let Some(entry) = selected_entry {
                    details_lines.push(Line::from(vec![
                        Span::styled(
                            &entry.name,
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            format!("id: {}", entry.id),
                            Style::default().fg(Color::Rgb(139, 233, 253)),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            format!("category: {}", entry.category),
                            Style::default().fg(Color::Rgb(189, 147, 249)),
                        ),
                    ]));

                    if let Some(ref repo) = entry.official_repo {
                        details_lines.push(Line::from(vec![
                            Span::styled("Official Repo: ", theme.header),
                            Span::styled(repo, Style::default().fg(Color::Rgb(139, 233, 253))),
                        ]));
                    }

                    if let Some(ref maintainer) = entry.maintainer {
                        details_lines.push(Line::from(vec![
                            Span::styled("Maintainer:    ", theme.header),
                            Span::styled(maintainer, Style::default().fg(Color::Rgb(80, 250, 123))),
                        ]));
                    }

                    if let Some(ref verified) = entry.last_verified {
                        details_lines.push(Line::from(vec![
                            Span::styled("Verified:      ", theme.header),
                            Span::styled(
                                format!("{} [Schema Audited]", verified),
                                Style::default().fg(Color::Rgb(241, 250, 140)),
                            ),
                        ]));
                    }
                    details_lines.push(Line::raw(""));

                    details_lines.push(Line::from(vec![Span::styled(
                        "Description: ",
                        theme.header,
                    )]));
                    details_lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(&entry.description, Style::default().fg(Color::White)),
                    ]));
                    details_lines.push(Line::raw(""));

                    details_lines.push(Line::from(vec![
                        Span::styled("Command:     ", theme.header),
                        Span::styled(
                            &entry.command,
                            Style::default().fg(Color::Rgb(139, 233, 253)),
                        ),
                    ]));

                    if !entry.args.is_empty() {
                        details_lines.push(Line::from(vec![
                            Span::styled("Arguments:   ", theme.header),
                            Span::styled(entry.args.join(" "), Style::default().fg(Color::White)),
                        ]));
                    }

                    if !entry.required_env.is_empty() {
                        details_lines.push(Line::raw(""));
                        details_lines.push(Line::from(vec![Span::styled(
                            "Required Environment Variables:",
                            theme.header,
                        )]));
                        for env_key in &entry.required_env {
                            let is_set = std::env::var(env_key).is_ok();
                            let (status_icon, status_style, status_note) = if is_set {
                                ("✓", theme.status_healthy, "detected in shell environment")
                            } else {
                                ("[ ! ]", theme.status_degraded, "not set in current shell")
                            };
                            details_lines.push(Line::from(vec![
                                Span::raw("  • "),
                                Span::styled(format!("{} ", status_icon), status_style),
                                Span::styled(
                                    env_key,
                                    Style::default().fg(Color::Rgb(241, 250, 140)),
                                ),
                                Span::styled(format!(" ({})", status_note), status_style),
                            ]));
                        }
                    }

                    details_lines.push(Line::raw(""));
                    details_lines.push(Line::from(vec![
                        Span::styled("Action:      ", theme.header),
                        Span::styled(
                            "Press [Enter] to choose target clients for this server.",
                            theme.key_shortcut,
                        ),
                    ]));
                } else {
                    details_lines.push(Line::from(vec![Span::styled(
                        "No servers found in this category.",
                        theme.muted,
                    )]));
                }

                let details_p = Paragraph::new(details_lines)
                    .block(details_block)
                    .wrap(Wrap { trim: false });
                f.render_widget(details_p, split[1]);
            }

            WizardSource::Manual => {
                let lines = vec![
                    Line::from(vec![Span::styled(
                        "Configure custom server parameters:",
                        theme.header,
                    )]),
                    Line::raw(""),
                    Line::from(vec![
                        Span::styled(
                            "  Server ID: ",
                            Style::default().fg(Color::Rgb(139, 233, 253)),
                        ),
                        Span::styled(
                            if wizard.server_id.is_empty() {
                                "(required)"
                            } else {
                                &wizard.server_id
                            },
                            Style::default().fg(Color::White),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "  Command:   ",
                            Style::default().fg(Color::Rgb(139, 233, 253)),
                        ),
                        Span::styled(
                            if wizard.command.is_empty() {
                                "(e.g. npx, uvx, node)"
                            } else {
                                &wizard.command
                            },
                            Style::default().fg(Color::White),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "  Arguments: ",
                            Style::default().fg(Color::Rgb(139, 233, 253)),
                        ),
                        Span::styled(
                            if wizard.args.is_empty() {
                                "(optional CLI arguments)"
                            } else {
                                &wizard.args
                            },
                            Style::default().fg(Color::White),
                        ),
                    ]),
                    Line::raw(""),
                    Line::from(vec![Span::styled(
                        "Press [Enter] to proceed to target client selection.",
                        theme.key_shortcut,
                    )]),
                ];

                let p = Paragraph::new(lines).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(theme.border_type)
                        .border_style(theme.border)
                        .title(" Custom Server Details "),
                );
                f.render_widget(p, inner);
            }

            WizardSource::PasteJson => {
                let mut lines = Vec::new();
                lines.push(Line::from(vec![Span::styled(
                    "Paste or edit your MCP JSON definition below:",
                    theme.header,
                )]));
                lines.push(Line::raw(""));
                lines.push(Line::styled(
                    if wizard.pasted_json.is_empty() {
                        "{\n  \"command\": \"npx\",\n  \"args\": [\"-y\", \"@modelcontextprotocol/server-example\"]\n}"
                    } else {
                        &wizard.pasted_json
                    },
                    Style::default().fg(Color::White),
                ));
                if let Some(ref err) = wizard.error_message {
                    lines.push(Line::raw(""));
                    lines.push(Line::styled(
                        format!("✖ Error: {}", err),
                        theme.status_broken,
                    ));
                }
                let p = Paragraph::new(lines).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(theme.border_type)
                        .border_style(theme.border)
                        .title(" JSON Definition Input "),
                );
                f.render_widget(p, inner);
            }
        },

        WizardStep::SelectTargets => {
            let total = wizard.target_locations.len();
            let visible_height = (inner.height as usize).saturating_sub(4).max(1);
            let (start, end) = crate::ui::layout::calculate_scroll_window(
                total,
                wizard.target_cursor,
                visible_height,
            );

            let selected_count = wizard.target_locations.iter().filter(|(_, s)| *s).count();

            let items: Vec<ListItem> = wizard.target_locations[start..end]
                .iter()
                .enumerate()
                .map(|(offset, (loc, selected))| {
                    let real_idx = start + offset;
                    let is_cursor = real_idx == wizard.target_cursor;

                    let checkbox = if *selected { "[✓] " } else { "[ ] " };
                    let check_style = if *selected {
                        theme.status_healthy
                    } else {
                        theme.muted
                    };

                    let cursor_str = if is_cursor { "▶ " } else { "  " };

                    let is_running = app.running_processes.contains(&loc.client_id);
                    let mut spans = vec![
                        Span::styled(
                            cursor_str,
                            if is_cursor { theme.title } else { theme.muted },
                        ),
                        Span::styled(checkbox, check_style),
                        Span::styled(
                            &loc.display_name,
                            if is_cursor {
                                theme.selected
                            } else {
                                Style::default().fg(Color::White)
                            },
                        ),
                    ];

                    if is_running {
                        spans.push(Span::styled(" [ACTIVE]", theme.pill_active));
                    } else if loc.exists {
                        spans.push(Span::styled(" [READY]", theme.pill_ready));
                    } else {
                        spans.push(Span::styled(" [AVAIL]", theme.pill_avail));
                    }

                    spans.push(Span::styled(
                        format!(" · {}", loc.path.display()),
                        theme.muted,
                    ));

                    ListItem::new(Line::from(spans))
                })
                .collect();

            let title = format!(
                " Target Clients & Harnesses (Selected: {}/{} | Cursor: {}/{}) [j/k: move, Space: toggle, a: all, n: none, Enter: next] ",
                selected_count,
                total,
                if total > 0 { wizard.target_cursor + 1 } else { 0 },
                total
            );

            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(theme.border_type)
                    .border_style(theme.border)
                    .title(title),
            );
            f.render_widget(list, inner);
        }

        WizardStep::PreviewDiff => {
            let diff_text = if wizard.diff_preview.is_empty() {
                "No target clients selected or no configuration modifications required.".to_string()
            } else {
                wizard.diff_preview.clone()
            };

            let mut lines: Vec<Line> = diff_text
                .lines()
                .map(|line| {
                    if line.starts_with('+') && !line.starts_with("+++") {
                        Line::styled(line, theme.status_healthy)
                    } else if line.starts_with('-') && !line.starts_with("---") {
                        Line::styled(line, theme.status_broken)
                    } else if line.starts_with("---") || line.starts_with("+++") {
                        Line::styled(line, theme.header)
                    } else if line.starts_with("@@") {
                        Line::styled(line, Style::default().fg(Color::Rgb(139, 233, 253)))
                    } else {
                        Line::styled(line, theme.muted)
                    }
                })
                .collect();

            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::styled("CONFIRMATION: ", theme.header),
                Span::styled(
                    "Press [Enter] to write configuration with atomic safety & backups, or [Esc] to go back.",
                    theme.key_shortcut,
                ),
            ]));

            let p = Paragraph::new(lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(theme.border_type)
                        .border_style(theme.border)
                        .title(" Unified Configuration Diff Preview "),
                )
                .wrap(Wrap { trim: false });
            f.render_widget(p, inner);
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
