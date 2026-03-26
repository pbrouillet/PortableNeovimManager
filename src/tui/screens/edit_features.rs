use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::tui::app::App;
use crate::tui::state::{FeatureCursorItem, Screen};

pub fn draw(frame: &mut Frame, app: &mut App, name: &str, area: Rect) {
    let has_message = app.message.is_some();
    let chunks = ratatui::layout::Layout::vertical(if has_message {
        vec![
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
            Constraint::Length(3),
        ]
    } else {
        vec![
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ]
    })
    .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        format!(" Edit Features: {name} "),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Build rows from hierarchical workload/feature model
    let items = app.visible_feature_items();
    let rows: Vec<Row> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let (checkbox, label, desc) = match item {
                FeatureCursorItem::AllToggle => {
                    let all_on = app.workload_checkboxes.iter().all(|wc| wc.enabled);
                    let any_on = app.workload_checkboxes.iter().any(|wc| wc.enabled);
                    let cb = if all_on {
                        "[x]"
                    } else if any_on {
                        "[-]"
                    } else {
                        "[ ]"
                    };
                    let enabled_count = app.workload_checkboxes.iter().filter(|wc| wc.enabled).count();
                    let total = app.workload_checkboxes.len();
                    (cb.to_string(), "★ All".to_string(), format!("Toggle all workloads ({enabled_count}/{total} enabled)"))
                }
                FeatureCursorItem::GroupHeader(name) => {
                    (String::new(), format!("── {name} ──"), String::new())
                }
                FeatureCursorItem::Workload(wi) => {
                    let wc = &app.workload_checkboxes[*wi];
                    let cb = if wc.enabled {
                        let all_on = wc.features.iter().all(|f| f.enabled);
                        if all_on { "[x]" } else { "[-]" }
                    } else {
                        "[ ]"
                    };
                    let arrow = if wc.expanded { "▼" } else { "▶" };
                    let feat_count = wc.features.len();
                    let label = format!("{arrow} {}", wc.name);
                    let desc = if feat_count > 1 {
                        format!("{} ({feat_count} features)", wc.description)
                    } else {
                        wc.description.clone()
                    };
                    (cb.to_string(), label, desc)
                }
                FeatureCursorItem::Feature(wi, fi) => {
                    let fc = &app.workload_checkboxes[*wi].features[*fi];
                    let cb = if fc.enabled { "[x]" } else { "[ ]" };
                    let label = format!("  {}", fc.name);
                    (cb.to_string(), label, String::new())
                }
            };
            let row = Row::new(vec![
                Cell::from(checkbox),
                Cell::from(label),
                Cell::from(desc),
            ]);
            if i == app.feature_cursor {
                row.style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                match item {
                    FeatureCursorItem::AllToggle => {
                        let any_on = app.workload_checkboxes.iter().any(|wc| wc.enabled);
                        if any_on {
                            row.style(Style::default().fg(Color::Cyan))
                        } else {
                            row
                        }
                    }
                    FeatureCursorItem::GroupHeader(_) => {
                        row.style(
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        )
                    }
                    FeatureCursorItem::Workload(wi) if app.workload_checkboxes[*wi].enabled => {
                        row.style(Style::default().fg(Color::Green))
                    }
                    FeatureCursorItem::Feature(wi, fi) if app.workload_checkboxes[*wi].features[*fi].enabled => {
                        row.style(Style::default().fg(Color::Green))
                    }
                    _ => row,
                }
            }
        })
        .collect();

    let widths = [
        Constraint::Length(5),
        Constraint::Length(20),
        Constraint::Min(20),
    ];

    let header_row = Row::new(vec![
        Cell::from(""),
        Cell::from("Workload / Feature"),
        Cell::from("Description"),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let table = Table::new(rows, widths).header(header_row).block(
        Block::new()
            .borders(Borders::ALL)
            .title(" Workload Features "),
    );
    frame.render_widget(table, chunks[1]);

    // Status message (optional)
    if has_message {
        let msg = app.message.as_deref().unwrap_or("");
        let message_widget = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(Color::Green),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(message_widget, chunks[2]);
    }

    // Footer
    let footer_idx = if has_message { 3 } else { 2 };
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        " Space: Toggle | →/l: Expand | ←/h: Collapse | t: Tutorial | Enter: Apply | Esc: Cancel ",
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[footer_idx]);
}

pub fn handle_keys(app: &mut App, code: KeyCode, name: &str) {
    let visible_count = app.visible_feature_items().len();
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.screen = Screen::InstanceDetail {
                name: name.to_string(),
            };
            app.message = None;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if visible_count > 0 {
                app.feature_cursor = (app.feature_cursor + 1) % visible_count;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if visible_count > 0 {
                app.feature_cursor = if app.feature_cursor == 0 {
                    visible_count - 1
                } else {
                    app.feature_cursor - 1
                };
            }
        }
        KeyCode::Char(' ') => {
            app.toggle_feature();
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.toggle_expand();
        }
        KeyCode::Left | KeyCode::Char('h') => {
            // Collapse: if on a feature, jump to its parent; if on workload, collapse it
            let items = app.visible_feature_items();
            if let Some(item) = items.get(app.feature_cursor) {
                match item {
                    FeatureCursorItem::AllToggle | FeatureCursorItem::GroupHeader(_) => {
                        // No-op
                    }
                    FeatureCursorItem::Feature(wi, _) => {
                        // Jump cursor to the parent workload
                        let wi = *wi;
                        if let Some(pos) = items.iter().position(|i| matches!(i, FeatureCursorItem::Workload(w) if *w == wi)) {
                            app.feature_cursor = pos;
                        }
                    }
                    FeatureCursorItem::Workload(wi) => {
                        app.workload_checkboxes[*wi].expanded = false;
                    }
                }
            }
        }
        KeyCode::Enter => {
            app.apply_features(name);
            app.screen = Screen::InstanceDetail {
                name: name.to_string(),
            };
        }
        KeyCode::Char('t') => {
            let items = app.visible_feature_items();
            if let Some(FeatureCursorItem::Workload(wi)) = items.get(app.feature_cursor) {
                let workload_id = &app.workload_checkboxes[*wi].workload_id;
                if let Some((title, content)) = app.registry.tutorial_content(workload_id) {
                    let return_to = Screen::EditFeatures { name: name.to_string() };
                    app.open_tutorial_view(title, content, return_to);
                } else {
                    app.message = Some("No tutorial available for this workload.".to_string());
                }
            }
        }
        _ => {}
    }
}
