use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};

use crate::config::{self, LEADER_KEY_OPTIONS};

use super::app::{App, Screen};

pub fn draw(frame: &mut Frame, app: &App) {
    match &app.screen {
        Screen::InstanceList => draw_instance_list(frame, app),
        Screen::InstanceDetail { name } => draw_instance_detail(frame, app, name),
        Screen::EditFeatures { name } => draw_edit_features(frame, app, name),
        Screen::EditLeaderKey { name } => draw_edit_leader(frame, app, name),
        Screen::TutorialList => draw_tutorial_list(frame, app),
        Screen::TutorialView { title, content, .. } => draw_tutorial_view(frame, app, title, content),
    }
}

fn draw_instance_list(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Determine if we have a status message to show.
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
        " Portable Neovim Manager ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Instance table
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
        .instances
        .iter()
        .enumerate()
        .map(|(i, inst)| {
            let features_str = inst
                .features
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
            if i == app.selected {
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
        " q: Quit | Enter: Details | t: Tutorials | f: Features | m: Leader | o: Open Dir | n: Nerd Font | s: Settings | l: Launch | u: Update | d: Delete ",
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[footer_idx]);
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
            .features
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
            " {} | Esc: Back | f: Features | m: Leader Key | o: Open Dir | l: Launch | u: Update | d: Delete ",
            app.message.as_deref().unwrap_or("")
        )
    } else {
        " Esc: Back | f: Features | m: Leader Key | o: Open Dir | l: Launch | u: Update | d: Delete ".to_string()
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

    // Checkbox list
    let rows: Vec<Row> = app
        .feature_checkboxes
        .iter()
        .enumerate()
        .map(|(i, (workload_id, enabled))| {
            let checkbox = if *enabled { "[x]" } else { "[ ]" };
            let workload = app.registry.find_by_id(workload_id);
            let name = workload.map(|w| w.name.as_str()).unwrap_or(workload_id);
            let desc = workload.map(|w| w.description.as_str()).unwrap_or("");
            let row = Row::new(vec![
                Cell::from(checkbox.to_string()),
                Cell::from(name.to_string()),
                Cell::from(desc.to_string()),
            ]);
            if i == app.feature_cursor {
                row.style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
            } else if *enabled {
                row.style(Style::default().fg(Color::Green))
            } else {
                row
            }
        })
        .collect();

    let widths = [
        Constraint::Length(5),
        Constraint::Length(12),
        Constraint::Min(20),
    ];

    let header_row = Row::new(vec![
        Cell::from(""),
        Cell::from("Feature"),
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
        " Space: Toggle | t: Tutorial | Enter: Apply & Save | Esc: Cancel ",
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

// ── Tutorial List ───────────────────────────────────────────────────────────

fn draw_tutorial_list(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::vertical(vec![
        Constraint::Length(3),
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

    // Topic list
    let rows: Vec<Row> = app
        .tutorial_topics
        .iter()
        .enumerate()
        .map(|(i, (id, title))| {
            let style = if i == app.tutorial_cursor {
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
    frame.render_widget(table, chunks[1]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        " j/k: Navigate | Enter: View | Esc: Back ",
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
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

    // Content with scroll
    let lines: Vec<Line> = content
        .lines()
        .map(|line| {
            if line.chars().all(|c| c == '=' || c == '-') && !line.is_empty() {
                Line::from(Span::styled(line, Style::default().fg(Color::DarkGray)))
            } else if line.starts_with("  ") && line.contains("  ") {
                Line::from(Span::styled(line, Style::default().fg(Color::Green)))
            } else {
                Line::from(line)
            }
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

    // Footer
    let scroll_info = if total_lines > visible_height {
        format!(
            " j/k: Scroll | d/u: Page | g/G: Top/Bottom | Line {}/{} | Esc: Back ",
            scroll + 1,
            total_lines
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
