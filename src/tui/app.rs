use std::io::{self, stdout};
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::config::{self, GlobalSettings, InstanceManifest, LEADER_KEY_OPTIONS};
use crate::workload::WorkloadRegistry;

use super::ui;

// ── Screen ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum Screen {
    InstanceList,
    InstanceDetail { name: String },
    EditFeatures { name: String },
    EditLeaderKey { name: String },
    TutorialList,
    TutorialView { title: String, content: String },
}

// ── App state ───────────────────────────────────────────────────────────────

pub struct App {
    pub instances: Vec<InstanceManifest>,
    pub selected: usize,
    pub screen: Screen,
    pub message: Option<String>,
    pub should_quit: bool,
    /// Checkbox state for the EditFeatures screen: (workload_id, enabled)
    pub feature_checkboxes: Vec<(String, bool)>,
    /// Cursor position in the feature checkbox list
    pub feature_cursor: usize,
    /// Cursor position in the leader key selection list
    pub leader_cursor: usize,
    /// Workload definitions loaded from workloads.json
    pub registry: WorkloadRegistry,
    /// Global application settings
    pub settings: GlobalSettings,
    /// Cursor position in the tutorial list
    pub tutorial_cursor: usize,
    /// Cached tutorial topics list: (id, title)
    pub tutorial_topics: Vec<(String, String)>,
    /// Filtered tutorial topics for display (indices into tutorial_topics)
    pub tutorial_filtered: Vec<usize>,
    /// Scroll offset for tutorial view
    pub tutorial_scroll: usize,
    /// Screen to return to when leaving tutorial screens
    pub tutorial_return_screen: Option<Box<Screen>>,
    /// Search query for tutorial list filtering
    pub tutorial_search: String,
    /// Whether the search input is active
    pub tutorial_search_active: bool,
}

impl App {
    fn new(instances: Vec<InstanceManifest>, registry: WorkloadRegistry, settings: GlobalSettings) -> Self {
        let tutorial_topics = registry.all_tutorial_topics();
        let tutorial_filtered = (0..tutorial_topics.len()).collect();
        Self {
            instances,
            selected: 0,
            screen: Screen::InstanceList,
            message: None,
            should_quit: false,
            feature_checkboxes: Vec::new(),
            feature_cursor: 0,
            leader_cursor: 0,
            registry,
            settings,
            tutorial_cursor: 0,
            tutorial_topics,
            tutorial_filtered,
            tutorial_scroll: 0,
            tutorial_return_screen: None,
            tutorial_search: String::new(),
            tutorial_search_active: false,
        }
    }

    fn refresh_instances(&mut self) {
        self.instances = load_instances(&self.settings);
        if self.selected >= self.instances.len() && !self.instances.is_empty() {
            self.selected = self.instances.len() - 1;
        }
        if self.instances.is_empty() {
            self.selected = 0;
        }
    }

    fn selected_name(&self) -> Option<String> {
        self.instances.get(self.selected).map(|i| i.name.clone())
    }

    fn open_tutorial_list(&mut self, return_to: Screen) {
        self.tutorial_return_screen = Some(Box::new(return_to));
        self.tutorial_cursor = 0;
        self.tutorial_search.clear();
        self.tutorial_search_active = false;
        self.update_tutorial_filter();
        self.screen = Screen::TutorialList;
        self.message = None;
    }

    fn open_tutorial_view(&mut self, title: String, content: String, return_to: Screen) {
        self.tutorial_return_screen = Some(Box::new(return_to));
        self.tutorial_scroll = 0;
        self.screen = Screen::TutorialView { title, content };
    }

    fn tutorial_return(&mut self) {
        if let Some(screen) = self.tutorial_return_screen.take() {
            self.screen = *screen;
        } else {
            self.screen = Screen::InstanceList;
        }
        self.message = None;
    }

