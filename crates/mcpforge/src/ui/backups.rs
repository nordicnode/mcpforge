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

pub fn render_backup_modal(f: &mut Frame, app: &App) {
    let theme = Theme::default();
    let area = centered_rect(90, 88, f.area());
    f.render_widget(Clear, area);

    let state = match app.backup_state {
        Some(ref s) => s,
        None => return,
    };

    let title = format!(
        " CONFIGURATION BACKUP SNAPSHOTS & DIFF INSPECTOR ({} snapshots) ",
        state.backups.len()
    );

    let outer_block = Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(theme.border_focus);
    f.render_widget(outer_block, area);

    let inner = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .margin(1)
        .split(area);

    // Left pane: Backup snapshots list
    let items: Vec<ListItem> = if state.backups.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            "  No backup snapshots found in ~/.config/mcpforge/backups/",
            theme.muted,
        )]))]
    } else {
        state
            .backups
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let is_sel = i == state.selected_index;
                let prefix = if is_sel { "▶ " } else { "  " };
                let style = if is_sel {
                    theme.selected
                } else {
                    Style::default().fg(Color::White)
                };

                let time_str = &b.timestamp;
                let line = Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(
                        format!("[{}] ", b.client_id),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(time_str.clone(), style),
                ]);
                ListItem::new(line)
            })
            .collect()
    };

    let list_block = Block::default()
        .title(Span::styled(" Snapshots ", theme.header))
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(theme.border);
    f.render_widget(List::new(items).block(list_block), inner[0]);

    // Right pane: Unified Diff Viewer
    let mut diff_lines = Vec::new();
    for line in state.diff_preview.lines() {
        let style = if line.starts_with('+') && !line.starts_with("+++") {
            Style::default().fg(Color::Green)
        } else if line.starts_with('-') && !line.starts_with("---") {
            Style::default().fg(Color::Red)
        } else if line.starts_with('@') {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if line.starts_with("---") || line.starts_with("+++") {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        diff_lines.push(Line::from(Span::styled(line, style)));
    }

    if diff_lines.is_empty() {
        diff_lines.push(Line::from(Span::styled(
            "Snapshot is identical to the current active configuration.",
            theme.status_healthy,
        )));
    }

    let diff_block = Block::default()
        .title(Span::styled(
            " Unified Diff (Snapshot vs Current Disk) · [r] Restore Snapshot ",
            theme.header,
        ))
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(theme.border);
    f.render_widget(
        Paragraph::new(diff_lines)
            .block(diff_block)
            .wrap(Wrap { trim: false }),
        inner[1],
    );
}
