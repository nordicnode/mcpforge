use anyhow::Result;
use mcp_core::types::HealthStatus;
use mcpforge::app::{App, CurrentView, WizardSource, WizardStep};
use mcpforge::ui;
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;
use serde::Serialize;
use std::fs::File;
use std::io::Write;

#[derive(Serialize)]
struct CellData {
    ch: String,
    fg: (u8, u8, u8),
    bg: (u8, u8, u8),
    bold: bool,
}

#[derive(Serialize)]
struct FrameData {
    width: u16,
    height: u16,
    cells: Vec<Vec<CellData>>,
}

fn color_to_rgb(c: Color, default_fg: bool) -> (u8, u8, u8) {
    match c {
        Color::Reset => {
            if default_fg {
                (248, 248, 242)
            } else {
                (24, 25, 38)
            }
        }
        Color::Black => (0, 0, 0),
        Color::Red => (255, 85, 85),
        Color::Green => (80, 250, 123),
        Color::Yellow => (241, 250, 140),
        Color::Blue => (189, 147, 249),
        Color::Magenta => (255, 121, 198),
        Color::Cyan => (139, 233, 253),
        Color::Gray => (150, 155, 175),
        Color::DarkGray => (98, 114, 164),
        Color::LightRed => (255, 110, 110),
        Color::LightGreen => (105, 255, 148),
        Color::LightYellow => (255, 255, 160),
        Color::LightBlue => (200, 160, 255),
        Color::LightMagenta => (255, 140, 210),
        Color::LightCyan => (160, 245, 255),
        Color::White => (248, 248, 242),
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Indexed(i) => match i {
            0 => (0, 0, 0),
            1 => (255, 85, 85),
            2 => (80, 250, 123),
            3 => (241, 250, 140),
            4 => (189, 147, 249),
            5 => (255, 121, 198),
            6 => (139, 233, 253),
            7 => (248, 248, 242),
            _ => (150, 155, 175),
        },
    }
}

fn dump_frame(app: &mut App, width: u16, height: u16, filename: &str) -> Result<()> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|f| {
        ui::render_ui(f, app);
    })?;

    let buffer = terminal.backend().buffer();
    let mut rows = Vec::new();

    for y in 0..height {
        let mut row = Vec::new();
        for x in 0..width {
            let cell = &buffer[(x, y)];
            row.push(CellData {
                ch: cell.symbol().to_string(),
                fg: color_to_rgb(cell.fg, true),
                bg: color_to_rgb(cell.bg, false),
                bold: cell.modifier.contains(ratatui::style::Modifier::BOLD),
            });
        }
        rows.push(row);
    }

    let frame = FrameData {
        width,
        height,
        cells: rows,
    };

    let json = serde_json::to_string_pretty(&frame)?;
    let mut f = File::create(filename)?;
    f.write_all(json.as_bytes())?;
    Ok(())
}

fn main() -> Result<()> {
    let mut app = App::new()?;
    let width = 126;
    let height = 36;

    // Pre-populate health cache with healthy statuses for a great screenshot
    for server in &app.servers {
        app.health_cache.insert(
            server.id.clone(),
            HealthStatus::Healthy {
                latency_ms: match server.id.as_str() {
                    "filesystem" => 4,
                    "github" => 28,
                    "postgres" => 12,
                    "sequential-thinking" => 6,
                    "fetch" => 18,
                    _ => 15,
                },
                tool_count: match server.id.as_str() {
                    "filesystem" => 14,
                    "github" => 26,
                    "postgres" => 9,
                    "sequential-thinking" => 1,
                    "fetch" => 1,
                    _ => 6,
                },
                server_name: server.id.clone(),
                server_version: "1.0.0".to_string(),
            },
        );
    }

    // 1. Dashboard View
    app.current_view = CurrentView::Dashboard;
    app.selected_index = 1; // github
    dump_frame(&mut app, width, height, "screenshot_dashboard.json")?;
    println!("Captured screenshot_dashboard.json");

    // 2. Clients View
    app.current_view = CurrentView::Clients;
    app.selected_client_index = 1; // Freebuff Desktop & CLI
    dump_frame(&mut app, width, height, "screenshot_clients.json")?;
    println!("Captured screenshot_clients.json");

    // 3. Wizard Catalog View (Step 2)
    app.start_wizard();
    if let Some(ref mut wizard) = app.wizard_state {
        wizard.step = WizardStep::ConfigureServer;
        wizard.source = WizardSource::FromRegistry;
        wizard.registry_category_index = 1; // Agents
        wizard.registry_cursor = 0; // langchain
    }
    dump_frame(&mut app, width, height, "screenshot_wizard_catalog.json")?;
    println!("Captured screenshot_wizard_catalog.json");

    // 4. Wizard Diff Preview (Step 4)
    if let Some(ref mut wizard) = app.wizard_state {
        for (loc, sel) in &mut wizard.target_locations {
            *sel = loc.client_id == "freebuff" || loc.client_id == "deepseek";
        }
        wizard.registry_cursor = 2; // langchain
    }
    app.compute_wizard_diff();
    if let Some(ref mut wizard) = app.wizard_state {
        wizard.step = WizardStep::PreviewDiff;
    }
    dump_frame(&mut app, width, height, "screenshot_wizard_diff.json")?;
    println!("Captured screenshot_wizard_diff.json");

    // 5. Delete Confirmation Modal
    app.start_delete();
    dump_frame(&mut app, width, height, "screenshot_delete_modal.json")?;
    println!("Captured screenshot_delete_modal.json");

    // 6. Interactive Schema Form Builder
    app.delete_state = None;
    app.current_view = CurrentView::ToolExplorer;
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "thought": { "type": "string", "description": "Your current thinking step" },
            "nextThoughtNeeded": { "type": "boolean" },
            "thoughtNumber": { "type": "integer", "minimum": 1 },
            "totalThoughts": { "type": "integer", "minimum": 1 }
        },
        "required": ["thought", "nextThoughtNeeded", "thoughtNumber", "totalThoughts"]
    });
    let form_fields = mcpforge::app::init_form_fields_from_schema(Some(&schema));
    app.tool_explorer_state = Some(mcpforge::app::ToolExplorerState {
        server_id: "sequentialthinking".to_string(),
        tools: vec![mcp_core::protocol::ToolDefinition {
            name: "sequentialthinking".to_string(),
            description: Some("Dynamic sequential thinking process for reasoning through complex tasks".to_string()),
            input_schema: Some(schema),
        }],
        selected_index: 0,
        is_loading: false,
        execution_result: None,
        error_message: None,
        params_input: "{}".to_string(),
        is_editing_params: false,
        is_form_mode: true,
        form_fields,
        form_active_index: 0,
    });
    dump_frame(&mut app, width, height, "screenshot_form_builder.json")?;
    println!("Captured screenshot_form_builder.json");

    // 7. Visual Backup Manager & Diff Inspector
    app.tool_explorer_state = None;
    app.open_backup_manager();
    dump_frame(&mut app, width, height, "screenshot_backup_modal.json")?;
    println!("Captured screenshot_backup_modal.json");

    Ok(())
}