    fn update_tutorial_filter(&mut self) {
        let query = self.tutorial_search.to_lowercase();
        if query.is_empty() {
            self.tutorial_filtered = (0..self.tutorial_topics.len()).collect();
        } else {
            self.tutorial_filtered = self
                .tutorial_topics
                .iter()
                .enumerate()
                .filter(|(_, (id, title))| {
                    id.to_lowercase().contains(&query)
                        || title.to_lowercase().contains(&query)
                })
                .map(|(i, _)| i)
                .collect();
        }
        if self.tutorial_cursor >= self.tutorial_filtered.len() {
            self.tutorial_cursor = 0;
        }
    }

    fn enter_edit_features(&mut self, name: &str) {
        let instance = self.instances.iter().find(|i| i.name == name);
        let current_features = instance.map(|i| &i.features);

        self.feature_checkboxes = self
            .registry
            .optional()
            .iter()
            .map(|w| {
                let enabled = current_features
                    .map(|feats| feats.contains(&w.id))
                    .unwrap_or(false);
                (w.id.clone(), enabled)
            })
            .collect();
        self.feature_cursor = 0;
        self.screen = Screen::EditFeatures {
            name: name.to_string(),
        };
        self.message = None;
    }

    fn toggle_feature(&mut self) {
        if let Some((workload_id, enabled)) =
            self.feature_checkboxes.get(self.feature_cursor).cloned()
        {
            let new_state = !enabled;

            if new_state {
                // Turning ON: also enable dependencies
                self.feature_checkboxes[self.feature_cursor].1 = true;
                if let Some(workload) = self.registry.find_by_id(&workload_id) {
                    for dep_id in &workload.depends_on {
                        if let Some(pos) = self
                            .feature_checkboxes
                            .iter()
                            .position(|(id, _)| id == dep_id)
                        {
                            self.feature_checkboxes[pos].1 = true;
                        }
                    }
                }
            } else {
                // Turning OFF: also disable dependents
                self.feature_checkboxes[self.feature_cursor].1 = false;
                let dependents = self.registry.dependents_of(&workload_id);
                for dep_id in &dependents {
                    if let Some(pos) = self
                        .feature_checkboxes
                        .iter()
                        .position(|(id, _)| id == dep_id)
                    {
                        self.feature_checkboxes[pos].1 = false;
                    }
                }
            }
        }
    }

    fn apply_features(&mut self, name: &str) {
        let features: Vec<String> = self
            .feature_checkboxes
            .iter()
            .filter(|(_, enabled)| *enabled)
            .map(|(id, _)| id.clone())
            .collect();

        match crate::instance::update_features(name, features, &self.registry, &self.settings) {
            Ok(()) => {
                self.refresh_instances();
                self.message = Some(format!("Features updated for '{name}'."));
            }
            Err(e) => {
                self.message = Some(format!("Failed to update features: {e}"));
            }
        }
    }

    fn enter_edit_leader(&mut self, name: &str) {
        let instance = self.instances.iter().find(|i| i.name == name);
        let current_key = instance.map(|i| i.leader_key.as_str()).unwrap_or(" ");

        // Find the index of the current leader key in options
        self.leader_cursor = LEADER_KEY_OPTIONS
            .iter()
            .position(|(v, _)| *v == current_key)
            .unwrap_or(0);
        self.screen = Screen::EditLeaderKey {
            name: name.to_string(),
        };
        self.message = None;
    }

