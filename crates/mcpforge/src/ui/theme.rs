use ratatui::style::{Color, Modifier, Style};

#[allow(dead_code)]
pub struct Theme {
    pub border: Style,
    pub title: Style,
    pub header: Style,
    pub selected: Style,
    pub status_healthy: Style,
    pub status_degraded: Style,
    pub status_broken: Style,
    pub status_disabled: Style,
    pub key_hint: Style,
    pub muted: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            border: Style::default().fg(Color::DarkGray),
            title: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            selected: Style::default()
                .bg(Color::Rgb(30, 45, 75))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            status_healthy: Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            status_degraded: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            status_broken: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            status_disabled: Style::default().fg(Color::DarkGray),
            key_hint: Style::default().fg(Color::Cyan),
            muted: Style::default().fg(Color::DarkGray),
        }
    }
}
