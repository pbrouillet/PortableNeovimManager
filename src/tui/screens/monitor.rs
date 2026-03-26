use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::monitor;
use crate::tui::app::App;
use crate::tui::state::Screen;

pub fn draw(frame: &mut Frame, app: &mut App, name: &str, area: Rect) {
    let chunks = Layout::vertical(vec![
        Constraint::Length(3),  // header
        Constraint::Min(5),    // body
        Constraint::Length(3), // footer
    ])
    .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        format!(" Memory Monitor: {name} "),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Body
    let body_content = if let Some(ref err) = app.monitor_error {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {err}"),
                Style::default().fg(Color::Red),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Instance is not running or PID file not found.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "  Launch the instance first with 'l' from the detail screen.",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    } else if let Some(ref snap) = app.monitor_snapshot {
        let mut lines = Vec::new();

        // Neovim process section
        lines.push(Line::from(vec![
            Span::styled("  PID: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(snap.nvim_process.pid.to_string()),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Neovim Process",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(vec![
            Span::styled(
                "    Working Set:    ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                monitor::format_bytes(snap.nvim_process.working_set_bytes),
                Style::default().fg(Color::Green),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                "    Virtual Memory: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(monitor::format_bytes(snap.nvim_process.virtual_memory_bytes)),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                "    CPU:            ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{:.1}%", snap.nvim_process.cpu_percent)),
        ]));

        // Lua heap
        if let Some(lua_bytes) = snap.lua_memory_bytes {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(
                    "  Lua Heap:         ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    monitor::format_bytes(lua_bytes),
                    Style::default().fg(Color::Green),
                ),
            ]));
        }

        // Child processes
        lines.push(Line::from(""));
        if snap.child_processes.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No child processes.",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("  Child Processes ({})", snap.child_processes.len()),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("    {:<8} {:<25} {:<15} {}", "PID", "NAME", "WORKING SET", "VIRTUAL"),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            for child in &snap.child_processes {
                let name_display = if child.name.len() > 24 {
                    format!("{}…", &child.name[..23])
                } else {
                    child.name.clone()
                };
                lines.push(Line::from(format!(
                    "    {:<8} {:<25} {:<15} {}",
                    child.pid,
                    name_display,
                    monitor::format_bytes(child.working_set_bytes),
                    monitor::format_bytes(child.virtual_memory_bytes),
                )));
            }
        }

        // Totals
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "  Total Working Set:  ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                monitor::format_bytes(snap.total_working_set_bytes),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                "  Total Virtual:      ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(monitor::format_bytes(snap.total_virtual_memory_bytes)),
        ]));

        // Timestamp
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  Last updated: {}", snap.timestamp.format("%H:%M:%S UTC")),
            Style::default().fg(Color::DarkGray),
        )));

        lines
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Loading...",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    };

    let body = Paragraph::new(body_content)
        .block(Block::new().borders(Borders::ALL).title(" Memory Usage "))
        .wrap(Wrap { trim: false });
    frame.render_widget(body, chunks[1]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        " Esc: Back | r: Refresh | Auto-refreshes every 2s ",
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
        KeyCode::Char('r') => {
            app.refresh_monitor(name);
        }
        _ => {}
    }
}
