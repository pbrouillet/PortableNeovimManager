use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::config::LEADER_KEY_OPTIONS;
use crate::tui::app::App;
use crate::tui::state::Screen;

pub fn draw(frame: &mut Frame, app: &mut App, name: &str, area: Rect) {
    let chunks = ratatui::layout::Layout::vertical(vec![
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
    .alignment(Alignment::Center)
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
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}

pub fn handle_keys(app: &mut App, code: KeyCode, name: &str) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.screen = Screen::InstanceDetail {
                name: name.to_string(),
            };
            app.message = None;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.leader_cursor = (app.leader_cursor + 1) % LEADER_KEY_OPTIONS.len();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.leader_cursor = if app.leader_cursor == 0 {
                LEADER_KEY_OPTIONS.len() - 1
            } else {
                app.leader_cursor - 1
            };
        }
        KeyCode::Enter => {
            app.apply_leader_key(name);
            app.screen = Screen::InstanceDetail {
                name: name.to_string(),
            };
        }
        _ => {}
    }
}
