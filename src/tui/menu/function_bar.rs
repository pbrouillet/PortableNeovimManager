use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::command::Command;
use crate::tui::state::Screen;

// ── Function bar entry ──────────────────────────────────────────────────────

pub struct FnBarEntry {
    pub key_label: &'static str,
    pub display_label: &'static str,
    pub command: Command,
}

// ── Function bar ────────────────────────────────────────────────────────────

pub struct FunctionBar {
    entries: Vec<FnBarEntry>,
    /// Cached positions for mouse hit testing.
    entry_rects: Vec<Rect>,
}

impl FunctionBar {
    pub fn for_screen(screen: &Screen) -> Self {
        let entries = match screen {
            Screen::InstanceList => vec![
                FnBarEntry { key_label: "F1", display_label: "Help", command: Command::OpenTutorials },
                FnBarEntry { key_label: "F2", display_label: "Launch", command: Command::LaunchInstance },
                FnBarEntry { key_label: "F5", display_label: "Create", command: Command::CreateInstance },
                FnBarEntry { key_label: "F6", display_label: "Feature", command: Command::EditFeatures },
                FnBarEntry { key_label: "F8", display_label: "Delete", command: Command::DeleteInstance },
                FnBarEntry { key_label: "F10", display_label: "Menu", command: Command::ActivateMenuBar },
            ],
            Screen::InstanceDetail { .. } => vec![
                FnBarEntry { key_label: "F1", display_label: "Help", command: Command::OpenTutorials },
                FnBarEntry { key_label: "F2", display_label: "Launch", command: Command::LaunchInstance },
                FnBarEntry { key_label: "F6", display_label: "Feature", command: Command::EditFeatures },
                FnBarEntry { key_label: "F8", display_label: "Delete", command: Command::DeleteInstance },
                FnBarEntry { key_label: "F10", display_label: "Menu", command: Command::ActivateMenuBar },
            ],
            Screen::EditFeatures { .. } => vec![
                FnBarEntry { key_label: "F1", display_label: "Help", command: Command::OpenTutorials },
                FnBarEntry { key_label: "F2", display_label: "Save", command: Command::Save },
                FnBarEntry { key_label: "Esc", display_label: "Back", command: Command::Back },
                FnBarEntry { key_label: "F10", display_label: "Menu", command: Command::ActivateMenuBar },
            ],
            Screen::Marketplace { .. } => vec![
                FnBarEntry { key_label: "F1", display_label: "Help", command: Command::OpenTutorials },
                FnBarEntry { key_label: "F2", display_label: "Apply", command: Command::Save },
                FnBarEntry { key_label: "/", display_label: "Search", command: Command::Search },
                FnBarEntry { key_label: "F10", display_label: "Menu", command: Command::ActivateMenuBar },
            ],
            // Default minimal bar for all other screens
            _ => vec![
                FnBarEntry { key_label: "F1", display_label: "Help", command: Command::OpenTutorials },
                FnBarEntry { key_label: "Esc", display_label: "Back", command: Command::Back },
                FnBarEntry { key_label: "F10", display_label: "Menu", command: Command::ActivateMenuBar },
            ],
        };
        Self {
            entries,
            entry_rects: Vec::new(),
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        self.entry_rects.clear();

        let mut spans = Vec::new();
        let mut x_offset: u16 = 0;

        for (i, entry) in self.entries.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("│", Style::default().fg(Color::DarkGray).bg(Color::Black)));
                x_offset += 1;
            }

            let key_span = Span::styled(
                entry.key_label,
                Style::default().fg(Color::Black).bg(Color::Gray).add_modifier(Modifier::BOLD),
            );
            let label_span = Span::styled(
                entry.display_label,
                Style::default().fg(Color::White).bg(Color::Black),
            );

            let entry_width = entry.key_label.len() as u16 + entry.display_label.len() as u16;
            self.entry_rects.push(Rect::new(
                area.x + x_offset,
                area.y,
                entry_width,
                1,
            ));

            spans.push(key_span);
            spans.push(label_span);
            x_offset += entry_width;
        }

        // Fill remaining
        let remaining = area.width.saturating_sub(x_offset);
        if remaining > 0 {
            spans.push(Span::styled(
                " ".repeat(remaining as usize),
                Style::default().bg(Color::Black),
            ));
        }

        let bar = Paragraph::new(Line::from(spans));
        frame.render_widget(bar, area);
    }

    /// Handle a mouse click. Returns Some(Command) if an F-key entry was clicked.
    pub fn handle_mouse(&self, event: MouseEvent) -> Option<Command> {
        if !matches!(event.kind, MouseEventKind::Down(MouseButton::Left)) {
            return None;
        }
        let pos = ratatui::layout::Position::new(event.column, event.row);
        for (i, rect) in self.entry_rects.iter().enumerate() {
            if rect.contains(pos) {
                return Some(self.entries[i].command);
            }
        }
        None
    }
}
