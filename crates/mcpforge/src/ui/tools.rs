use crate::app::App;
use crate::ui::layout::centered_rect;
use crate::ui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn render_tools_modal(f: &mut Frame, app: &App) {
    let theme = Theme::default();
    let area = centered_rect(90, 88, f.area());
    f.render_widget(Clear, area);

    let state = match app.tool_explorer_state {
        Some(ref s) => s,
        None => return,
    };

    let title = format!(" TOOL EXPLORER & PLAYGROUND: {} ", state.server_id);
    let outer_block = Block::default()
        .title(Span::styled(title, theme.title))
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(theme.border_focus);
    f.render_widget(outer_block, area);

    let inner = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
        .margin(1)
        .split(area);

    // Left pane: Tool List
    let items: Vec<ListItem> = if state.is_loading {
        vec![ListItem::new(Line::from(vec![Span::styled(
            "  Connecting & querying tools from server...",
            Style::default().fg(Color::Yellow),
        )]))]
    } else if state.tools.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            "  No tools exposed by this server.",
            theme.muted,
        )]))]
    } else {
        state
            .tools
            .iter()
            .enumerate()
            .map(|(i, tool)| {
                let is_sel = i == state.selected_index;
                let prefix = if is_sel { "▶ " } else { "  " };
                let style = if is_sel {
                    theme.selected
                } else {
                    Style::default().fg(Color::White)
                };

                let line = Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(tool.name.clone(), style),
                ]);
                ListItem::new(line)
            })
            .collect()
    };

    let list_block = Block::default()
        .title(Span::styled(
            format!(" Exposed Tools ({}) ", state.tools.len()),
            theme.header,
        ))
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(theme.border);
    f.render_widget(List::new(items).block(list_block), inner[0]);

    // Right pane splits: Tool Spec, Parameters/Form, Live Output
    let right_splits = if state.is_form_mode {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40), // Tool spec & schema
                Constraint::Length(6),      // Interactive Form Bar
                Constraint::Min(5),         // Output
            ])
            .split(inner[1])
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(45), // Tool spec & schema
                Constraint::Length(3),      // Raw JSON input bar
                Constraint::Min(5),         // Output
            ])
            .split(inner[1])
    };

    // 1. Tool Specification & Schema
    let details_content = if let Some(tool) = state.tools.get(state.selected_index) {
        let desc = tool
            .description
            .as_deref()
            .unwrap_or("No description provided.");
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Tool Name:   ", theme.key_shortcut),
                Span::styled(
                    &tool.name,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Description: ", theme.key_shortcut),
                Span::styled(desc, Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(Span::styled("INPUT PARAMETERS SCHEMA:", theme.header)),
        ];

        if let Some(ref schema) = tool.input_schema {
            if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                let required = schema
                    .get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<std::collections::HashSet<_>>()
                    })
                    .unwrap_or_default();

                for (prop_name, prop_val) in props {
                    let prop_type = prop_val
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("any");
                    let prop_desc = prop_val
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("");
                    let req_str = if required.contains(prop_name.as_str()) {
                        " (required)"
                    } else {
                        " (optional)"
                    };

                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("  • {} ", prop_name),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("[{}]{}", prop_type, req_str),
                            Style::default().fg(Color::Magenta),
                        ),
                    ]));
                    if !prop_desc.is_empty() {
                        lines.push(Line::from(vec![Span::styled(
                            format!("    {}", prop_desc),
                            theme.muted,
                        )]));
                    }
                }
            } else {
                lines.push(Line::from(Span::styled(
                    "  No input parameters required ({})",
                    theme.status_healthy,
                )));
            }
        } else {
            lines.push(Line::from(Span::styled(
                "  No schema defined.",
                theme.muted,
            )));
        }

        lines
    } else {
        vec![Line::from("Select a tool from the list.")]
    };

    let details_block = Block::default()
        .title(Span::styled(" Tool Specification ", theme.header))
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(theme.border);
    f.render_widget(
        Paragraph::new(details_content)
            .block(details_block)
            .wrap(Wrap { trim: true }),
        right_splits[0],
    );

    // 2. Interactive Parameters: Form Mode or Raw JSON Mode
    if state.is_form_mode {
        let form_lines = if let Some(field) = state.form_fields.get(state.form_active_index) {
            let req_badge = if field.is_required { " *" } else { "" };
            let toggle_hint = if field.field_type == "boolean" {
                "  (Press [Space] to toggle)"
            } else {
                ""
            };

            vec![
                Line::from(vec![
                    Span::styled(
                        format!(
                            "[Field {}/{}] ",
                            state.form_active_index + 1,
                            state.form_fields.len()
                        ),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{}{}", field.name, req_badge),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" [{}]", field.field_type),
                        Style::default().fg(Color::Magenta),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  Value: ", theme.key_shortcut),
                    Span::styled(
                        format!("{}█", field.value),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(toggle_hint, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::styled("  Desc:  ", theme.muted),
                    Span::styled(
                        if field.description.is_empty() {
                            "No description"
                        } else {
                            &field.description
                        },
                        theme.muted,
                    ),
                ]),
            ]
        } else {
            vec![Line::from(
                "This tool takes no parameters. Press [Enter] to run.",
            )]
        };

        let form_title =
            " INTERACTIVE SCHEMA FORM · [Tab/Shift+Tab] Next/Prev · [Enter] Run · [f] JSON Mode ";
        let form_block = Block::default()
            .title(Span::styled(
                form_title,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(theme.border_type)
            .border_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_widget(
            Paragraph::new(form_lines).block(form_block),
            right_splits[1],
        );
    } else {
        let (param_title, param_border_style, param_text) = if state.is_editing_params {
            (
                " [EDITING PARAMETERS: Type JSON · Enter to Run · Esc to Cancel] ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                format!("{}█", state.params_input),
            )
        } else {
            (
                " Parameters (JSON) · [f] Form Builder · [e] Edit · [r] Reset Schema Defaults ",
                theme.border,
                state.params_input.clone(),
            )
        };

        let param_block = Block::default()
            .title(Span::styled(
                param_title,
                if state.is_editing_params {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    theme.key_shortcut
                },
            ))
            .borders(Borders::ALL)
            .border_type(theme.border_type)
            .border_style(param_border_style);
        f.render_widget(
            Paragraph::new(param_text)
                .block(param_block)
                .style(Style::default().fg(Color::White)),
            right_splits[1],
        );
    }

    // 3. Lower right: Live Execution Output Panel
    let mut exec_lines = Vec::new();
    let mut output_ready = false;

    if let Some(ref res) = state.execution_result {
        output_ready = true;
        exec_lines.push(Line::from(Span::styled(
            "Execution Output (Press [v] for Fullscreen Inspector):",
            theme.status_healthy,
        )));
        for l in res.lines().take(12) {
            exec_lines.push(Line::from(Span::styled(
                l,
                Style::default().fg(Color::White),
            )));
        }
        if res.lines().count() > 12 {
            exec_lines.push(Line::from(Span::styled(
                format!(
                    "... (+{} more lines. Press [v] to view full output)",
                    res.lines().count() - 12
                ),
                Style::default().fg(Color::Yellow),
            )));
        }
    } else if let Some(ref err) = state.error_message {
        output_ready = true;
        exec_lines.push(Line::from(Span::styled(
            format!("Error: {}", err),
            theme.status_broken,
        )));
    } else {
        exec_lines.push(Line::from(vec![
            Span::styled("Playground Ready: ", theme.key_shortcut),
            Span::styled(
                "Press [Enter] to invoke this tool, or [f] for step-by-step form builder.",
                Style::default().fg(Color::Yellow),
            ),
        ]));
        exec_lines.push(Line::from(""));
        exec_lines.push(Line::from(vec![
            Span::styled("CLI Command:      ", theme.key_shortcut),
            Span::styled(
                format!(
                    "mcpforge call {} {} '{}'",
                    state.server_id,
                    state
                        .tools
                        .get(state.selected_index)
                        .map(|t| t.name.as_str())
                        .unwrap_or("<tool>"),
                    state.params_input
                ),
                theme.status_healthy,
            ),
        ]));
    }

    let exec_title = if output_ready {
        " Live Playground Output · [v] Fullscreen Pager · [Enter] Test Call "
    } else {
        " Live Playground Output · [Enter] Test Call "
    };

    let exec_block = Block::default()
        .title(Span::styled(exec_title, theme.header))
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(theme.border);
    f.render_widget(
        Paragraph::new(exec_lines)
            .block(exec_block)
            .wrap(Wrap { trim: false }),
        right_splits[2],
    );
}
