use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::tui::app::App;
use crate::tui::state::Screen;
use crate::tui::ui::populate_table_cache;

pub fn draw(frame: &mut Frame, app: &mut App, instance_name: &str, area: Rect) {
    let chunks = ratatui::layout::Layout::vertical(vec![
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
    .alignment(Alignment::Center)
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
        populate_table_cache(&mut app.layout_cache, chunks[2], app.marketplace_packages.len());
    }

    // Footer
    let footer_text =
        " Esc: Back | Tab: Category | /: Search | Space: Toggle | Enter: Apply | R: Refresh Registry ";
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        footer_text,
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[3]);
}

pub async fn handle_keys(app: &mut App, code: KeyCode, instance_name: &str) {
    if app.marketplace_search_active {
        match code {
            KeyCode::Esc => {
                app.marketplace_search_active = false;
                app.marketplace_search.clear();
                app.update_marketplace_filter();
            }
            KeyCode::Enter => {
                app.marketplace_search_active = false;
            }
            KeyCode::Backspace => {
                app.marketplace_search.pop();
                app.update_marketplace_filter();
            }
            KeyCode::Char(c) => {
                app.marketplace_search.push(c);
                app.update_marketplace_filter();
            }
            _ => {}
        }
        return;
    }

    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.screen = Screen::InstanceDetail {
                name: instance_name.to_string(),
            };
            app.message = None;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.marketplace_packages.is_empty() {
                app.marketplace_cursor =
                    (app.marketplace_cursor + 1) % app.marketplace_packages.len();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !app.marketplace_packages.is_empty() {
                app.marketplace_cursor = if app.marketplace_cursor == 0 {
                    app.marketplace_packages.len() - 1
                } else {
                    app.marketplace_cursor - 1
                };
            }
        }
        KeyCode::Char(' ') => {
            app.marketplace_toggle_selected();
        }
        KeyCode::Tab => {
            use crate::mason_registry::MasonCategory;
            app.marketplace_category = match &app.marketplace_category {
                Some(MasonCategory::Lsp) => Some(MasonCategory::Dap),
                Some(MasonCategory::Dap) => Some(MasonCategory::Formatter),
                Some(MasonCategory::Formatter) => Some(MasonCategory::Linter),
                Some(MasonCategory::Linter) => None,
                None | Some(_) => Some(MasonCategory::Lsp),
            };
            app.update_marketplace_filter();
        }
        KeyCode::BackTab => {
            use crate::mason_registry::MasonCategory;
            app.marketplace_category = match &app.marketplace_category {
                Some(MasonCategory::Lsp) => None,
                Some(MasonCategory::Dap) => Some(MasonCategory::Lsp),
                Some(MasonCategory::Formatter) => Some(MasonCategory::Dap),
                Some(MasonCategory::Linter) => Some(MasonCategory::Formatter),
                None | Some(_) => Some(MasonCategory::Linter),
            };
            app.update_marketplace_filter();
        }
        KeyCode::Enter => match app.marketplace_apply(instance_name) {
            Ok(0) => app.message = Some("No new packages to add.".to_string()),
            Ok(n) => {
                app.message = Some(format!("✓ Added {n} package(s). Launch to install."));
                app.refresh_instances();
            }
            Err(e) => app.message = Some(format!("Error: {e}")),
        },
        KeyCode::Char('/') => {
            app.marketplace_search_active = true;
            app.marketplace_search.clear();
            app.message = None;
        }
        KeyCode::Char('R') => {
            app.marketplace_status = Some("Fetching registry from GitHub...".to_string());
            match crate::mason_registry::fetch_registry(true).await {
                Ok(reg) => {
                    let count = reg.len();
                    app.marketplace_registry = Some(reg);
                    app.update_marketplace_filter();
                    app.marketplace_status =
                        Some(format!("✓ Registry refreshed. {count} packages."));
                }
                Err(e) => {
                    app.marketplace_status = Some(format!("Error: {e}"));
                }
            }
        }
        _ => {}
    }
}
