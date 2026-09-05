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
        .title(" MCPFORGE HELP & INTERACTIVE GUIDE ")
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(theme.border_focus);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let shortcuts = [
        ("Dashboard Navigation & Two-Pane Focus", ""),
        (
            "  j / k / Up / Down",
            "Move selection in active pane (or scroll inspector)",
        ),
        ("  Enter / l / Right", "Focus Server Inspector pane"),
        ("  h / Left / Esc", "Return focus to Server List"),
        (
            "  1 - 5 / Tab",
            "Switch Inspector sub-tab (Overview/Clients/Env/Telemetry/JSON)",
        ),
        (
            "  c",
            "Copy server configuration JSON to clipboard (in Inspector)",
        ),
        ("  Space", "Toggle server enabled / disabled"),
        ("  t", "Open Tool Explorer & Interactive Playground"),
        ("  T", "Execute live background JSON-RPC 2.0 handshake test"),
        ("  b", "Open Configuration Snapshots & Diff Inspector"),
        ("  r", "Run instant diagnostic health checks on servers"),
        ("  / ", "Fuzzy search and filter configured servers"),
        ("  u", "Auto-sync all servers across detected clients"),
        (
            "  d / Del / x",
            "Remove server (interactive modal with all/selective modes)",
        ),
        ("  a", "Open Add Server Wizard"),
        ("  ?", "Toggle this interactive help screen"),
        ("  q", "Quit MCPForge"),
        ("", ""),
        ("Tool Explorer & Playground", ""),
        ("  j / k", "Navigate tools exposed by selected server"),
        ("  f", "Toggle Interactive Schema-Guided Form Builder"),
        ("  Tab / Shift+Tab", "Next / Previous field in Form Builder"),
        ("  Space", "Toggle boolean values in Form Builder"),
        ("  e", "Edit raw JSON parameters directly"),
        ("  r", "Reset parameters to synthesized schema defaults"),
        ("  Enter", "Execute live tool call over stdio / HTTP"),
        ("  v", "Open Fullscreen Output Pager & Clipboard Exporter"),
        (
            "  c",
            "Copy full tool output to system clipboard (in Pager)",
        ),
        ("", ""),
        ("Clients & Harnesses View", ""),
        (
            "  Tab / 1 / 2",
            "Toggle between [1] Servers and [2] Clients views",
        ),
        ("  j / k", "Navigate through detected AI client harnesses"),
        (
            "  v",
            "Inspect raw active configuration file in modal viewer",
        ),
        ("  m", "Run live cross-compatibility matrix verification"),
        ("  u", "Sync all servers directly into selected client"),
        ("  d / Del / x", "Remove server from selected client"),
        ("  r", "Rescan OS process table for live AI clients"),
        ("", ""),
        ("Remove Server Dialog", ""),
        (
            "  Tab / m",
            "Switch between [Remove from All] and [Selective]",
        ),
        ("  Space", "Toggle client checkbox in selective mode"),
        ("  a / c", "Select all clients (a) / clear selection (c)"),
        ("  Enter / y", "Confirm and apply removal with .bak safety"),
        ("  Esc / n", "Cancel removal and return to dashboard"),
        ("", ""),
        ("Client Lifecycle & Status Legend", ""),
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
        ("Add Server Wizard", ""),
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
