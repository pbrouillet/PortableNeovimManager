use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};

use crate::tui::app::App;
use crate::tui::state::Screen;

pub fn draw_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let has_search = app.tutorial_search_active || !app.tutorial_search.is_empty();
    let chunks = ratatui::layout::Layout::vertical(vec![
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
    .alignment(Alignment::Center)
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
        .alignment(Alignment::Center)
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
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[3]);
}

pub fn draw_view(frame: &mut Frame, app: &mut App, title: &str, content: &str, area: Rect) {
    let chunks = ratatui::layout::Layout::vertical(vec![
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
    .alignment(Alignment::Center)
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
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}

pub fn handle_list_keys(app: &mut App, code: KeyCode) {
    if app.tutorial_search_active {
        match code {
            KeyCode::Esc => {
                app.tutorial_search_active = false;
                app.tutorial_search.clear();
                app.update_tutorial_filter();
            }
            KeyCode::Enter => {
                app.tutorial_search_active = false;
            }
            KeyCode::Backspace => {
                app.tutorial_search.pop();
                app.update_tutorial_filter();
            }
            KeyCode::Char(c) => {
                app.tutorial_search.push(c);
                app.update_tutorial_filter();
            }
            _ => {}
        }
        return;
    }

    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.tutorial_return();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.tutorial_filtered.is_empty() {
                app.tutorial_cursor = (app.tutorial_cursor + 1) % app.tutorial_filtered.len();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !app.tutorial_filtered.is_empty() {
                app.tutorial_cursor = if app.tutorial_cursor == 0 {
                    app.tutorial_filtered.len() - 1
                } else {
                    app.tutorial_cursor - 1
                };
            }
        }
        KeyCode::Enter => {
            if let Some(&topic_idx) = app.tutorial_filtered.get(app.tutorial_cursor) {
                if let Some((id, _)) = app.tutorial_topics.get(topic_idx) {
                    if let Some((title, content)) = app.registry.tutorial_content(id) {
                        app.tutorial_scroll = 0;
                        // Return to TutorialList (not the original caller) from the view
                        app.tutorial_return_screen = Some(Box::new(Screen::TutorialList));
                        app.screen = Screen::TutorialView { title, content };
                    }
                }
            }
        }
        KeyCode::Char('/') => {
            app.tutorial_search_active = true;
        }
        _ => {}
    }
}

pub fn handle_view_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.tutorial_return();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.tutorial_scroll = app.tutorial_scroll.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.tutorial_scroll = app.tutorial_scroll.saturating_sub(1);
        }
        KeyCode::PageDown | KeyCode::Char('d') => {
            app.tutorial_scroll = app.tutorial_scroll.saturating_add(10);
        }
        KeyCode::PageUp | KeyCode::Char('u') => {
            app.tutorial_scroll = app.tutorial_scroll.saturating_sub(10);
        }
        KeyCode::Home | KeyCode::Char('g') => {
            app.tutorial_scroll = 0;
        }
        KeyCode::End | KeyCode::Char('G') => {
            // Set to a large number; rendering will clamp
            app.tutorial_scroll = usize::MAX;
        }
        _ => {}
    }
}
