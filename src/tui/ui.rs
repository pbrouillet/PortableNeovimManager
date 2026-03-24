use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};

use crate::config::{self, LEADER_KEY_OPTIONS};

use super::app::{App, FeatureCursorItem, Screen};

pub fn draw(frame: &mut Frame, app: &App) {
    match &app.screen {
        Screen::InstanceList => draw_instance_list(frame, app),
        Screen::InstanceDetail { name } => draw_instance_detail(frame, app, name),
        Screen::EditFeatures { name } => draw_edit_features(frame, app, name),
        Screen::EditLeaderKey { name } => draw_edit_leader(frame, app, name),
        Screen::ConfirmDelete { name } => draw_confirm_delete(frame, name),
        Screen::EditSettings => draw_edit_settings(frame, app),
        Screen::TutorialList => draw_tutorial_list(frame, app),
        Screen::TutorialView { title, content, .. } => draw_tutorial_view(frame, app, title, content),
        Screen::Marketplace { instance_name } => draw_marketplace(frame, app, instance_name),
    }
}

fn draw_instance_list(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let has_message = app.message.is_some();
    let has_search = app.instance_search_active || !app.instance_search.is_empty();
    let mut constraints = vec![Constraint::Length(3)]; // header
    if has_search {
        constraints.push(Constraint::Length(3)); // search bar
    }
    constraints.push(Constraint::Min(5)); // table
    if has_message {
        constraints.push(Constraint::Length(2)); // status message
    }
    constraints.push(Constraint::Length(3)); // footer

    let chunks = Layout::vertical(constraints).split(area);

    let mut chunk_idx = 0;

    // Header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        " Portable Neovim Manager ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(header, chunks[chunk_idx]);
    chunk_idx += 1;

    // Search bar
    if has_search {
        let search_text = if app.instance_search_active {
            format!("/ {}_", app.instance_search)
        } else {
            format!("/ {}", app.instance_search)
        };
        let match_info = format!(
            " ({}/{})",
            app.instance_filtered.len(),
            app.instances.len()
        );
        let search_bar = Paragraph::new(Line::from(vec![
            Span::styled(search_text, Style::default().fg(Color::Yellow)),
            Span::styled(match_info, Style::default().fg(Color::DarkGray)),
        ]))
        .block(
            Block::new()
                .borders(Borders::ALL)
                .border_style(if app.instance_search_active {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }),
        );
        frame.render_widget(search_bar, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // Instance table (using filtered indices)
    let header_row = Row::new(vec![
        Cell::from("Name"),
        Cell::from("Version"),
        Cell::from("Features"),
        Cell::from("Updated"),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .instance_filtered
        .iter()
        .enumerate()
        .map(|(display_i, &inst_idx)| {
            let inst = &app.instances[inst_idx];
            let features_str = inst
                .workloads
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let updated = inst.updated_at.format("%Y-%m-%d %H:%M").to_string();
            let row = Row::new(vec![
                Cell::from(inst.name.clone()),
                Cell::from(inst.nvim_version.clone()),
                Cell::from(features_str),
                Cell::from(updated),
            ]);
            if display_i == app.selected {
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

    let widths = [
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(35),
        Constraint::Percentage(25),
    ];

    let table = Table::new(rows, widths)
        .header(header_row)
        .block(Block::new().borders(Borders::ALL).title(" Instances "));
    frame.render_widget(table, chunks[chunk_idx]);
    chunk_idx += 1;

    // Status message (optional)
    if has_message {
        let msg = app.message.as_deref().unwrap_or("");
        let message_widget = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(Color::Green),
        )))
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(message_widget, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // Footer
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        " q: Quit | Enter: Details | /: Search | t: Tutorials | p: Packages | f: Features | m: Leader | o: Open Dir | n: Nerd Font | s: Settings | l: Launch | u: Update | d: Delete ",
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[chunk_idx]);
}

fn draw_instance_detail(frame: &mut Frame, app: &App, name: &str) {
    let area = frame.area();

    let chunks = Layout::vertical(vec![
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(3),
    ])
    .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        format!(" Instance: {name} "),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Detail body
    let instance = app.instances.iter().find(|i| i.name == name);
    let body = if let Some(inst) = instance {
        let features_str = inst
            .workloads
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let leader_display = config::leader_key_display(&inst.leader_key);
        let created_str = inst.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let updated_str = inst.updated_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let lines = vec![
            Line::from(vec![
                Span::styled("Name:     ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(inst.name.clone()),
            ]),
            Line::from(vec![
                Span::styled("Version:  ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(inst.nvim_version.clone()),
            ]),
            Line::from(vec![
                Span::styled("Features: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(features_str),
            ]),
            Line::from(vec![
                Span::styled("Leader:   ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    leader_display.to_string(),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(vec![
                Span::styled("Created:  ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(created_str),
            ]),
            Line::from(vec![
                Span::styled("Updated:  ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(updated_str),
            ]),
        ];
        Paragraph::new(lines)
    } else {
        Paragraph::new(Line::from(Span::styled(
            "Instance not found.",
            Style::default().fg(Color::Red),
        )))
    };

    let body = body
        .block(Block::new().borders(Borders::ALL).title(" Details "))
        .wrap(Wrap { trim: false });
    frame.render_widget(body, chunks[1]);

    // Footer
    let footer_text = if app.message.is_some() {
        format!(
            " {} | Esc: Back | f: Features | p: Packages | m: Leader Key | t: Tutorials | o: Open Dir | l: Launch | u: Update | d: Delete ",
            app.message.as_deref().unwrap_or("")
        )
    } else {
        " Esc: Back | f: Features | p: Packages | m: Leader Key | t: Tutorials | o: Open Dir | l: Launch | u: Update | d: Delete ".to_string()
    };
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        footer_text,
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}

fn draw_edit_features(frame: &mut Frame, app: &App, name: &str) {
    let area = frame.area();

    let has_message = app.message.is_some();
    let chunks = Layout::vertical(if has_message {
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
    .alignment(ratatui::layout::Alignment::Center)
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
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(message_widget, chunks[2]);
    }

    // Footer
    let footer_idx = if has_message { 3 } else { 2 };
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        " Space: Toggle | →/l: Expand | ←/h: Collapse | t: Tutorial | Enter: Apply | Esc: Cancel ",
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[footer_idx]);
}

fn draw_edit_leader(frame: &mut Frame, app: &App, name: &str) {
    let area = frame.area();

    let chunks = Layout::vertical(vec![
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(3),
    ])
    .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        format!(" Leader Key: {name} "),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Current leader key from instance
    let instance = app.instances.iter().find(|i| i.name == name);
    let current_key = instance.map(|i| i.leader_key.as_str()).unwrap_or(" ");

    // Option list
    let rows: Vec<Row> = LEADER_KEY_OPTIONS
        .iter()
        .enumerate()
        .map(|(i, (value, display))| {
            let is_current = *value == current_key;
            let indicator = if is_current { "●" } else { " " };
            let row = Row::new(vec![
                Cell::from(indicator),
                Cell::from(*display),
                Cell::from(if *value == " " {
                    "<Space>".to_string()
                } else {
                    value.to_string()
                }),
            ]);
            if i == app.leader_cursor {
                row.style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
            } else if is_current {
                row.style(Style::default().fg(Color::Green))
            } else {
                row
            }
        })
        .collect();

    let widths = [
        Constraint::Length(3),
        Constraint::Length(12),
        Constraint::Min(10),
    ];

    let header_row = Row::new(vec![Cell::from(""), Cell::from("Name"), Cell::from("Key")]).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let table = Table::new(rows, widths).header(header_row).block(
        Block::new()
            .borders(Borders::ALL)
            .title(" Select Leader Key (● = current) "),
    );
    frame.render_widget(table, chunks[1]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        " Enter: Apply & Save | Esc: Cancel ",
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}

// ── Edit Settings ───────────────────────────────────────────────────────────

fn draw_edit_settings(frame: &mut Frame, app: &App) {
    let area = frame.area();

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
    .alignment(ratatui::layout::Alignment::Center)
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
        .alignment(ratatui::layout::Alignment::Center);
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
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[chunk_idx]);
}

// ── Confirm Delete ──────────────────────────────────────────────────────────

fn draw_confirm_delete(frame: &mut Frame, name: &str) {
    let area = frame.area();

    let chunks = Layout::vertical(vec![
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(3),
    ])
    .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        " Confirm Delete ",
        Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Confirmation message
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Are you sure you want to delete instance "),
            Span::styled(
                format!("'{name}'"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("?"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  This will permanently remove the instance directory,",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  including its Neovim binary, plugins, config, and data.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Press "),
            Span::styled("y", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" to confirm or "),
            Span::styled("n", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" to cancel."),
        ]),
    ];

    let body = Paragraph::new(lines)
        .block(Block::new().borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(body, chunks[1]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        " y: Confirm Delete | n/Esc: Cancel ",
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}

// ── Tutorial List ───────────────────────────────────────────────────────────

fn draw_tutorial_list(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let has_search = app.tutorial_search_active || !app.tutorial_search.is_empty();
    let chunks = Layout::vertical(vec![
        Constraint::Length(3),
        if has_search { Constraint::Length(3) } else { Constraint::Length(0) },
        Constraint::Min(5),
        Constraint::Length(3),
    ])
    .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        " Tutorials ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Search bar
    if has_search {
        let search_text = if app.tutorial_search_active {
            format!("/ {}_", app.tutorial_search)
        } else {
            format!("/ {}", app.tutorial_search)
        };
        let search_bar = Paragraph::new(Line::from(vec![Span::styled(
            search_text,
            Style::default().fg(Color::Yellow),
        )]))
        .block(
            Block::new()
                .borders(Borders::ALL)
                .border_style(if app.tutorial_search_active {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }),
        );
        frame.render_widget(search_bar, chunks[1]);
    }

    // Topic list using filtered indices
    let list_area = chunks[2];
    if app.tutorial_filtered.is_empty() {
        let msg = if app.tutorial_search.is_empty() {
            "No tutorials available."
        } else {
            "No tutorials match your search."
        };
        let empty = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(Color::DarkGray),
        )))
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::new().borders(Borders::ALL));
        frame.render_widget(empty, list_area);
    } else {
        let visible_height = list_area.height.saturating_sub(4) as usize; // borders + header row + margin
        let offset = if app.tutorial_cursor >= visible_height {
            app.tutorial_cursor - visible_height + 1
        } else {
            0
        };

        let rows: Vec<Row> = app
            .tutorial_filtered
            .iter()
            .enumerate()
            .skip(offset)
            .map(|(display_i, &topic_idx)| {
                let (id, title) = &app.tutorial_topics[topic_idx];
                let style = if display_i == app.tutorial_cursor {
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                Row::new(vec![
                    Cell::from(id.as_str()),
                    Cell::from(title.as_str()),
                ])
                .style(style)
            })
            .collect();

        let widths = [Constraint::Length(20), Constraint::Min(30)];
        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["TOPIC", "DESCRIPTION"])
                    .style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                    .bottom_margin(1),
            )
            .block(Block::new().borders(Borders::ALL));
        frame.render_widget(table, list_area);
    }

    // Footer
    let footer_text = if app.tutorial_search_active {
        " Type to filter | Enter: Accept | Esc: Clear "
    } else {
        " j/k: Navigate | Enter: View | /: Search | Esc: Back "
    };
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        footer_text,
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[3]);
}

// ── Tutorial View ───────────────────────────────────────────────────────────

fn draw_tutorial_view(frame: &mut Frame, app: &App, title: &str, content: &str) {
    let area = frame.area();

    let chunks = Layout::vertical(vec![
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(3),
    ])
    .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        format!(" {title} "),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Content with rich styling
    let raw_lines: Vec<&str> = content.lines().collect();
    let lines: Vec<Line> = raw_lines
        .iter()
        .enumerate()
        .map(|(i, &line)| {
            // Separator lines (=== or ---)
            if !line.is_empty() && line.chars().all(|c| c == '=' || c == '-') {
                return Line::from(Span::styled(
                    line,
                    Style::default().fg(Color::DarkGray),
                ));
            }

            // Heading: line immediately before a separator
            let is_heading = if i + 1 < raw_lines.len() {
                let next = raw_lines[i + 1];
                !next.is_empty() && next.chars().all(|c| c == '=' || c == '-')
            } else {
                false
            };
            if is_heading {
                return Line::from(Span::styled(
                    line,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            // Command/code lines (indented commands with : or $ prefix, or containing pnm/nvim commands)
            let trimmed = line.trim_start();
            if trimmed.starts_with(':')
                || trimmed.starts_with('$')
                || trimmed.starts_with("pnm ")
                || trimmed.starts_with("nvim ")
                || trimmed.starts_with("pip ")
                || trimmed.starts_with("npm ")
                || trimmed.starts_with("dotnet ")
            {
                return Line::from(Span::styled(
                    line,
                    Style::default().fg(Color::Yellow),
                ));
            }

            // Keybinding lines: indented text with a key combo pattern (e.g., "  <leader>r  Run")
            if line.starts_with("  ") && trimmed.starts_with('<') {
                if let Some(pos) = trimmed.find('>') {
                    let key = &trimmed[..pos + 1];
                    let rest = &trimmed[pos + 1..];
                    return Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            key.to_string(),
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(rest.to_string()),
                    ]);
                }
            }

            // Bullet points (- or * at line start)
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                let indent = &line[..line.len() - trimmed.len()];
                let bullet = &trimmed[..2];
                let text = &trimmed[2..];
                return Line::from(vec![
                    Span::raw(indent.to_string()),
                    Span::styled(bullet.to_string(), Style::default().fg(Color::Cyan)),
                    Span::raw(text.to_string()),
                ]);
            }

            // Regular text
            Line::from(line)
        })
        .collect();

    let total_lines = lines.len();
    let visible_height = chunks[1].height.saturating_sub(2) as usize;
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = app.tutorial_scroll.min(max_scroll);

    let paragraph = Paragraph::new(lines)
        .scroll((scroll as u16, 0))
        .wrap(Wrap { trim: false })
        .block(Block::new().borders(Borders::ALL));
    frame.render_widget(paragraph, chunks[1]);

    // Footer with scroll indicator
    let scroll_info = if total_lines > visible_height {
        let pct = if max_scroll > 0 {
            (scroll * 100) / max_scroll
        } else {
            100
        };
        format!(
            " j/k: Scroll | d/u: Page | g/G: Top/Bottom | {}% | Esc: Back ",
            pct
        )
    } else {
        " Esc: Back ".to_string()
    };

    let footer = Paragraph::new(Line::from(vec![Span::styled(
        scroll_info,
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}

fn draw_marketplace(frame: &mut Frame, app: &App, instance_name: &str) {
    let area = frame.area();

    let chunks = Layout::vertical(vec![
        Constraint::Length(3), // Header
        Constraint::Length(3), // Category tabs + search
        Constraint::Min(5),   // Package list
        Constraint::Length(3), // Footer
    ])
    .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        format!(" Marketplace — {instance_name} "),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Category tabs + search
    let categories = ["LSP", "DAP", "Formatter", "Linter", "All"];
    let active_idx = match &app.marketplace_category {
        Some(crate::mason_registry::MasonCategory::Lsp) => 0,
        Some(crate::mason_registry::MasonCategory::Dap) => 1,
        Some(crate::mason_registry::MasonCategory::Formatter) => 2,
        Some(crate::mason_registry::MasonCategory::Linter) => 3,
        None | Some(_) => 4,
    };

    let mut tab_spans: Vec<Span> = Vec::new();
    for (i, cat) in categories.iter().enumerate() {
        if i > 0 {
            tab_spans.push(Span::raw(" │ "));
        }
        if i == active_idx {
            tab_spans.push(Span::styled(
                format!(" {cat} "),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            tab_spans.push(Span::styled(
                format!(" {cat} "),
                Style::default().fg(Color::Gray),
            ));
        }
    }

    if app.marketplace_search_active {
        tab_spans.push(Span::raw("  /"));
        tab_spans.push(Span::styled(
            &app.marketplace_search,
            Style::default().fg(Color::Yellow),
        ));
        tab_spans.push(Span::styled("█", Style::default().fg(Color::Yellow)));
    } else if !app.marketplace_search.is_empty() {
        tab_spans.push(Span::raw("  filter: "));
        tab_spans.push(Span::styled(
            &app.marketplace_search,
            Style::default().fg(Color::Yellow),
        ));
    }

    let tabs = Paragraph::new(Line::from(tab_spans))
        .block(Block::new().borders(Borders::ALL).title(" Category (Tab to switch) "));
    frame.render_widget(tabs, chunks[1]);

    // Package list
    if app.marketplace_registry.is_none() {
        let msg = app
            .marketplace_status
            .as_deref()
            .unwrap_or("No registry loaded. Press R to fetch.");
        let body = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(Color::Yellow),
        )))
        .block(Block::new().borders(Borders::ALL).title(" Packages "));
        frame.render_widget(body, chunks[2]);
    } else if app.marketplace_packages.is_empty() {
        let body =
            Paragraph::new(Line::from(Span::raw("No packages match the current filter.")))
                .block(Block::new().borders(Borders::ALL).title(" Packages "));
        frame.render_widget(body, chunks[2]);
    } else {
        let reg = app.marketplace_registry.as_ref().unwrap();
        let visible_height = (chunks[2].height as usize).saturating_sub(2);
        let scroll_offset = if app.marketplace_cursor >= visible_height {
            app.marketplace_cursor - visible_height + 1
        } else {
            0
        };

        let rows: Vec<Row> = app
            .marketplace_packages
            .iter()
            .skip(scroll_offset)
            .take(visible_height)
            .enumerate()
            .map(|(vis_idx, &pkg_idx)| {
                let pkg = &reg.packages[pkg_idx];
                let actual_idx = scroll_offset + vis_idx;
                let is_cursor = actual_idx == app.marketplace_cursor;
                let is_installed = app.marketplace_installed.contains(&pkg.name);
                let is_selected = app.marketplace_selected.contains(&pkg.name);

                let marker = if is_installed && is_selected {
                    "●+"
                } else if is_installed {
                    " ● "
                } else if is_selected {
                    " + "
                } else {
                    "   "
                };

                let marker_style = if is_selected {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else if is_installed {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default()
                };

                let langs = pkg.languages.join(", ");
                let desc = if pkg.description.len() > 45 {
                    format!("{}...", &pkg.description[..42])
                } else {
                    pkg.description.clone()
                };

                let row_style = if is_cursor {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(Span::styled(marker.to_string(), marker_style)),
                    Cell::from(Span::styled(
                        pkg.name.as_str(),
                        if is_cursor {
                            Style::default().add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        },
                    )),
                    Cell::from(Span::styled(langs, Style::default().fg(Color::Yellow))),
                    Cell::from(Span::raw(desc)),
                ])
                .style(row_style)
            })
            .collect();

        let status_info = if let Some(ref status) = app.marketplace_status {
            format!(" {} ", status)
        } else {
            format!(
                " {} packages | {} selected ",
                app.marketplace_packages.len(),
                app.marketplace_selected.len()
            )
        };

        let table = Table::new(
            rows,
            [
                Constraint::Length(3),
                Constraint::Length(28),
                Constraint::Length(18),
                Constraint::Min(20),
            ],
        )
        .header(
            Row::new(vec![
                Cell::from(Span::styled("", Style::default())),
                Cell::from(Span::styled(
                    "NAME",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "LANGUAGES",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "DESCRIPTION",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
            ])
            .style(Style::default().fg(Color::Cyan)),
        )
        .block(Block::new().borders(Borders::ALL).title(status_info));
        frame.render_widget(table, chunks[2]);
    }

    // Footer
    let footer_text =
        " Esc: Back | Tab: Category | /: Search | Space: Toggle | Enter: Apply | R: Refresh Registry ";
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        footer_text,
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[3]);
}