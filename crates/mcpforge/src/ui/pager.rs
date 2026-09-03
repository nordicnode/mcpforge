use crate::app::App;
use crate::ui::layout::centered_rect;
use crate::ui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_pager_modal(f: &mut Frame, app: &App) {
    let theme = Theme::default();
    let area = centered_rect(95, 92, f.area());
    f.render_widget(Clear, area);

    let state = match app.tool_explorer_state {
        Some(ref s) => s,
        None => return,
    };

    let full_text = match state.execution_result {
        Some(ref res) => res.as_str(),
        None => match state.error_message {
            Some(ref err) => err.as_str(),
            None => "No output available.",
        },
    };

    let lines: Vec<&str> = full_text.lines().collect();
    let total_lines = lines.len();

    let title = format!(
        " FULLSCREEN TOOL OUTPUT INSPECTOR: {} ({} lines) ",
        state
            .tools
            .get(state.selected_index)
            .map(|t| t.name.as_str())
            .unwrap_or("<tool>"),
        total_lines
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
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(2)])
        .margin(1)
        .split(area);

    // Calculate viewport
    let height = inner[0].height as usize;
    let scroll = app.pager_scroll.min(total_lines.saturating_sub(height));

    let mut display_lines = Vec::new();
    for (idx, line_str) in lines.iter().enumerate().skip(scroll).take(height) {
        let line_num_str = format!("{:4} │ ", idx + 1);
        display_lines.push(Line::from(vec![
            Span::styled(line_num_str, theme.muted),
            Span::styled(*line_str, Style::default().fg(Color::White)),
        ]));
    }

    f.render_widget(Paragraph::new(display_lines), inner[0]);

    // Footer with controls
    let footer_text = vec![
        Span::styled("[j/k / Up/Down] ", theme.key_shortcut),
        Span::styled("Scroll   ", Style::default().fg(Color::White)),
        Span::styled("[g/G] ", theme.key_shortcut),
        Span::styled("Top/Bottom   ", Style::default().fg(Color::White)),
        Span::styled("[c] ", theme.key_shortcut),
        Span::styled("Copy Output   ", Style::default().fg(Color::White)),
        Span::styled("[Esc / q] ", theme.key_shortcut),
        Span::styled("Close Pager", Style::default().fg(Color::White)),
    ];
    f.render_widget(Paragraph::new(Line::from(footer_text)), inner[1]);
}
