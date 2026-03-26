use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::config::{self, LEADER_KEY_OPTIONS};
use crate::tui::app::App;
use crate::tui::state::Screen;

const SETTINGS_FIELD_COUNT: usize = 4;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let has_message = app.message.is_some();
    let mut constraints = vec![
        Constraint::Length(3),  // header
        Constraint::Min(5),    // settings list
    ];
    if has_message {
        constraints.push(Constraint::Length(2)); // status
    }
    constraints.push(Constraint::Length(3)); // footer

    let chunks = Layout::vertical(constraints).split(area);
    let mut chunk_idx = 0;

    // Header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        " Settings ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(header, chunks[chunk_idx]);
    chunk_idx += 1;

    // Settings rows
    let leader_display = config::leader_key_display(&app.settings.default_leader_key);
    let confirm_display = if app.settings.confirm_destructive { "Yes" } else { "No" };

    let fields: Vec<(&str, String)> = vec![
        ("Instances Directory", app.settings.instances_dir.to_string_lossy().to_string()),
        ("Default Leader Key", leader_display.to_string()),
        ("Confirm Destructive", confirm_display.to_string()),
        ("Default JS Runtime", crate::runtime::runtime_display_name(app.settings.default_js_runtime.as_deref()).to_string()),
    ];

    let rows: Vec<Row> = fields
        .iter()
        .enumerate()
        .map(|(i, (label, value))| {
            let display_value = if app.settings_editing && i == app.settings_cursor {
                format!("{}▏", app.settings_edit_buffer)
            } else {
                value.clone()
            };
            let row = Row::new(vec![
                Cell::from(*label),
                Cell::from(display_value),
            ]);
            if i == app.settings_cursor {
                row.style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                row
            }
        })
        .collect();

    let widths = [Constraint::Length(25), Constraint::Min(30)];
    let header_row = Row::new(vec!["Setting", "Value"]).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    let table = Table::new(rows, widths)
        .header(header_row)
        .block(Block::new().borders(Borders::ALL).title(" Global Settings "));
    frame.render_widget(table, chunks[chunk_idx]);
    chunk_idx += 1;

    // Status message
    if has_message {
        let msg = app.message.as_deref().unwrap_or("");
        let message_widget = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(Color::Green),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(message_widget, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // Footer
    let footer_text = if app.settings_editing {
        " Type new value | Enter: Save | Esc: Cancel "
    } else {
        " j/k: Navigate | Enter: Edit/Toggle | Space: Toggle (bool) | Esc: Back "
    };
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        footer_text,
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[chunk_idx]);
}

