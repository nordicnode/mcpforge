pub mod clients;
pub mod dashboard;
pub mod delete;
pub mod help;
pub mod layout;
pub mod theme;
pub mod wizard;

use crate::app::{App, CurrentView};
use ratatui::Frame;

pub fn render_ui(f: &mut Frame, app: &App) {
    match app.current_view {
        CurrentView::Dashboard => {
            dashboard::render_dashboard(f, app);
        }
        CurrentView::Clients => {
            clients::render_clients_view(f, app);
        }
        CurrentView::AddWizard => {
            dashboard::render_dashboard(f, app);
            wizard::render_wizard(f, app);
        }
        CurrentView::Help => {
            dashboard::render_dashboard(f, app);
            help::render_help(f);
        }
        CurrentView::DeleteConfirm => {
            dashboard::render_dashboard(f, app);
            delete::render_delete_modal(f, app);
        }
        CurrentView::ViewSnippet => {
            dashboard::render_dashboard(f, app);
            dashboard::render_snippet_modal(f, app);
        }
    }
}
