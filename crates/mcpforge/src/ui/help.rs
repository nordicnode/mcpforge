use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::ui::theme::Theme;

pub fn render_help(f: &mut Frame) {
    let theme = Theme::default();
    let area = centered_rect(65, 70, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" HELP & KEYBINDINGS ")
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let shortcuts = [
        ("Dashboard Navigation", ""),
        ("  j / Down", "Move cursor down"),
        ("  k / Up", "Move cursor up"),
        ("  r", "Run health checks on current servers"),
        ("  / ", "Search and filter servers"),
        ("  Space", "Toggle server enabled / disabled"),
        ("  d", "Delete selected server from clients"),
        ("  a", "Open Add Server Wizard"),
        ("  ?", "Toggle this help screen"),
        ("  q / Esc", "Quit MCPForge"),
        ("", ""),
        ("Add Wizard", ""),
        ("  Up / Down", "Navigate options and catalog entries"),
        ("  Space", "Toggle target client selection"),
        ("  Enter", "Proceed to next step / Apply changes"),
        ("  Esc", "Cancel wizard and return to dashboard"),
    ];

    let lines: Vec<Line> = shortcuts
        .iter()
        .map(|(key, desc)| {
            if desc.is_empty() {
                Line::styled(
                    *key,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Line::from(vec![
                    Span::styled(format!("{:<14}", key), Style::default().fg(Color::Cyan)),
                    Span::raw(*desc),
                ])
            }
        })
        .collect();

    let p = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(p, inner);
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
