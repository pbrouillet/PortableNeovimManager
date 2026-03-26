use crossterm::event::{KeyCode, KeyModifiers, MouseEvent, MouseEventKind, MouseButton};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::definitions::Menu;
use crate::tui::command::Command;

// ── Menu bar state ──────────────────────────────────────────────────────────

pub struct MenuBarState {
    pub is_active: bool,
    pub current_menu: usize,
    pub current_item: usize,
    pub is_dropped: bool,
}

impl Default for MenuBarState {
    fn default() -> Self {
        Self {
            is_active: false,
            current_menu: 0,
            current_item: 0,
            is_dropped: false,
        }
    }
}

// ── Menu bar widget ─────────────────────────────────────────────────────────

pub struct MenuBar {
    pub menus: Vec<Menu>,
    pub state: MenuBarState,
    /// Cached positions of menu labels on the bar (for mouse hit testing).
    menu_rects: Vec<Rect>,
    /// Cached positions of dropdown items (for mouse hit testing).
    item_rects: Vec<Rect>,
    /// Area where the bar was last drawn.
    bar_area: Rect,
}

impl MenuBar {
    pub fn new(menus: Vec<Menu>) -> Self {
        Self {
            menus,
            state: MenuBarState::default(),
            menu_rects: Vec::new(),
            item_rects: Vec::new(),
            bar_area: Rect::default(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.state.is_active
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        self.bar_area = area;
        self.menu_rects.clear();

        let mut spans = Vec::new();
        let mut x_offset: u16 = 1; // 1 for left padding

        for (i, menu) in self.menus.iter().enumerate() {
            let is_selected = self.state.is_active && self.state.current_menu == i;

            // Build the label with underlined hotkey
            let label = &menu.label;
            let hotkey_pos = label
                .chars()
                .position(|c| c.to_ascii_uppercase() == menu.hotkey.to_ascii_uppercase());

            let style_normal = if is_selected {
                Style::default().bg(Color::White).fg(Color::Black)
            } else if self.state.is_active {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };
            let style_hotkey = style_normal.add_modifier(Modifier::UNDERLINED);

            // Space before label
            spans.push(Span::styled(" ", style_normal));

            if let Some(pos) = hotkey_pos {
                for (ci, ch) in label.chars().enumerate() {
                    if ci == pos {
                        spans.push(Span::styled(ch.to_string(), style_hotkey));
                    } else {
                        spans.push(Span::styled(ch.to_string(), style_normal));
                    }
                }
            } else {
                spans.push(Span::styled(label.as_str(), style_normal));
            }

            // Space after label
            spans.push(Span::styled(" ", style_normal));

            let label_width = label.len() as u16 + 2; // +2 for padding spaces
            self.menu_rects.push(Rect::new(
                area.x + x_offset,
                area.y,
                label_width,
                1,
            ));
            x_offset += label_width;
        }

        // Fill remaining width
        let remaining = area.width.saturating_sub(x_offset + 1);
        if remaining > 0 {
            spans.push(Span::styled(
                " ".repeat(remaining as usize),
                Style::default().bg(Color::DarkGray),
            ));
        }

        let bar = Paragraph::new(Line::from(spans))
            .style(Style::default().bg(Color::DarkGray));
        frame.render_widget(bar, area);

        // Draw dropdown if active
        if self.state.is_active && self.state.is_dropped {
            self.draw_dropdown(frame, area);
        }
    }

    fn draw_dropdown(&mut self, frame: &mut Frame, bar_area: Rect) {
        self.item_rects.clear();

        let menu_idx = self.state.current_menu;
        let Some(menu) = self.menus.get(menu_idx) else { return };
        let Some(menu_rect) = self.menu_rects.get(menu_idx) else { return };

        // Calculate dropdown dimensions
        let mut max_label_width: u16 = 0;
        let mut max_shortcut_width: u16 = 0;
        let mut item_count: u16 = 0;
        for item in &menu.items {
            match item {
                super::definitions::MenuItem::Action { label, shortcut_display, .. } => {
                    max_label_width = max_label_width.max(label.len() as u16);
                    if let Some(sc) = shortcut_display {
                        max_shortcut_width = max_shortcut_width.max(sc.len() as u16);
                    }
                    item_count += 1;
                }
                super::definitions::MenuItem::Separator => {
                    item_count += 1;
                }
            }
        }

        let gap = if max_shortcut_width > 0 { 2 } else { 0 };
        let inner_width = max_label_width + gap + max_shortcut_width;
        let dropdown_width = inner_width + 4; // 2 border + 2 padding
        let dropdown_height = item_count + 2; // 2 for border

        let dropdown_x = menu_rect.x.min(
            bar_area.x + bar_area.width.saturating_sub(dropdown_width),
        );
        let dropdown_y = bar_area.y + 1;

        let dropdown_area = Rect::new(
            dropdown_x,
            dropdown_y,
            dropdown_width.min(bar_area.width),
            dropdown_height.min(frame.area().height.saturating_sub(dropdown_y)),
        );

        // Clear the area and draw border
        frame.render_widget(Clear, dropdown_area);
        let border = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
            .style(Style::default().bg(Color::DarkGray));
        frame.render_widget(border, dropdown_area);

        // Draw items
        let inner = Rect::new(
            dropdown_area.x + 1,
            dropdown_area.y + 1,
            dropdown_area.width.saturating_sub(2),
            dropdown_area.height.saturating_sub(2),
        );

        let mut visible_idx: usize = 0;
        for (i, item) in menu.items.iter().enumerate() {
            if visible_idx as u16 >= inner.height {
                break;
            }

            let item_y = inner.y + visible_idx as u16;
            let item_rect = Rect::new(inner.x, item_y, inner.width, 1);

            match item {
                super::definitions::MenuItem::Action { label, hotkey, shortcut_display, .. } => {
                    let is_selected = self.state.current_item == i;
                    let style = if is_selected {
                        Style::default().bg(Color::White).fg(Color::Black)
                    } else {
                        Style::default().bg(Color::DarkGray).fg(Color::White)
                    };
                    let style_hotkey = if is_selected {
                        style
                    } else {
                        Style::default().bg(Color::DarkGray).fg(Color::Yellow)
                    };

                    let mut spans = Vec::new();
                    spans.push(Span::styled(" ", style));

                    // Label with hotkey underlined
                    let hotkey_pos = label
                        .chars()
                        .position(|c| c.to_ascii_uppercase() == hotkey.to_ascii_uppercase());
                    if let Some(pos) = hotkey_pos {
                        for (ci, ch) in label.chars().enumerate() {
                            if ci == pos {
                                spans.push(Span::styled(
                                    ch.to_string(),
                                    style_hotkey.add_modifier(Modifier::UNDERLINED),
                                ));
                            } else {
                                spans.push(Span::styled(ch.to_string(), style));
                            }
                        }
                    } else {
                        spans.push(Span::styled(label.as_str(), style));
                    }

                    // Pad to align shortcuts
                    let label_len = label.len() as u16;
                    let pad = inner_width.saturating_sub(label_len + 1)
                        .saturating_sub(shortcut_display.as_ref().map_or(0, |s| s.len() as u16));
                    if pad > 0 {
                        spans.push(Span::styled(" ".repeat(pad as usize), style));
                    }

                    // Shortcut display
                    if let Some(sc) = shortcut_display {
                        spans.push(Span::styled(
                            sc.as_str(),
                            if is_selected {
                                style
                            } else {
                                Style::default().bg(Color::DarkGray).fg(Color::DarkGray)
                            },
                        ));
                    }

                    // Fill remaining
                    spans.push(Span::styled(" ", style));

                    self.item_rects.push(item_rect);
                    frame.render_widget(Paragraph::new(Line::from(spans)), item_rect);
                }
                super::definitions::MenuItem::Separator => {
                    let sep = "─".repeat(inner.width as usize);
                    frame.render_widget(
                        Paragraph::new(sep).style(Style::default().fg(Color::Gray).bg(Color::DarkGray)),
                        item_rect,
                    );
                    self.item_rects.push(Rect::default()); // placeholder
                }
            }
            visible_idx += 1;
        }
    }

    // ── Input handling ──────────────────────────────────────────────────────

    /// Handle a key event. Returns Some(Command) if a menu action was triggered.
    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Option<Command> {
        // Alt+letter opens a menu
        if modifiers.contains(KeyModifiers::ALT) {
            if let KeyCode::Char(ch) = code {
                let ch_upper = ch.to_ascii_uppercase();
                for (i, menu) in self.menus.iter().enumerate() {
                    if menu.hotkey.to_ascii_uppercase() == ch_upper {
                        self.state.is_active = true;
                        self.state.current_menu = i;
                        self.state.current_item = self.first_action_index(i);
                        self.state.is_dropped = true;
                        return None;
                    }
                }
            }
        }

        if !self.state.is_active {
            return None;
        }

        match code {
            KeyCode::Esc => {
                self.deactivate();
                None
            }
            KeyCode::Left => {
                self.move_menu(-1);
                None
            }
            KeyCode::Right => {
                self.move_menu(1);
                None
            }
            KeyCode::Up => {
                if self.state.is_dropped {
                    self.move_item(-1);
                }
                None
            }
            KeyCode::Down => {
                if !self.state.is_dropped {
                    self.state.is_dropped = true;
                    self.state.current_item = self.first_action_index(self.state.current_menu);
                } else {
                    self.move_item(1);
                }
                None
            }
            KeyCode::Enter => {
                if self.state.is_dropped {
                    let cmd = self.current_command();
                    self.deactivate();
                    cmd
                } else {
                    self.state.is_dropped = true;
                    self.state.current_item = self.first_action_index(self.state.current_menu);
                    None
                }
            }
            KeyCode::Char(ch) if !modifiers.contains(KeyModifiers::CONTROL) => {
                // Hotkey within dropdown
                if self.state.is_dropped {
                    let ch_upper = ch.to_ascii_uppercase();
                    if let Some(menu) = self.menus.get(self.state.current_menu) {
                        for (i, item) in menu.items.iter().enumerate() {
                            if let super::definitions::MenuItem::Action { hotkey, .. } = item {
                                if hotkey.to_ascii_uppercase() == ch_upper {
                                    self.state.current_item = i;
                                    let cmd = self.current_command();
                                    self.deactivate();
                                    return cmd;
                                }
                            }
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Handle a mouse event. Returns Some(Command) if a menu action was triggered.
    pub fn handle_mouse(&mut self, event: MouseEvent) -> Option<Command> {
        let x = event.column;
        let y = event.row;

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check menu bar labels
                for (i, rect) in self.menu_rects.iter().enumerate() {
                    if rect.contains(ratatui::layout::Position::new(x, y)) {
                        if self.state.is_active && self.state.current_menu == i && self.state.is_dropped {
                            self.deactivate();
                        } else {
                            self.state.is_active = true;
                            self.state.current_menu = i;
                            self.state.current_item = self.first_action_index(i);
                            self.state.is_dropped = true;
                        }
                        return None;
                    }
                }

                // Check dropdown items
                if self.state.is_active && self.state.is_dropped {
                    for (i, rect) in self.item_rects.iter().enumerate() {
                        if !rect.is_empty() && rect.contains(ratatui::layout::Position::new(x, y)) {
                            // Map item_rects index back to menu item index
                            if let Some(menu) = self.menus.get(self.state.current_menu) {
                                let mut visible = 0;
                                for (mi, item) in menu.items.iter().enumerate() {
                                    if visible == i {
                                        if let super::definitions::MenuItem::Action { command, .. } = item {
                                            let cmd = *command;
                                            self.deactivate();
                                            return Some(cmd);
                                        }
                                        break;
                                    }
                                    visible += 1;
                                    if visible > menu.items.len() { break; }
                                    let _ = mi;
                                }
                            }
                        }
                    }
                    // Click outside dropdown closes it
                    self.deactivate();
                }
                None
            }
            _ => None,
        }
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    pub fn deactivate(&mut self) {
        self.state.is_active = false;
        self.state.is_dropped = false;
    }

    fn move_menu(&mut self, delta: i32) {
        let len = self.menus.len();
        if len == 0 { return; }
        let new = (self.state.current_menu as i32 + delta).rem_euclid(len as i32) as usize;
        self.state.current_menu = new;
        self.state.current_item = self.first_action_index(new);
    }

    fn move_item(&mut self, delta: i32) {
        let Some(menu) = self.menus.get(self.state.current_menu) else { return };
        let len = menu.items.len();
        if len == 0 { return; }

        let mut idx = self.state.current_item as i32;
        loop {
            idx = (idx + delta).rem_euclid(len as i32);
            if matches!(menu.items[idx as usize], super::definitions::MenuItem::Action { .. }) {
                break;
            }
            // Prevent infinite loop if all items are separators
            if idx as usize == self.state.current_item { break; }
        }
        self.state.current_item = idx as usize;
    }

    fn first_action_index(&self, menu_idx: usize) -> usize {
        if let Some(menu) = self.menus.get(menu_idx) {
            for (i, item) in menu.items.iter().enumerate() {
                if matches!(item, super::definitions::MenuItem::Action { .. }) {
                    return i;
                }
            }
        }
        0
    }

    fn current_command(&self) -> Option<Command> {
        let menu = self.menus.get(self.state.current_menu)?;
        let item = menu.items.get(self.state.current_item)?;
        match item {
            super::definitions::MenuItem::Action { command, .. } => Some(*command),
            _ => None,
        }
    }
}
