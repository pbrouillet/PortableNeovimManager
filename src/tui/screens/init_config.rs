use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::config;
use crate::plugins;
use crate::tui::app::App;
use crate::tui::lua_highlight;
use crate::tui::state::Screen;

pub fn draw(frame: &mut Frame, app: &mut App, name: &str, area: Rect) {
    if app.init_config_editing {
        draw_editor(frame, app, name, area);
        return;
    }

    // View mode with syntax highlighting
    let dir = config::instance_dir(&app.settings, name);
    let manifest_path = config::InstanceManifest::manifest_path(&dir);
    let manifest = config::InstanceManifest::load(&manifest_path).ok();

    let eff_pre = manifest.as_ref()
        .map(|m| plugins::resolve_init_lua_pre(m, &app.settings))
        .unwrap_or(None);
    let eff_post = manifest.as_ref()
        .map(|m| plugins::resolve_init_lua_post(m, &app.settings))
        .unwrap_or(None);

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Percentage(50),
        Constraint::Percentage(50),
        Constraint::Length(3),
    ]).split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        format!(" Init Config — {name} "),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )]))
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Pre-plugins panel with syntax highlighting
    let pre_border_style = if app.init_config_panel == 0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let pre_lines = highlight_lua_text(eff_pre.as_deref(), "(empty — no pre-plugins overrides)");
    let pre_widget = Paragraph::new(pre_lines)
        .scroll((app.init_config_pre_scroll, 0))
        .block(
            Block::new()
                .borders(Borders::ALL)
                .border_style(pre_border_style)
                .title(" Pre-plugins Lua (before lazy.setup) "),
        );
    frame.render_widget(pre_widget, chunks[1]);

    // Post-plugins panel with syntax highlighting
    let post_border_style = if app.init_config_panel == 1 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let post_lines = highlight_lua_text(eff_post.as_deref(), "(empty — no post-plugins overrides)");
    let post_widget = Paragraph::new(post_lines)
        .scroll((app.init_config_post_scroll, 0))
        .block(
            Block::new()
                .borders(Borders::ALL)
                .border_style(post_border_style)
                .title(" Post-plugins Lua (after plugin setup) "),
        );
    frame.render_widget(post_widget, chunks[2]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        " Tab: Switch panel | ↑/↓: Scroll | e: Edit | d: Reset defaults | Esc: Back ",
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[3]);
}

fn highlight_lua_text<'a>(text: Option<&str>, empty_msg: &str) -> Vec<Line<'a>> {
    match text {
        Some(s) if !s.trim().is_empty() => {
            s.lines().map(|l| Line::from(lua_highlight::highlight_lua_line(l))).collect()
        }
        _ => vec![Line::from(Span::styled(
            empty_msg.to_string(),
            Style::default().fg(Color::DarkGray),
        ))],
    }
}

