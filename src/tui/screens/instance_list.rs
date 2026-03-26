use std::io;

use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Terminal,
};

use crate::tui::app::App;
use crate::tui::state::Screen;
use crate::tui::ui::populate_table_cache;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
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
    .alignment(Alignment::Center)
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

    let table_area = chunks[chunk_idx];
    let table = Table::new(rows, widths)
        .header(header_row)
        .block(Block::new().borders(Borders::ALL).title(" Instances "));
    frame.render_widget(table, table_area);
    chunk_idx += 1;

    // Populate layout cache for mouse click-to-select
    populate_table_cache(&mut app.layout_cache, table_area, app.instance_filtered.len());

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
        " q: Quit | Enter: Details | c: Create | /: Search | r: Refresh | s: Settings | t: Tutorials | n: Nerd Font ",
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
    // Handle search input mode
    if app.instance_search_active {
        match code {
            KeyCode::Esc => {
                app.instance_search_active = false;
                app.instance_search.clear();
                app.update_instance_filter();
            }
            KeyCode::Enter => {
                app.instance_search_active = false;
            }
            KeyCode::Backspace => {
                app.instance_search.pop();
                app.update_instance_filter();
            }
            KeyCode::Char(c) => {
                app.instance_search.push(c);
                app.update_instance_filter();
            }
            _ => {}
        }
        return;
    }

    match code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.instance_filtered.is_empty() {
                app.selected = (app.selected + 1) % app.instance_filtered.len();
            }
            app.message = None;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !app.instance_filtered.is_empty() {
                app.selected = if app.selected == 0 {
                    app.instance_filtered.len() - 1
                } else {
                    app.selected - 1
                };
            }
            app.message = None;
        }
        KeyCode::Enter => {
            if let Some(name) = app.selected_name() {
                app.screen = Screen::InstanceDetail { name };
                app.message = None;
            }
        }
        KeyCode::Char('c') => {
            app.enter_create_instance();
        }
        KeyCode::Char('r') => {
            app.refresh_instances();
            app.message = Some("Refreshed instance list.".to_string());
        }
        KeyCode::Char('n') => {
            crate::tui::app::do_install_font(app, terminal).await;
        }
        KeyCode::Char('s') => {
            app.settings_cursor = 0;
            app.settings_editing = false;
            app.settings_edit_buffer.clear();
            app.screen = Screen::EditSettings;
            app.message = None;
        }
        KeyCode::Char('t') => {
            app.open_tutorial_list(Screen::InstanceList);
        }
        KeyCode::Char('/') => {
            app.instance_search_active = true;
            app.instance_search.clear();
            app.message = None;
        }
        _ => {}
    }
}
