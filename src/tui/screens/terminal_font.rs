use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::tui::app::{self, App};
use crate::tui::state::Screen;
use crate::tui::ui::populate_table_cache;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = ratatui::layout::Layout::vertical(vec![
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(3),
    ])
    .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        " Configure Terminal Font ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Body: list of toggleable items
    let mut rows = Vec::new();

    // Row 0: "Apply to all profiles (defaults)" toggle
    let defaults_check = if app.terminal_apply_defaults { "[✓]" } else { "[ ]" };
    let defaults_style = if app.terminal_cursor == 0 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    rows.push(Row::new(vec![
        Cell::from(Span::styled(
            format!(" {defaults_check} Apply to all profiles (defaults)"),
            defaults_style,
        )),
        Cell::from(Span::styled("all installations", Style::default().fg(Color::DarkGray))),
        Cell::from(""),
    ]));

    // Profile rows
    for (i, entry) in app.terminal_entries.iter().enumerate() {
        let row_idx = i + 1;

        let check = if entry.read_only {
            " ℹ "
        } else if entry.selected {
            "[✓]"
        } else {
            "[ ]"
        };
        let style = if app.terminal_cursor == row_idx {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let font_display = entry
            .current_font
            .as_deref()
            .unwrap_or("(default)");

        rows.push(Row::new(vec![
            Cell::from(Span::styled(
                format!(" {check} {}", entry.name),
                style,
            )),
            Cell::from(Span::styled(
                entry.install_label.as_str(),
                Style::default().fg(Color::DarkGray),
            )),
            Cell::from(Span::styled(
                font_display,
                Style::default().fg(Color::DarkGray),
            )),
        ]));
    }

    let widths = [
        Constraint::Percentage(45),
        Constraint::Percentage(30),
        Constraint::Percentage(25),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec![
                Cell::from(Span::styled(
                    " Profile",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "Installation",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "Current Font",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
            ])
            .style(Style::default().fg(Color::Cyan)),
        )
        .block(Block::new().borders(Borders::ALL));

    frame.render_widget(table, chunks[1]);
    populate_table_cache(&mut app.layout_cache, chunks[1], app.terminal_entries.len());

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" Space", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(": Toggle | "),
        Span::styled("Enter/a", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(": Apply | "),
        Span::styled("Esc", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(": Cancel"),
    ]))
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}

pub fn handle_keys(app: &mut App, code: KeyCode) {
    // Total items: 1 ("Apply to all" toggle) + profile entries
    let total = 1 + app.terminal_entries.len();

    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.screen = Screen::InstanceList;
            app.message = Some("Terminal font configuration cancelled.".to_string());
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.terminal_cursor > 0 {
                app.terminal_cursor -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.terminal_cursor + 1 < total {
                app.terminal_cursor += 1;
            }
        }
        KeyCode::Char(' ') => {
            if app.terminal_cursor == 0 {
                app.terminal_apply_defaults = !app.terminal_apply_defaults;
            } else {
                let idx = app.terminal_cursor - 1;
                if let Some(entry) = app.terminal_entries.get_mut(idx) {
                    entry.selected = !entry.selected;
                }
            }
        }
        KeyCode::Enter | KeyCode::Char('a') => {
            app::do_apply_terminal_font(app);
        }
        _ => {}
    }
}
