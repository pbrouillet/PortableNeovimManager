use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::tui::app::App;
use crate::tui::state::Screen;

pub fn draw(frame: &mut Frame, name: &str, area: Rect) {
    let chunks = ratatui::layout::Layout::vertical(vec![
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
    .alignment(Alignment::Center)
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
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}

pub fn handle_keys(app: &mut App, code: KeyCode, name: &str) {
    match code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            match crate::instance::delete(name, &app.settings) {
                Ok(()) => {
                    app.refresh_instances();
                    app.message = Some(format!("Deleted '{name}'."));
                    app.screen = Screen::InstanceList;
                }
                Err(e) => {
                    app.message = Some(format!("Delete failed: {e}"));
                    app.screen = Screen::InstanceList;
                }
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.message = Some("Delete cancelled.".to_string());
            // Return to detail if we came from there, otherwise list
            if app.instances.iter().any(|i| i.name == name) {
                app.screen = Screen::InstanceDetail { name: name.to_string() };
            } else {
                app.screen = Screen::InstanceList;
            }
        }
        _ => {}
    }
}