pub fn handle_keys(app: &mut App, code: KeyCode) {
    if app.settings_editing {
        match code {
            KeyCode::Esc => {
                app.settings_editing = false;
                app.settings_edit_buffer.clear();
            }
            KeyCode::Enter => {
                // Apply the edit
                match app.settings_cursor {
                    0 => {
                        // instances_dir
                        let new_path = std::path::PathBuf::from(&app.settings_edit_buffer);
                        if app.settings_edit_buffer.is_empty() {
                            app.message = Some("Path cannot be empty.".to_string());
                        } else {
                            app.settings.instances_dir = new_path;
                            app.message = Some("Instances directory updated.".to_string());
                        }
                    }
                    1 => {
                        // default_leader_key — validate against allowed keys
                        let key = &app.settings_edit_buffer;
                        if LEADER_KEY_OPTIONS.iter().any(|(v, _)| *v == key.as_str()) {
                            app.settings.default_leader_key = key.clone();
                            app.message = Some(format!(
                                "Default leader key set to '{}'.",
                                config::leader_key_display(key)
                            ));
                        } else {
                            app.message = Some("Invalid leader key. Use Space, comma, backslash, or semicolon.".to_string());
                        }
                    }
                    2 => {
                        // confirm_destructive — toggle handled separately
                    }
                    3 => {
                        // default_js_runtime
                        let val = app.settings_edit_buffer.trim().to_string();
                        if val.is_empty() {
                            app.settings.default_js_runtime = None;
                            app.message = Some("Default JS runtime cleared (using system Node).".to_string());
                        } else if let Err(e) = crate::runtime::find_runtime_binary(&val) {
                            app.message = Some(format!("Invalid runtime: {e}"));
                        } else {
                            app.settings.default_js_runtime = Some(val.clone());
                            app.message = Some(format!("Default JS runtime set to '{val}'."));
                        }
                    }
                    _ => {}
                }
                app.settings_editing = false;
                app.settings_edit_buffer.clear();

                // Save settings
                if let Err(e) = config::save_global_settings(&app.settings) {
                    app.message = Some(format!("Failed to save settings: {e}"));
                }
            }
            KeyCode::Backspace => {
                app.settings_edit_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.settings_edit_buffer.push(c);
            }
            _ => {}
        }
        return;
    }

    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.screen = Screen::InstanceList;
            app.message = None;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.settings_cursor = (app.settings_cursor + 1) % SETTINGS_FIELD_COUNT;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.settings_cursor = if app.settings_cursor == 0 {
                SETTINGS_FIELD_COUNT - 1
            } else {
                app.settings_cursor - 1
            };
        }
        KeyCode::Enter => {
            match app.settings_cursor {
                0 => {
                    // Edit instances_dir
                    app.settings_editing = true;
                    app.settings_edit_buffer = app.settings.instances_dir.to_string_lossy().to_string();
                }
                1 => {
                    // Cycle through leader key options
                    let current = &app.settings.default_leader_key;
                    let idx = LEADER_KEY_OPTIONS
                        .iter()
                        .position(|(v, _)| *v == current.as_str())
                        .unwrap_or(0);
                    let next = (idx + 1) % LEADER_KEY_OPTIONS.len();
                    app.settings.default_leader_key = LEADER_KEY_OPTIONS[next].0.to_string();
                    app.message = Some(format!(
                        "Default leader key: {}",
                        config::leader_key_display(LEADER_KEY_OPTIONS[next].0)
                    ));
                    if let Err(e) = config::save_global_settings(&app.settings) {
                        app.message = Some(format!("Failed to save: {e}"));
                    }
                }
                2 => {
                    // Toggle confirm_destructive
                    app.settings.confirm_destructive = !app.settings.confirm_destructive;
                    let state = if app.settings.confirm_destructive { "on" } else { "off" };
                    app.message = Some(format!("Confirm destructive actions: {state}"));
                    if let Err(e) = config::save_global_settings(&app.settings) {
                        app.message = Some(format!("Failed to save: {e}"));
                    }
                }
                3 => {
                    // Cycle default_js_runtime: None → "bun" → None
                    if app.settings.default_js_runtime.is_some() {
                        app.settings.default_js_runtime = None;
                        app.message = Some("Default JS runtime: Node (system)".to_string());
                    } else {
                        // Try to enable bun
                        if crate::runtime::find_runtime_binary("bun").is_ok() {
                            app.settings.default_js_runtime = Some("bun".to_string());
                            app.message = Some("Default JS runtime: Bun".to_string());
                        } else {
                            app.message = Some("Bun not found on PATH. Install Bun first.".to_string());
                        }
                    }
                    if let Err(e) = config::save_global_settings(&app.settings) {
                        app.message = Some(format!("Failed to save: {e}"));
                    }
                }
                _ => {}
            }
        }
        KeyCode::Char(' ') if app.settings_cursor == 2 => {
            // Quick toggle for boolean field
            app.settings.confirm_destructive = !app.settings.confirm_destructive;
            let state = if app.settings.confirm_destructive { "on" } else { "off" };
            app.message = Some(format!("Confirm destructive actions: {state}"));
            if let Err(e) = config::save_global_settings(&app.settings) {
                app.message = Some(format!("Failed to save: {e}"));
            }
        }
        _ => {}
    }
}
