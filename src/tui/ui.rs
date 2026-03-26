use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
};

use crate::tui::state::LayoutCache;

use super::app::App;
use super::menu::function_bar::FunctionBar;
use super::state::Screen;

const MIN_WIDTH: u16 = 40;
const MIN_HEIGHT: u16 = 10;

/// Compute per-row Rects for a bordered Table with a 1-row header and populate the layout cache.
pub(crate) fn populate_table_cache(cache: &mut LayoutCache, table_area: Rect, num_items: usize) {
    // Table has a 1px border on each side and a 1-row header
    let inner_y = table_area.y + 1; // top border
    let inner_height = table_area.height.saturating_sub(2); // top + bottom border
    if inner_height <= 1 { return; } // no room for data rows
    let data_y = inner_y + 1; // skip header row
    let data_height = inner_height - 1;
    let inner_width = table_area.width.saturating_sub(2);

    for i in 0..num_items.min(data_height as usize) {
        cache.list_items.push((
            Rect::new(table_area.x + 1, data_y + i as u16, inner_width, 1),
            i,
        ));
    }
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let msg = Paragraph::new(format!(
            "Terminal too small ({}×{})\nMinimum: {}×{}",
            area.width, area.height, MIN_WIDTH, MIN_HEIGHT
        ))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Red));
        frame.render_widget(msg, area);
        return;
    }

    // Reset layout cache for this frame
    app.layout_cache.reset();

    // Split into 3 vertical areas: menu bar (1), content (flex), function bar (1)
    let chrome = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    // Draw menu bar at top
    app.menu_bar.draw(frame, chrome[0]);

    // Draw function bar at bottom
    let mut fn_bar = FunctionBar::for_screen(&app.screen);
    fn_bar.draw(frame, chrome[2]);

    // Draw screen content in the middle area
    let content = chrome[1];
    let screen = app.screen.clone();
    match &screen {
        Screen::InstanceList => draw_instance_list(frame, app, content),
        Screen::InstanceDetail { name } => draw_instance_detail(frame, app, name, content),
        Screen::EditFeatures { name } => draw_edit_features(frame, app, name, content),
        Screen::EditLeaderKey { name } => draw_edit_leader(frame, app, name, content),
        Screen::ConfirmDelete { name } => draw_confirm_delete(frame, name, content),
        Screen::EditSettings => draw_edit_settings(frame, app, content),
        Screen::TutorialList => draw_tutorial_list(frame, app, content),
        Screen::TutorialView { title, content: body, .. } => draw_tutorial_view(frame, app, title, body, content),
        Screen::Marketplace { instance_name } => draw_marketplace(frame, app, instance_name, content),
        Screen::CreateInstance => draw_create_instance(frame, app, content),
        Screen::Monitor { name } => draw_monitor(frame, app, name, content),
        Screen::InitConfig { name } => draw_init_config(frame, app, name, content),
        Screen::ConfigureTerminalFont => draw_configure_terminal_font(frame, app, content),
    }
}

fn draw_instance_list(frame: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    super::screens::instance_list::draw(frame, app, area);
}

fn draw_instance_detail(frame: &mut Frame, app: &mut App, name: &str, area: ratatui::layout::Rect) {
    super::screens::instance_detail::draw(frame, app, name, area);
}

fn draw_edit_features(frame: &mut Frame, app: &mut App, name: &str, area: ratatui::layout::Rect) {
    super::screens::edit_features::draw(frame, app, name, area);
}

fn draw_edit_leader(frame: &mut Frame, app: &mut App, name: &str, area: ratatui::layout::Rect) {
    super::screens::edit_leader::draw(frame, app, name, area);
}

// ── Edit Settings ───────────────────────────────────────────────────────────

fn draw_edit_settings(frame: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    super::screens::settings::draw(frame, app, area);
}

// ── Confirm Delete ──────────────────────────────────────────────────────────

fn draw_confirm_delete(frame: &mut Frame, name: &str, area: ratatui::layout::Rect) {
    super::screens::confirm_delete::draw(frame, name, area);
}

// ── Tutorial List ───────────────────────────────────────────────────────────

fn draw_tutorial_list(frame: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    super::screens::tutorial::draw_list(frame, app, area);
}

// ── Tutorial View ───────────────────────────────────────────────────────────

fn draw_tutorial_view(frame: &mut Frame, app: &mut App, title: &str, content: &str, area: ratatui::layout::Rect) {
    super::screens::tutorial::draw_view(frame, app, title, content, area);
}

// ── Create Instance ─────────────────────────────────────────────────────────

fn draw_create_instance(frame: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    super::screens::create::draw(frame, app, area);
}

fn draw_marketplace(frame: &mut Frame, app: &mut App, instance_name: &str, area: ratatui::layout::Rect) {
    super::screens::marketplace::draw(frame, app, instance_name, area);
}

fn draw_monitor(frame: &mut Frame, app: &mut App, name: &str, area: ratatui::layout::Rect) {
    super::screens::monitor::draw(frame, app, name, area);
}

fn draw_init_config(frame: &mut Frame, app: &mut App, name: &str, area: ratatui::layout::Rect) {
    super::screens::init_config::draw(frame, app, name, area);
}

fn draw_configure_terminal_font(frame: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    super::screens::terminal_font::draw(frame, app, area);
}