use std::io;

use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};

use crate::config;
use crate::tui::app::{self, App};
use crate::tui::state::Screen;

pub fn draw(frame: &mut Frame, app: &mut App, name: &str, area: Rect) {
    let has_message = app.message.is_some();
    let mut constraints = vec![
        Constraint::Length(3),  // header
        Constraint::Min(5),    // body
    ];
    if has_message {
        constraints.push(Constraint::Length(2)); // status message
    }
    constraints.push(Constraint::Length(3)); // footer

    let chunks = Layout::vertical(constraints).split(area);
    let mut chunk_idx = 0;

    // Header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        format!(" Instance: {name} "),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(header, chunks[chunk_idx]);
    chunk_idx += 1;

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
        let runtime_display = crate::runtime::runtime_display_name(inst.js_runtime.as_deref());
        let created_str = inst.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let updated_str = inst.updated_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let lines = vec![
            Line::from(vec![
                Span::styled("Name:       ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(inst.name.clone()),
            ]),
            Line::from(vec![
                Span::styled("Version:    ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(inst.nvim_version.clone()),
            ]),
            Line::from(vec![
                Span::styled("Features:   ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(features_str),
            ]),
            Line::from(vec![
                Span::styled("Leader:     ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    leader_display.to_string(),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(vec![
                Span::styled("JS Runtime: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    runtime_display.to_string(),
                    if inst.js_runtime.is_some() {
                        Style::default().fg(Color::Magenta)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled("Created:    ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(created_str),
            ]),
            Line::from(vec![
                Span::styled("Updated:    ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(updated_str),
            ]),
            Line::from(vec![
                Span::styled("Init Config:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::raw({
                    let pre_lines = inst.init_lua_pre.as_ref().map(|s| s.lines().count()).unwrap_or(0);
                    let post_lines = inst.init_lua_post.as_ref().map(|s| s.lines().count()).unwrap_or(0);
                    if pre_lines == 0 && post_lines == 0 {
                        "(none)".to_string()
                    } else {
                        format!("{pre_lines} lines pre, {post_lines} lines post")
                    }
                }),
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
    frame.render_widget(body, chunks[chunk_idx]);
    chunk_idx += 1;

    // Status message (optional)
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
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        " Esc: Back | l: Launch | u: Update | d: Delete | f: Features | p: Packages | m: Leader | B: Bun | M: Monitor | I: Init Config | o: Open Dir ",
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[chunk_idx]);
}

pub async fn handle_keys(
    app: &mut App,
    code: KeyCode,
    name: &str,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.screen = Screen::InstanceList;
            app.message = None;
        }
        KeyCode::Char('l') => {
            app::do_launch(name, app, terminal);
        }
        KeyCode::Char('u') => {
            app::do_update(name, app, terminal).await;
        }
        KeyCode::Char('d') => {
            app.screen = Screen::ConfirmDelete { name: name.to_string() };
            app.message = None;
        }
        KeyCode::Char('f') => {
            app.enter_edit_features(name);
        }
        KeyCode::Char('m') => {
            app.enter_edit_leader(name);
        }
        KeyCode::Char('o') => {
            app::open_instance_dir(name, app);
        }
        KeyCode::Char('t') => {
            app.open_tutorial_list(Screen::InstanceDetail { name: name.to_string() });
        }
        KeyCode::Char('p') => {
            app.enter_marketplace(name);
        }
        KeyCode::Char('M') => {
            app.enter_monitor(name);
        }
        KeyCode::Char('B') => {
            app::toggle_bun_runtime(name, app);
        }
        KeyCode::Char('I') => {
            app.enter_init_config(name);
        }
        _ => {}
    }
}