    fn apply_leader_key(&mut self, name: &str) {
        let Some((key_value, _)) = LEADER_KEY_OPTIONS.get(self.leader_cursor) else {
            self.message = Some("Invalid leader key selection.".to_string());
            return;
        };

        let base = config::instance_dir(&self.settings, name);
        let manifest_path = InstanceManifest::manifest_path(&base);

        match InstanceManifest::load(&manifest_path) {
            Ok(mut manifest) => {
                manifest.leader_key = key_value.to_string();
                manifest.updated_at = chrono::Utc::now();

                // Regenerate init.lua with updated leader key
                let data_dir = base.join("data");
                let init_lua = crate::plugins::generate_init_lua(
                    &data_dir,
                    &self.registry,
                    &manifest.features,
                    &manifest.leader_key,
                );
                let init_lua_path = base.join("config").join("nvim").join("init.lua");

                if let Err(e) = std::fs::write(&init_lua_path, init_lua) {
                    self.message = Some(format!("Failed to write init.lua: {e}"));
                    return;
                }
                if let Err(e) = manifest.save(&manifest_path) {
                    self.message = Some(format!("Failed to save manifest: {e}"));
                    return;
                }

                self.refresh_instances();
                self.message = Some(format!(
                    "Leader key set to '{}' for '{name}'.",
                    config::leader_key_display(key_value)
                ));
            }
            Err(e) => {
                self.message = Some(format!("Failed to load manifest: {e}"));
            }
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn load_instances(settings: &GlobalSettings) -> Vec<InstanceManifest> {
    let dir = config::instances_dir(settings);
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = InstanceManifest::manifest_path(&path);
                if let Ok(m) = InstanceManifest::load(&manifest_path) {
                    out.push(m);
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Leave the alternate screen so an external command can use the terminal,
/// then re-enter afterwards.
fn leave_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();
}

fn enter_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    let _ = enable_raw_mode();
    let _ = execute!(stdout(), EnterAlternateScreen);
    let _ = terminal.hide_cursor();
    let _ = terminal.clear();
}

// ── Main loop ───────────────────────────────────────────────────────────────

pub async fn run(settings: GlobalSettings) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let instances = load_instances(&settings);
    let registry = crate::workload::load_workloads();
    let mut app = App::new(instances, registry, settings);

    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match &app.screen {
                    Screen::InstanceList => {
                        handle_list_keys(&mut app, key.code, &mut terminal).await;
                    }
                    Screen::InstanceDetail { name } => {
                        let name = name.clone();
                        handle_detail_keys(&mut app, key.code, &name, &mut terminal).await;
                    }
                    Screen::EditFeatures { name } => {
                        let name = name.clone();
                        handle_features_keys(&mut app, key.code, &name);
                    }
                    Screen::EditLeaderKey { name } => {
                        let name = name.clone();
                        handle_leader_keys(&mut app, key.code, &name);
                    }
                    Screen::TutorialList => {
                        handle_tutorial_list_keys(&mut app, key.code);
                    }
                    Screen::TutorialView { .. } => {
                        handle_tutorial_view_keys(&mut app, key.code);
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

// ── Key handlers ────────────────────────────────────────────────────────────

async fn handle_list_keys(
    app: &mut App,
    code: KeyCode,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.instances.is_empty() {
                app.selected = (app.selected + 1) % app.instances.len();
            }
            app.message = None;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !app.instances.is_empty() {
                app.selected = if app.selected == 0 {
                    app.instances.len() - 1
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
            app.message = Some("Use CLI: pnm create <name>".to_string());
        }
        KeyCode::Char('r') => {
            app.refresh_instances();
            app.message = Some("Refreshed instance list.".to_string());
        }
        KeyCode::Char('l') => {
            if let Some(name) = app.selected_name() {
                do_launch(&name, app, terminal);
            }
        }
        KeyCode::Char('u') => {
            if let Some(name) = app.selected_name() {
                do_update(&name, app, terminal).await;
            }
        }
        KeyCode::Char('d') => {
            if let Some(name) = app.selected_name() {
                do_delete(&name, app, terminal);
            }
        }
        KeyCode::Char('f') => {
            if let Some(name) = app.selected_name() {
                app.enter_edit_features(&name);
            }
        }
        KeyCode::Char('m') => {
            if let Some(name) = app.selected_name() {
                app.enter_edit_leader(&name);
            }
        }
        KeyCode::Char('o') => {
            if let Some(name) = app.selected_name() {
                open_instance_dir(&name, app);
            }
        }
        KeyCode::Char('n') => {
            do_install_font(app, terminal).await;
        }
        KeyCode::Char('s') => {
            do_init_settings(app);
        }
        KeyCode::Char('t') => {
            app.open_tutorial_list(Screen::InstanceList);
        }
        _ => {}
    }
}

async fn handle_detail_keys(
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
            do_launch(name, app, terminal);
        }
        KeyCode::Char('u') => {
            do_update(name, app, terminal).await;
        }
        KeyCode::Char('d') => {
            do_delete(name, app, terminal);
            // If deleted, go back to the list.
            if !app.instances.iter().any(|i| i.name == name) {
                app.screen = Screen::InstanceList;
            }
        }
        KeyCode::Char('f') => {
            app.enter_edit_features(name);
        }
        KeyCode::Char('m') => {
            app.enter_edit_leader(name);
        }
        KeyCode::Char('o') => {
            open_instance_dir(name, app);
        }
        KeyCode::Char('t') => {
            app.open_tutorial_list(Screen::InstanceDetail { name: name.to_string() });
        }
        _ => {}
    }
}

fn handle_features_keys(app: &mut App, code: KeyCode, name: &str) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.screen = Screen::InstanceDetail {
                name: name.to_string(),
            };
            app.message = None;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.feature_checkboxes.is_empty() {
                app.feature_cursor = (app.feature_cursor + 1) % app.feature_checkboxes.len();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !app.feature_checkboxes.is_empty() {
                app.feature_cursor = if app.feature_cursor == 0 {
                    app.feature_checkboxes.len() - 1
                } else {
                    app.feature_cursor - 1
                };
            }
        }
        KeyCode::Char(' ') => {
            app.toggle_feature();
        }
        KeyCode::Enter => {
            app.apply_features(name);
            app.screen = Screen::InstanceDetail {
                name: name.to_string(),
            };
        }
        KeyCode::Char('t') => {
            if let Some((workload_id, _)) = app.feature_checkboxes.get(app.feature_cursor) {
                if let Some((title, content)) = app.registry.tutorial_content(workload_id) {
                    let return_to = Screen::EditFeatures { name: name.to_string() };
                    app.open_tutorial_view(title, content, return_to);
                } else {
                    app.message = Some("No tutorial available for this feature.".to_string());
                }
            }
        }
        _ => {}
    }
}

fn handle_leader_keys(app: &mut App, code: KeyCode, name: &str) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.screen = Screen::InstanceDetail {
                name: name.to_string(),
            };
            app.message = None;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.leader_cursor = (app.leader_cursor + 1) % LEADER_KEY_OPTIONS.len();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.leader_cursor = if app.leader_cursor == 0 {
                LEADER_KEY_OPTIONS.len() - 1
            } else {
                app.leader_cursor - 1
            };
        }
        KeyCode::Enter => {
            app.apply_leader_key(name);
            app.screen = Screen::InstanceDetail {
                name: name.to_string(),
            };
        }
        _ => {}
    }
}

