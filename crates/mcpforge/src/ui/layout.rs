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

pub fn calculate_scroll_window(total: usize, selected: usize, height: usize) -> (usize, usize) {
    if total == 0 || height == 0 {
        return (0, 0);
    }
    if total <= height {
        return (0, total);
    }
    let half = height / 2;
    let start = if selected < half {
        0
    } else if selected + (height - half) >= total {
        total.saturating_sub(height)
    } else {
        selected - half
    };
    let end = (start + height).min(total);
    (start, end)
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_scroll_window() {
        assert_eq!(calculate_scroll_window(0, 0, 10), (0, 0));
        assert_eq!(calculate_scroll_window(5, 2, 10), (0, 5));

        // 50 items, height 10
        // selected at 0
        let (s0, e0) = calculate_scroll_window(50, 0, 10);
        assert_eq!((s0, e0), (0, 10));
        assert_eq!(s0, 0);
        assert!(e0 > 0);

        // selected at 20 (middle)
        let (s20, e20) = calculate_scroll_window(50, 20, 10);
        assert!(20 >= s20 && 20 < e20);
        assert_eq!(e20 - s20, 10);

        // selected at 49 (end)
        let (s49, e49) = calculate_scroll_window(50, 49, 10);
        assert_eq!((s49, e49), (40, 50));
        assert!(49 >= s49 && 49 < e49);
    }
}
