use std::io;

use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

use crate::config;
use crate::tui::app::{self, App};
use crate::tui::state::Screen;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let has_error = app.create_error.is_some();
    let mut constraints = vec![
        Constraint::Length(3),  // header
        Constraint::Length(5),  // form (name + preset)
        Constraint::Min(3),    // preset details
    ];
    if has_error {
        constraints.push(Constraint::Length(2)); // error message
    }
    constraints.push(Constraint::Length(3)); // footer

    let chunks = Layout::vertical(constraints).split(area);
    let mut chunk_idx = 0;

    // Header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        " Create New Instance ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(header, chunks[chunk_idx]);
    chunk_idx += 1;

    // Form fields
    let name_style = if app.create_field_cursor == 0 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let preset_style = if app.create_field_cursor == 1 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let name_display = if app.create_field_cursor == 0 {
        format!("{}▏", app.create_name)
    } else if app.create_name.is_empty() {
        "(enter name)".to_string()
    } else {
        app.create_name.clone()
    };

    let preset_name = app.registry.presets
        .get(app.create_preset_cursor)
        .map(|p| p.name.as_str())
        .unwrap_or("(none)");
    let preset_display = if app.create_field_cursor == 1 {
        format!("◀ {preset_name} ▶")
    } else {
        preset_name.to_string()
    };

    let form_lines = vec![
        Line::from(vec![
            Span::styled("  Name:   ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(name_display, name_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Preset: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(preset_display, preset_style),
        ]),
    ];
    let form = Paragraph::new(form_lines)
        .block(Block::new().borders(Borders::ALL));
    frame.render_widget(form, chunks[chunk_idx]);
    chunk_idx += 1;

    // Preset details
    let preset_info = if let Some(preset) = app.registry.presets.get(app.create_preset_cursor) {
        let workloads_str = if preset.workloads.is_empty() {
            "  (base workloads only)".to_string()
        } else {
            format!("  {}", preset.workloads.join(", "))
        };
        vec![
            Line::from(Span::styled(
                format!("  {}", preset.description),
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Workloads: ", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(Span::styled(workloads_str, Style::default().fg(Color::Green))),
        ]
    } else {
        vec![Line::from("")]
    };
    let details = Paragraph::new(preset_info)
        .block(Block::new().borders(Borders::ALL).title(" Preset Info "));
    frame.render_widget(details, chunks[chunk_idx]);
    chunk_idx += 1;

    // Error message (optional)
    if has_error {
        let err = app.create_error.as_deref().unwrap_or("");
        let error_widget = Paragraph::new(Line::from(Span::styled(
            err,
            Style::default().fg(Color::Red),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(error_widget, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // Footer
    let footer_text = if app.create_field_cursor == 0 {
        " Type instance name | Tab/↓: Preset | Enter: Create | Esc: Cancel "
    } else {
        " ←/→: Cycle preset | Tab/↑: Name | Enter: Create | Esc: Cancel "
    };
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        footer_text,
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[chunk_idx]);
}

pub async fn handle_keys(
    app: &mut App,
    code: KeyCode,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) {
    // When on the name field (field 0), capture text input
    if app.create_field_cursor == 0 {
        match code {
            KeyCode::Esc => {
                app.screen = Screen::InstanceList;
                app.message = None;
                return;
            }
            KeyCode::Tab | KeyCode::Down => {
                app.create_field_cursor = 1;
                return;
            }
            KeyCode::Enter => {
                // Fall through to the create logic below
            }
            KeyCode::Backspace => {
                app.create_name.pop();
                // Live validation
                if !app.create_name.is_empty() {
                    app.create_error = crate::cli::validate_instance_name(&app.create_name).err();
                } else {
                    app.create_error = None;
                }
                return;
            }
            KeyCode::Char(c) => {
                app.create_name.push(c);
                // Live validation
                app.create_error = crate::cli::validate_instance_name(&app.create_name).err();
                return;
            }
            _ => return,
        }
    }

    // When on the preset field (field 1)
    if app.create_field_cursor == 1 && code != KeyCode::Enter {
        match code {
            KeyCode::Esc => {
                app.screen = Screen::InstanceList;
                app.message = None;
                return;
            }
            KeyCode::Tab | KeyCode::Up | KeyCode::BackTab => {
                app.create_field_cursor = 0;
                return;
            }
            KeyCode::Left | KeyCode::Char('h') => {
                let count = app.registry.presets.len();
                if count > 0 {
                    app.create_preset_cursor = if app.create_preset_cursor == 0 {
                        count - 1
                    } else {
                        app.create_preset_cursor - 1
                    };
                }
                return;
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let count = app.registry.presets.len();
                if count > 0 {
                    app.create_preset_cursor = (app.create_preset_cursor + 1) % count;
                }
                return;
            }
            _ => return,
        }
    }

    // Enter pressed — attempt creation
    if code == KeyCode::Enter {
        if app.create_name.is_empty() {
            app.create_error = Some("Name cannot be empty.".to_string());
            app.create_field_cursor = 0;
            return;
        }
        if let Err(e) = crate::cli::validate_instance_name(&app.create_name) {
            app.create_error = Some(e);
            app.create_field_cursor = 0;
            return;
        }

        // Check if instance already exists
        let instance_dir = config::instance_dir(&app.settings, &app.create_name);
        if instance_dir.exists() {
            app.create_error = Some(format!("Instance '{}' already exists.", app.create_name));
            app.create_field_cursor = 0;
            return;
        }

        // Resolve preset to feature list
        let features = if let Some(preset) = app.registry.presets.get(app.create_preset_cursor) {
            preset.workloads.clone()
        } else {
            Vec::new()
        };

        let name = app.create_name.clone();
        app::leave_tui(terminal);
        println!("Creating instance '{}' ...", name);

        match crate::instance::create(&name, None, features, &app.registry, &app.settings).await {
            Ok(()) => {
                app.refresh_instances();
                app.message = Some(format!("✓ Created instance '{name}'."));
            }
            Err(e) => {
                eprintln!("Create failed: {e}");
                app.message = Some(format!("Create failed: {e}"));
            }
        }

        println!("\nPress Enter to return to TUI...");
        let _ = std::io::stdin().read_line(&mut String::new());
        app::enter_tui(terminal);
        app.screen = Screen::InstanceList;
    }
}