pub fn draw_editor(frame: &mut Frame, app: &mut App, name: &str, area: Rect) {
    let panel_label = if app.init_config_panel == 0 {
        "Pre-plugins"
    } else {
        "Post-plugins"
    };

    let chunks = Layout::vertical([
        Constraint::Length(3),  // header
        Constraint::Min(5),    // editor
        Constraint::Length(3), // footer
    ]).split(area);

    // Header with dirty indicator
    let dirty_indicator = if app.init_config_dirty { " [modified]" } else { "" };
    let header = Paragraph::new(Line::from(vec![Span::styled(
        format!(" Editing {panel_label} — {name}{dirty_indicator} "),
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )]))
    .alignment(Alignment::Center)
    .block(Block::new().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Editor area (inside borders)
    let editor_block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(format!(" {panel_label} Lua "));
    let inner = editor_block.inner(chunks[1]);
    frame.render_widget(editor_block, chunks[1]);

    let visible_height = inner.height as usize;
    let gutter_width: u16 = 4; // "NN │"

    // Auto-scroll: ensure cursor is visible
    let mut scroll = app.init_config_editor_scroll as usize;
    if app.init_config_cursor_row >= scroll + visible_height {
        scroll = app.init_config_cursor_row - visible_height + 1;
    }
    if app.init_config_cursor_row < scroll {
        scroll = app.init_config_cursor_row;
    }

    // Render lines with gutter + syntax highlighting
    for (vi, line_idx) in (scroll..app.init_config_buffer.len().min(scroll + visible_height)).enumerate() {
        let y = inner.y + vi as u16;
        let line_content = &app.init_config_buffer[line_idx];

        // Line number gutter
        let line_num = format!("{:>2} │", line_idx + 1);
        let gutter_style = if line_idx == app.init_config_cursor_row {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        frame.render_widget(
            Paragraph::new(Span::styled(line_num, gutter_style)),
            Rect::new(inner.x, y, gutter_width, 1),
        );

        // Syntax-highlighted content
        let content_x = inner.x + gutter_width;
        let content_width = inner.width.saturating_sub(gutter_width);
        let spans = lua_highlight::highlight_lua_line(line_content);

        // Highlight cursor line background
        if line_idx == app.init_config_cursor_row {
            let bg = Paragraph::new(Line::from(spans))
                .style(Style::default().bg(Color::Rgb(40, 40, 50)));
            frame.render_widget(
                bg,
                Rect::new(content_x, y, content_width, 1),
            );
        } else {
            frame.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect::new(content_x, y, content_width, 1),
            );
        }

        // Render cursor
        if line_idx == app.init_config_cursor_row {
            let cursor_screen_col = app.init_config_cursor_col as u16;
            if cursor_screen_col < content_width {
                let cursor_x = content_x + cursor_screen_col;
                let cursor_char = if app.init_config_cursor_col < line_content.len() {
                    line_content.chars().nth(app.init_config_cursor_col).unwrap_or(' ')
                } else {
                    ' '
                };
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        cursor_char.to_string(),
                        Style::default().bg(Color::White).fg(Color::Black),
                    )),
                    Rect::new(cursor_x, y, 1, 1),
                );
            }
        }
    }

    // Footer — show discard prompt or normal keybinds
    let footer_text = if app.init_config_confirm_discard {
        " ⚠ Unsaved changes! Press y to discard, any other key to cancel "
    } else {
        " Ctrl+S: Save | Esc: Exit | Ctrl+Z: Undo | Ctrl+Y: Redo | Tab: Indent "
    };
    let footer_style = if app.init_config_confirm_discard {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let footer = Paragraph::new(Line::from(vec![Span::styled(footer_text, footer_style)]))
        .alignment(Alignment::Center)
        .block(Block::new().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}

pub fn handle_keys(app: &mut App, key: KeyEvent, name: &str) {
    if app.init_config_editing {
        handle_editor_keys(app, key, name);
        return;
    }

    // View mode
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.screen = Screen::InstanceDetail { name: name.to_string() };
            app.message = None;
        }
        KeyCode::Tab => {
            app.init_config_panel = 1 - app.init_config_panel;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.init_config_panel == 0 {
                app.init_config_pre_scroll = app.init_config_pre_scroll.saturating_add(1);
            } else {
                app.init_config_post_scroll = app.init_config_post_scroll.saturating_add(1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.init_config_panel == 0 {
                app.init_config_pre_scroll = app.init_config_pre_scroll.saturating_sub(1);
            } else {
                app.init_config_post_scroll = app.init_config_post_scroll.saturating_sub(1);
            }
        }
        KeyCode::Char('e') => {
            // Enter edit mode for the focused panel
            let dir = config::instance_dir(&app.settings, name);
            let manifest_path = config::InstanceManifest::manifest_path(&dir);
            if let Ok(manifest) = config::InstanceManifest::load(&manifest_path) {
                let content = if app.init_config_panel == 0 {
                    plugins::resolve_init_lua_pre(&manifest, &app.settings)
                } else {
                    plugins::resolve_init_lua_post(&manifest, &app.settings)
                };
                let text = content.unwrap_or_default();
                app.init_config_buffer = if text.is_empty() {
                    vec![String::new()]
                } else {
                    text.lines().map(|l| l.to_string()).collect()
                };
                app.init_config_cursor_row = 0;
                app.init_config_cursor_col = 0;
                app.init_config_editor_scroll = 0;
                app.init_config_undo.clear();
                app.init_config_redo.clear();
                app.init_config_original_buffer = app.init_config_buffer.clone();
                app.init_config_dirty = false;
                app.init_config_confirm_discard = false;
                app.init_config_editing = true;
                app.message = None;
            }
        }
        KeyCode::Char('d') => {
            reset_init_config_defaults(app, name);
        }
        _ => {}
    }
}

pub fn handle_editor_keys(app: &mut App, key: KeyEvent, name: &str) {
    // Handle discard confirmation prompt
    if app.init_config_confirm_discard {
        match key.code {
            KeyCode::Char('y') => {
                // Discard changes and exit
                app.init_config_confirm_discard = false;
                app.init_config_editing = false;
                app.init_config_dirty = false;
                app.message = Some("Changes discarded.".to_string());
            }
            _ => {
                // Cancel prompt, stay in editor
                app.init_config_confirm_discard = false;
            }
        }
        return;
    }

    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    match key.code {
        KeyCode::Esc => {
            if app.init_config_dirty {
                app.init_config_confirm_discard = true;
            } else {
                app.init_config_editing = false;
            }
        }
        KeyCode::Char('s') if ctrl => {
            save_init_config_editor(app, name);
            app.init_config_dirty = false;
            app.init_config_editing = false;
        }
        KeyCode::Char('z') if ctrl => {
            // Undo
            if let Some((buf, row, col)) = app.init_config_undo.pop() {
                app.init_config_redo.push((
                    app.init_config_buffer.clone(),
                    app.init_config_cursor_row,
                    app.init_config_cursor_col,
                ));
                app.init_config_buffer = buf;
                app.init_config_cursor_row = row;
                app.init_config_cursor_col = col;
            }
        }
        KeyCode::Char('y') if ctrl => {
            // Redo
            if let Some((buf, row, col)) = app.init_config_redo.pop() {
                app.init_config_undo.push((
                    app.init_config_buffer.clone(),
                    app.init_config_cursor_row,
                    app.init_config_cursor_col,
                ));
                app.init_config_buffer = buf;
                app.init_config_cursor_row = row;
                app.init_config_cursor_col = col;
            }
        }
        KeyCode::Char(c) if !ctrl => {
            push_editor_undo(app);
            let row = app.init_config_cursor_row;
            let col = app.init_config_cursor_col;
            app.init_config_buffer[row].insert(col, c);
            app.init_config_cursor_col += 1;
        }
        KeyCode::Enter => {
            push_editor_undo(app);
            let row = app.init_config_cursor_row;
            let col = app.init_config_cursor_col;
            let rest = app.init_config_buffer[row][col..].to_string();
            app.init_config_buffer[row].truncate(col);
            app.init_config_buffer.insert(row + 1, rest);
            app.init_config_cursor_row += 1;
            app.init_config_cursor_col = 0;
        }
        KeyCode::Backspace => {
            let row = app.init_config_cursor_row;
            let col = app.init_config_cursor_col;
            if col > 0 {
                push_editor_undo(app);
                app.init_config_buffer[row].remove(col - 1);
                app.init_config_cursor_col -= 1;
            } else if row > 0 {
                push_editor_undo(app);
                let line = app.init_config_buffer.remove(row);
                app.init_config_cursor_row -= 1;
                app.init_config_cursor_col = app.init_config_buffer[row - 1].len();
                app.init_config_buffer[row - 1].push_str(&line);
            }
        }
        KeyCode::Delete => {
            let row = app.init_config_cursor_row;
            let col = app.init_config_cursor_col;
            if col < app.init_config_buffer[row].len() {
                push_editor_undo(app);
                app.init_config_buffer[row].remove(col);
            } else if row + 1 < app.init_config_buffer.len() {
                push_editor_undo(app);
                let next = app.init_config_buffer.remove(row + 1);
                app.init_config_buffer[row].push_str(&next);
            }
        }
        KeyCode::Left => {
            if app.init_config_cursor_col > 0 {
                app.init_config_cursor_col -= 1;
            } else if app.init_config_cursor_row > 0 {
                app.init_config_cursor_row -= 1;
                app.init_config_cursor_col = app.init_config_buffer[app.init_config_cursor_row].len();
            }
        }
        KeyCode::Right => {
            let row = app.init_config_cursor_row;
            if app.init_config_cursor_col < app.init_config_buffer[row].len() {
                app.init_config_cursor_col += 1;
            } else if row + 1 < app.init_config_buffer.len() {
                app.init_config_cursor_row += 1;
                app.init_config_cursor_col = 0;
            }
        }
        KeyCode::Up => {
            if app.init_config_cursor_row > 0 {
                app.init_config_cursor_row -= 1;
                let line_len = app.init_config_buffer[app.init_config_cursor_row].len();
                app.init_config_cursor_col = app.init_config_cursor_col.min(line_len);
            }
        }
        KeyCode::Down => {
            if app.init_config_cursor_row + 1 < app.init_config_buffer.len() {
                app.init_config_cursor_row += 1;
                let line_len = app.init_config_buffer[app.init_config_cursor_row].len();
                app.init_config_cursor_col = app.init_config_cursor_col.min(line_len);
            }
        }
        KeyCode::Home => {
            app.init_config_cursor_col = 0;
        }
        KeyCode::End => {
            app.init_config_cursor_col = app.init_config_buffer[app.init_config_cursor_row].len();
        }
        KeyCode::Tab => {
            push_editor_undo(app);
            let row = app.init_config_cursor_row;
            let col = app.init_config_cursor_col;
            app.init_config_buffer[row].insert_str(col, "  ");
            app.init_config_cursor_col += 2;
        }
        _ => {}
    }

    // Recompute dirty flag
    app.init_config_dirty = app.init_config_buffer != app.init_config_original_buffer;

    // Auto-scroll to keep cursor visible
    let scroll = app.init_config_editor_scroll as usize;
    if app.init_config_cursor_row < scroll {
        app.init_config_editor_scroll = app.init_config_cursor_row as u16;
    }
    // We'll clamp max scroll in the draw function where we know the viewport height
}

fn push_editor_undo(app: &mut App) {
    app.init_config_undo.push((
        app.init_config_buffer.clone(),
        app.init_config_cursor_row,
        app.init_config_cursor_col,
    ));
    app.init_config_redo.clear();
    // Limit undo stack
    if app.init_config_undo.len() > 100 {
        app.init_config_undo.remove(0);
    }
}

fn save_init_config_editor(app: &mut App, name: &str) {
    let text = app.init_config_buffer.join("\n");
    let value = if text.trim().is_empty() { None } else { Some(text) };

    let dir = config::instance_dir(&app.settings, name);
    let manifest_path = config::InstanceManifest::manifest_path(&dir);
    if let Ok(mut manifest) = config::InstanceManifest::load(&manifest_path) {
        if app.init_config_panel == 0 {
            manifest.init_lua_pre = value;
        } else {
            manifest.init_lua_post = value;
        }
        manifest.updated_at = chrono::Utc::now();
        if let Err(e) = manifest.save(&manifest_path) {
            app.message = Some(format!("Failed to save: {e}"));
            return;
        }
        // Regenerate init.lua
        let data_dir = dir.join("data");
        let init_lua = crate::plugins::generate_init_lua_full(
            &data_dir,
            &app.registry,
            &manifest.workloads,
            &manifest.disabled_features,
            &manifest.extra_features,
            &manifest.leader_key,
            &manifest.mason_packages,
            manifest.init_lua_pre.as_deref(),
            manifest.init_lua_post.as_deref(),
        );
        let init_lua_path = dir.join("config").join("nvim").join("init.lua");
        if let Err(e) = std::fs::write(&init_lua_path, init_lua) {
            app.message = Some(format!("Failed to write init.lua: {e}"));
            return;
        }
        // Update cached instance
        if let Some(inst) = app.instances.iter_mut().find(|i| i.name == name) {
            inst.init_lua_pre = manifest.init_lua_pre;
            inst.init_lua_post = manifest.init_lua_post;
        }
        let label = if app.init_config_panel == 0 { "pre-plugins" } else { "post-plugins" };
        app.message = Some(format!("Saved {label} overrides."));
    }
}

fn reset_init_config_defaults(app: &mut App, name: &str) {
    let dir = config::instance_dir(&app.settings, name);
    let manifest_path = config::InstanceManifest::manifest_path(&dir);
    if let Ok(mut manifest) = config::InstanceManifest::load(&manifest_path) {
        manifest.init_lua_pre = crate::plugins::generate_default_pre(&manifest.workloads);
        manifest.init_lua_post = crate::plugins::generate_default_post(&manifest.workloads);
        manifest.updated_at = chrono::Utc::now();
        if let Err(e) = manifest.save(&manifest_path) {
            app.message = Some(format!("Failed to save: {e}"));
            return;
        }
        let data_dir = dir.join("data");
        let init_lua = crate::plugins::generate_init_lua_full(
            &data_dir,
            &app.registry,
            &manifest.workloads,
            &manifest.disabled_features,
            &manifest.extra_features,
            &manifest.leader_key,
            &manifest.mason_packages,
            manifest.init_lua_pre.as_deref(),
            manifest.init_lua_post.as_deref(),
        );
        let init_lua_path = dir.join("config").join("nvim").join("init.lua");
        if let Err(e) = std::fs::write(&init_lua_path, init_lua) {
            app.message = Some(format!("Failed to write init.lua: {e}"));
            return;
        }
        if let Some(inst) = app.instances.iter_mut().find(|i| i.name == name) {
            inst.init_lua_pre = manifest.init_lua_pre;
            inst.init_lua_post = manifest.init_lua_post;
        }
        app.message = Some("Reset init overrides to smart defaults.".to_string());
    }
}
