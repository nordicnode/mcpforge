use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, WizardSource, WizardStep};
use crate::ui::theme::Theme;

pub fn render_wizard(f: &mut Frame, app: &App) {
    let wizard = match &app.wizard_state {
        Some(w) => w,
        None => return,
    };

    let theme = Theme::default();
    let area = centered_rect(80, 80, f.area());
    f.render_widget(Clear, area);

    let title = match wizard.step {
        WizardStep::SelectSource => " ADD SERVER (Step 1/4: Choose Source) ",
        WizardStep::ConfigureServer => " ADD SERVER (Step 2/4: Configure Server) ",
        WizardStep::SelectTargets => " ADD SERVER (Step 3/4: Select Client Targets) ",
        WizardStep::PreviewDiff => " ADD SERVER (Step 4/4: Preview Diff & Confirm) ",
    };

    let block = Block::default()
        .title(title)
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    match wizard.step {
        WizardStep::SelectSource => {
            let sources = [
                (
                    "From Registry",
                    "Browse local-first curated catalog of verified MCP servers",
                ),
                (
                    "Paste JSON Snippet",
                    "Paste configuration JSON (e.g. from GitHub README or docs)",
                ),
                (
                    "Manual Entry",
                    "Specify command, arguments, and environment variables manually",
                ),
            ];

            let items: Vec<ListItem> = sources
                .iter()
                .enumerate()
                .map(|(i, (name, desc))| {
                    let is_selected = matches!(
                        (i, wizard.source),
                        (0, WizardSource::FromRegistry)
                            | (1, WizardSource::PasteJson)
                            | (2, WizardSource::Manual)
                    );

                    let prefix = if is_selected { " ▸ [x] " } else { "   [ ] " };
                    let style = if is_selected {
                        theme.selected
                    } else {
                        Style::default().fg(Color::White)
                    };

                    let line = Line::from(vec![
                        Span::styled(prefix, style),
                        Span::styled(*name, style),
                        Span::raw(" - "),
                        Span::styled(*desc, theme.muted),
                    ]);
                    ListItem::new(line)
                })
                .collect();

            let list = List::new(items).block(
                Block::default().title(" Select Source (Up/Down to choose, Enter to continue) "),
            );
            f.render_widget(list, inner);
        }

        WizardStep::ConfigureServer => match wizard.source {
            WizardSource::FromRegistry => {
                let entries = app.registry.entries();
                let total = entries.len();
                let visible_height = (inner.height as usize).saturating_sub(2).max(1);
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
                        let prefix = if is_selected { " ▸ " } else { "   " };
                        let style = if is_selected {
                            theme.selected
                        } else {
                            Style::default().fg(Color::White)
                        };

                        let line = Line::from(vec![
                            Span::styled(prefix, style),
                            Span::styled(&entry.name, style),
                            Span::styled(
                                format!(" ({})", entry.id),
                                Style::default().fg(Color::LightCyan),
                            ),
                            Span::raw(" - "),
                            Span::styled(&entry.description, theme.muted),
                        ]);
                        ListItem::new(line)
                    })
                    .collect();

                let title = format!(
                    " Curated Catalog ({}/{}) [j/k: browse, Enter: select] ",
                    if total > 0 {
                        wizard.registry_cursor + 1
                    } else {
                        0
                    },
                    total
                );
                let list = List::new(items).block(Block::default().title(title));
                f.render_widget(list, inner);
            }
            WizardSource::Manual => {
                let lines = vec![
                    Line::from("Configure server details:"),
                    Line::raw(""),
                    Line::from(vec![
                        Span::styled("Server ID: ", Style::default().fg(Color::Cyan)),
                        Span::styled(&wizard.server_id, Style::default().fg(Color::White)),
                    ]),
                    Line::from(vec![
                        Span::styled("Command:   ", Style::default().fg(Color::Cyan)),
                        Span::styled(&wizard.command, Style::default().fg(Color::White)),
                    ]),
                    Line::from(vec![
                        Span::styled("Arguments: ", Style::default().fg(Color::Cyan)),
                        Span::styled(&wizard.args, Style::default().fg(Color::White)),
                    ]),
                    Line::raw(""),
                    Line::from(Span::styled(
                        "Press Enter to proceed to target selection.",
                        theme.muted,
                    )),
                ];

                let p = Paragraph::new(lines);
                f.render_widget(p, inner);
            }
            WizardSource::PasteJson => {
                let mut lines = Vec::new();
                lines.push(Line::from("Paste or edit JSON definition:"));
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
                    lines.push(Line::styled(format!("Error: {}", err), theme.status_broken));
                }
                let p = Paragraph::new(lines);
                f.render_widget(p, inner);
            }
        },

        WizardStep::SelectTargets => {
            let items: Vec<ListItem> = wizard
                .target_locations
                .iter()
                .map(|(loc, selected)| {
                    let checkbox = if *selected { "[✓] " } else { "[ ] " };
                    let style = if *selected {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };

                    let line = Line::from(vec![
                        Span::styled(checkbox, style),
                        Span::styled(&loc.display_name, Style::default().fg(Color::White)),
                        Span::styled(format!(" ({})", loc.path.display()), theme.muted),
                    ]);
                    ListItem::new(line)
                })
                .collect();

            let list = List::new(items).block(
                Block::default().title(" Select Clients ([Space] toggle, [Enter] preview diff) "),
            );
            f.render_widget(list, inner);
        }

        WizardStep::PreviewDiff => {
            let diff_text = if wizard.diff_preview.is_empty() {
                "No client targets selected or no changes detected.".to_string()
            } else {
                wizard.diff_preview.clone()
            };

            let mut lines: Vec<Line> = diff_text
                .lines()
                .map(|line| {
                    if line.starts_with('+') && !line.starts_with("+++") {
                        Line::styled(line, Style::default().fg(Color::Green))
                    } else if line.starts_with('-') && !line.starts_with("---") {
                        Line::styled(line, Style::default().fg(Color::Red))
                    } else if line.starts_with("---") {
                        Line::styled(
                            line,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Line::styled(line, theme.muted)
                    }
                })
                .collect();

            lines.push(Line::raw(""));
            lines.push(Line::from(vec![Span::styled(
                "Press [Enter] to apply atomic write & backups, or [Esc] to cancel.",
                theme.key_hint,
            )]));

            let p = Paragraph::new(lines).wrap(Wrap { trim: false });
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
