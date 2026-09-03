pub mod dashboard;
pub mod help;
pub mod layout;
pub mod theme;
pub mod wizard;

use crate::app::{App, CurrentView};
use ratatui::Frame;

pub fn render_ui(f: &mut Frame, app: &App) {
    dashboard::render_dashboard(f, app);

    match app.current_view {
        CurrentView::Dashboard => {}
        CurrentView::AddWizard => {
            wizard::render_wizard(f, app);
        }
        CurrentView::Help => {
            help::render_help(f);
        }
    }
}
