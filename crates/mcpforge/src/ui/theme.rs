use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::BorderType;

#[allow(dead_code)]
pub struct Theme {
    pub border: Style,
    pub border_focus: Style,
    pub border_type: BorderType,
    pub title: Style,
    pub header: Style,
    pub selected: Style,
    pub status_healthy: Style,
    pub status_degraded: Style,
    pub status_broken: Style,
    pub status_disabled: Style,
    pub key_hint: Style,
    pub key_shortcut: Style,
    pub muted: Style,
    pub accent: Style,
    pub pill_active: Style,
    pub pill_ready: Style,
    pub pill_avail: Style,
    pub banner_bg: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            border: Style::default().fg(Color::Rgb(68, 71, 90)), // Dracula comment/slate
            border_focus: Style::default()
                .fg(Color::Rgb(139, 233, 253)) // Cyan focus
                .add_modifier(Modifier::BOLD),
            border_type: BorderType::Rounded,
            title: Style::default()
                .fg(Color::Rgb(139, 233, 253)) // Cyan
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(Color::Rgb(241, 250, 140)) // Warm soft yellow
                .add_modifier(Modifier::BOLD),
            selected: Style::default()
                .bg(Color::Rgb(40, 50, 80)) // Deep Indigo-Navy
                .fg(Color::Rgb(255, 255, 255))
                .add_modifier(Modifier::BOLD),
            status_healthy: Style::default()
                .fg(Color::Rgb(80, 250, 123)) // Emerald Green
                .add_modifier(Modifier::BOLD),
            status_degraded: Style::default()
                .fg(Color::Rgb(255, 184, 108)) // Warm Orange-Amber
                .add_modifier(Modifier::BOLD),
            status_broken: Style::default()
                .fg(Color::Rgb(255, 85, 85)) // Coral Red
                .add_modifier(Modifier::BOLD),
            status_disabled: Style::default().fg(Color::Rgb(98, 114, 164)),
            key_hint: Style::default().fg(Color::Rgb(189, 147, 249)), // Soft Lavender
            key_shortcut: Style::default()
                .fg(Color::Rgb(241, 250, 140))
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(Color::Rgb(120, 125, 145)),
            accent: Style::default()
                .fg(Color::Rgb(189, 147, 249)) // Purple
                .add_modifier(Modifier::BOLD),
            pill_active: Style::default()
                .fg(Color::Rgb(80, 250, 123))
                .add_modifier(Modifier::BOLD),
            pill_ready: Style::default()
                .fg(Color::Rgb(139, 233, 253))
                .add_modifier(Modifier::BOLD),
            pill_avail: Style::default().fg(Color::Rgb(98, 114, 164)),
            banner_bg: Style::default().bg(Color::Rgb(30, 35, 55)).fg(Color::White),
        }
    }
}