fn handle_tutorial_list_keys(app: &mut App, code: KeyCode) {
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

fn handle_tutorial_view_keys(app: &mut App, code: KeyCode) {
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

// ── Operations ──────────────────────────────────────────────────────────────

fn do_init_settings(app: &mut App) {
    match config::init_global_settings() {
        Ok(true) => {
            let path = config::settings_json_path();
            app.settings = config::load_global_settings();
            app.message = Some(format!("Created settings.json at {}", path.display()));
        }
        Ok(false) => {
            let path = config::settings_json_path();
            app.message = Some(format!("settings.json already exists at {}", path.display()));
        }
        Err(e) => {
            app.message = Some(format!("Failed to create settings.json: {e}"));
        }
    }
}

fn open_instance_dir(name: &str, app: &mut App) {
    let dir = config::instance_dir(&app.settings, name);
    if !dir.exists() {
        app.message = Some(format!("Instance directory not found for '{name}'."));
        return;
    }

    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("explorer").arg(&dir).spawn();

    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(&dir).spawn();

    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open").arg(&dir).spawn();

    match result {
        Ok(_) => {
            app.message = Some(format!("Opened directory for '{name}'."));
        }
        Err(e) => {
            app.message = Some(format!("Failed to open directory: {e}"));
        }
    }
}

async fn do_install_font(app: &mut App, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    leave_tui(terminal);
    let msg = crate::font::install_nerd_font().await;
    println!("{msg}");
    println!("\nPress Enter to return to TUI...");
    let _ = std::io::stdin().read_line(&mut String::new());
    app.message = Some("Nerd Font install completed.".to_string());
    enter_tui(terminal);
}

fn do_launch(name: &str, app: &mut App, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    let instance_dir = config::instance_dir(&app.settings, name);
    leave_tui(terminal);

    match crate::neovim::launch(&instance_dir, &[]) {
        Ok(_status) => {
            app.message = Some(format!("Neovim exited for '{name}'."));
        }
        Err(e) => {
            eprintln!("Launch error: {e}");
            app.message = Some(format!("Launch failed: {e}"));
        }
    }

    enter_tui(terminal);
}

async fn do_update(
    name: &str,
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) {
    leave_tui(terminal);
    println!("Updating instance '{name}' ...");

    let result: Result<String, String> = async {
        let instance_dir = config::instance_dir(&app.settings, name);
        let manifest_path = InstanceManifest::manifest_path(&instance_dir);
        let mut manifest = InstanceManifest::load(&manifest_path).map_err(|e| e.to_string())?;

        let release = crate::github::fetch_latest_stable()
            .await
            .map_err(|e| e.to_string())?;

        if manifest.nvim_version == release.tag_name {
            return Ok(format!("'{name}' already at {}.", release.tag_name));
        }

        println!(
            "Updating from {} to {} ...",
            manifest.nvim_version, release.tag_name
        );

        let asset = crate::github::select_asset(&release).map_err(|e| e.to_string())?;
        let data = crate::github::download_asset(asset)
            .await
            .map_err(|e| e.to_string())?;

        // Clear old binaries and extract new ones.
        let bin_dir = instance_dir.join("bin");
        let _ = std::fs::remove_dir_all(&bin_dir);
        std::fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;

        let tmp_dir = instance_dir.join("_update_tmp");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;

        crate::archive::extract(&data, &tmp_dir, &asset.name).map_err(|e| e.to_string())?;
        crate::archive::install_nvim_binary(&tmp_dir, &bin_dir).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_dir_all(&tmp_dir);

        manifest.nvim_version = release.tag_name.clone();
        manifest.updated_at = chrono::Utc::now();
        manifest.save(&manifest_path).map_err(|e| e.to_string())?;

        Ok(format!("Updated '{name}' to {}.", release.tag_name))
    }
    .await;

    match result {
        Ok(msg) => {
            println!("{msg}");
            app.message = Some(msg);
            app.refresh_instances();
        }
        Err(e) => {
            eprintln!("Update failed: {e}");
            app.message = Some(format!("Update failed: {e}"));
        }
    }

    println!("\nPress Enter to return to TUI...");
    let _ = std::io::stdin().read_line(&mut String::new());
    enter_tui(terminal);
}

fn do_delete(name: &str, app: &mut App, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    let instance_dir = config::instance_dir(&app.settings, name);
    leave_tui(terminal);

    println!("Deleting instance '{name}' ...");
    match std::fs::remove_dir_all(&instance_dir) {
        Ok(()) => {
            println!("Deleted '{name}'.");
            app.refresh_instances();
            app.message = Some(format!("Deleted '{name}'."));
        }
        Err(e) => {
            eprintln!("Delete error: {e}");
            app.message = Some(format!("Delete failed: {e}"));
        }
    }

    println!("\nPress Enter to return to TUI...");
    let _ = std::io::stdin().read_line(&mut String::new());
    enter_tui(terminal);
}
