use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub header: Rect,
    pub main: Rect,
    pub footer: Rect,
}

pub fn create_app_layout(area: Rect) -> AppLayout {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Main area
            Constraint::Length(1), // Footer key hints
        ])
        .split(area);

    AppLayout {
        header: chunks[0],
        main: chunks[1],
        footer: chunks[2],
    }
}

pub fn create_split_main_layout(area: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35), // Left: Server list
            Constraint::Percentage(65), // Right: Server inspector / details
        ])
        .split(area);

    (chunks[0], chunks[1])
}
