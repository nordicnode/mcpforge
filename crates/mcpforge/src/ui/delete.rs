use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme::Theme;

pub fn render_delete_modal(f: &mut Frame, app: &App) {
    let state = match &app.delete_state {
        Some(s) => s,
        None => return,
    };

    let theme = Theme::default();
    let area = centered_rect(72, 70, f.area());
    f.render_widget(Clear, area);

    let title = format!(" 🗑️  REMOVE MCP SERVER: {} ", state.server_id);

    let block = Block::default()
        .title(title)
        .title_style(theme.status_broken)
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(theme.status_broken);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Mode toggle & warning header
            Constraint::Min(6),    // Targets list
            Constraint::Length(4), // Safety notice & key action footer
        ])
        .split(inner);

    // 1. Header & Mode Switcher
    let mode_text = if state.remove_all_mode {
        Line::from(vec![
            Span::styled("Removal Mode: ", theme.header),
            Span::styled("[ REMOVE FROM ALL CLIENTS ]", theme.status_broken),
            Span::styled(
                "  (Press [Tab] or [m] for Selective Removal)",
                theme.key_hint,
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("Removal Mode: ", theme.header),
            Span::styled("[ SELECTIVE CLIENT REMOVAL ]", theme.status_degraded),
            Span::styled("  (Press [Tab] or [m] to Remove from All)", theme.key_hint),
        ])
    };

    let warning_text = Line::from(vec![
        Span::styled("⚠️  Warning: ", theme.status_degraded),
        Span::styled(
            format!("You are about to remove MCP server '{}'.", state.server_id),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let header_p = Paragraph::new(vec![warning_text, Line::raw(""), mode_text]);
    f.render_widget(header_p, chunks[0]);

    // 2. Targets Checklist or Summary
    let total = state.target_locations.len();
    let visible_height = (chunks[1].height as usize).saturating_sub(2).max(1);
    let (start, end) =
        crate::ui::layout::calculate_scroll_window(total, state.target_cursor, visible_height);

    let selected_count = state.target_locations.iter().filter(|(_, s)| *s).count();

    let items: Vec<ListItem> = state.target_locations[start..end]
        .iter()
        .enumerate()
        .map(|(offset, (loc, selected))| {
            let real_idx = start + offset;
            let is_cursor = real_idx == state.target_cursor;

            let (checkbox, check_style) = if state.remove_all_mode {
                ("✖ ", theme.status_broken)
            } else if *selected {
                ("[✓] ", theme.status_broken)
            } else {
                ("[ ] ", theme.muted)
            };

            let cursor_str = if is_cursor && !state.remove_all_mode {
                "▶ "
            } else {
                "  "
            };

            let line = Line::from(vec![
                Span::styled(
                    cursor_str,
                    if is_cursor { theme.title } else { theme.muted },
                ),
                Span::styled(checkbox, check_style),
                Span::styled(
                    &loc.display_name,
                    if is_cursor && !state.remove_all_mode {
                        theme.selected
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
                Span::styled(format!(" ({})", loc.path.display()), theme.muted),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list_title = if state.remove_all_mode {
        format!(" Target Configurations to be Removed ({}) ", total)
    } else {
        format!(
            " Select Target Clients (Selected: {}/{} | Cursor: {}/{}) [Space: toggle, a: all, n: none] ",
            selected_count,
            total,
            if total > 0 { state.target_cursor + 1 } else { 0 },
            total
        )
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(theme.border_type)
            .border_style(theme.border)
            .title(list_title),
    );
    f.render_widget(list, chunks[1]);

    // 3. Safety Notice & Actions
    let safety_line = Line::from(vec![
        Span::styled("🛡️  Atomic Safety: ", theme.status_healthy),
        Span::styled(
            "Sidecar backups (.bak) will be created automatically before saving.",
            theme.muted,
        ),
    ]);

    let action_line = Line::from(vec![
        Span::styled("ACTIONS: ", theme.header),
        Span::styled("[Enter / y] Confirm Removal", theme.status_broken),
        Span::raw("   "),
        Span::styled("[Esc / n] Cancel & Keep Server", theme.key_shortcut),
    ]);

    let footer_p =
        Paragraph::new(vec![safety_line, Line::raw(""), action_line]).wrap(Wrap { trim: false });
    f.render_widget(footer_p, chunks[2]);
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
