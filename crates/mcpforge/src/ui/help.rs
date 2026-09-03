use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::ui::theme::Theme;

pub fn render_help(f: &mut Frame) {
    let theme = Theme::default();
    let area = centered_rect(72, 85, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" 💡 MCPFORGE HELP & INTERACTIVE GUIDE ")
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(theme.border_focus);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let shortcuts = [
        ("⚡ Dashboard Navigation", ""),
        ("  j / Down", "Move selection cursor down"),
        ("  k / Up", "Move selection cursor up"),
        ("  Space", "Toggle server enabled / disabled"),
        ("  r", "Run instant diagnostic health checks on servers"),
        ("  / ", "Fuzzy search and filter configured servers"),
        ("  u", "Auto-sync all servers across detected clients"),
        (
            "  d / Del / x",
            "Remove server (interactive modal with all/selective modes)",
        ),
        ("  a", "Open Add Server Wizard"),
        ("  ?", "Toggle this interactive help screen"),
        ("  q / Esc", "Quit MCPForge"),
        ("", ""),
        ("🤖 Clients & Harnesses View", ""),
        (
            "  Tab / 1 / 2",
            "Toggle between [1] Servers and [2] Clients views",
        ),
        ("  j / k", "Navigate through detected AI client harnesses"),
        ("  u", "Sync all servers directly into selected client"),
        ("  d / Del / x", "Remove server from selected client"),
        ("  r", "Rescan OS process table for live AI clients"),
        ("", ""),
        ("🗑️ Remove Server Dialog", ""),
        (
            "  Tab / m",
            "Switch between [Remove from All] and [Selective]",
        ),
        ("  Space", "Toggle client checkbox in selective mode"),
        ("  a / c", "Select all clients (a) / clear selection (c)"),
        ("  Enter / y", "Confirm and apply removal with .bak safety"),
        ("  Esc / n", "Cancel removal and return to dashboard"),
        ("", ""),
        ("🚦 Client Lifecycle & Status Legend", ""),
        (
            "  ● ACTIVE",
            "Application process running + config file active on disk",
        ),
        (
            "  ○ READY",
            "Configured on disk (idle). Loaded on next client launch",
        ),
        (
            "  ● RUNNING",
            "Process running, but needs config file. Press [u] to setup",
        ),
        (
            "  · AVAILABLE",
            "Supported adapter. Not yet installed or configured",
        ),
        ("", ""),
        ("🚀 Add Server Wizard", ""),
        ("  j / k / Up / Down", "Navigate catalog entries or options"),
        ("  Space", "Toggle target client selection in Step 3"),
        ("  a / n", "Select all clients (a) / deselect all (n)"),
        (
            "  Enter",
            "Proceed to next step / Apply atomic write & backups",
        ),
        ("  Esc", "Return to previous step / Cancel wizard"),
        ("", ""),
        ("Press [Esc], [?], or [q] to close this help window.", ""),
    ];

    let lines: Vec<Line> = shortcuts
        .iter()
        .map(|(key, desc)| {
            if desc.is_empty() {
                if key.starts_with("Press") {
                    Line::styled(*key, theme.key_shortcut)
                } else {
                    Line::styled(*key, theme.header)
                }
            } else {
                Line::from(vec![
                    Span::styled(
                        format!("{:<20}", key),
                        Style::default().fg(Color::Rgb(139, 233, 253)),
                    ),
                    Span::styled(*desc, Style::default().fg(Color::White)),
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
