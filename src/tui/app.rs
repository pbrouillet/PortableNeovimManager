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
    ConfirmDelete { name: String },
    EditSettings,
    TutorialList,
    TutorialView { title: String, content: String },
    Marketplace { instance_name: String },
    CreateInstance,
}

// ── App state ───────────────────────────────────────────────────────────────

/// A single feature checkbox within a workload.
#[derive(Clone, Debug)]
pub struct FeatureCheckbox {
    pub name: String,
    pub enabled: bool,
}

/// A workload group in the EditFeatures screen.
#[derive(Clone, Debug)]
pub struct WorkloadCheckbox {
    pub workload_id: String,
    pub name: String,
    pub description: String,
    pub category: Option<String>,
    pub enabled: bool,
    pub expanded: bool,
    pub features: Vec<FeatureCheckbox>,
}

/// An item the cursor can land on in the feature list.
#[derive(Clone, Debug)]
pub enum FeatureCursorItem {
    AllToggle,
    GroupHeader(String),
    Workload(usize),
    Feature(usize, usize),
}

pub struct App {
    pub instances: Vec<InstanceManifest>,
    pub selected: usize,
    pub screen: Screen,
    pub message: Option<String>,
    pub should_quit: bool,
    /// Hierarchical checkbox state for the EditFeatures screen
    pub workload_checkboxes: Vec<WorkloadCheckbox>,
    /// Cursor position in the visible feature list
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
    /// Search query for instance list filtering
    pub instance_search: String,
    /// Whether the instance search input is active
    pub instance_search_active: bool,
    /// Filtered instance indices (into self.instances)
    pub instance_filtered: Vec<usize>,
    /// Cursor position in the settings screen
    pub settings_cursor: usize,
    /// Whether a settings field is being edited
    pub settings_editing: bool,
    /// Buffer for editing a settings field value
    pub settings_edit_buffer: String,
    /// Cached mason registry for the marketplace screen
    pub marketplace_registry: Option<crate::mason_registry::MasonRegistry>,
    /// All packages (filtered view)
    pub marketplace_packages: Vec<usize>,
    /// Current cursor position in marketplace
    pub marketplace_cursor: usize,
    /// Current category filter (None = all)
    pub marketplace_category: Option<crate::mason_registry::MasonCategory>,
    /// Search query for marketplace
    pub marketplace_search: String,
    /// Whether search input is active
    pub marketplace_search_active: bool,
    /// Packages selected for install in this session
    pub marketplace_selected: std::collections::HashSet<String>,
    /// Instance's existing mason_packages (to show already-installed status)
    pub marketplace_installed: Vec<String>,
    /// Loading/error message for marketplace
    pub marketplace_status: Option<String>,
    /// Name input for creating a new instance
    pub create_name: String,
    /// Cursor position in the create form (0 = name, 1 = preset)
    pub create_field_cursor: usize,
    /// Selected preset index (into registry.presets)
    pub create_preset_cursor: usize,
    /// Validation error message for the create form
    pub create_error: Option<String>,
}

impl App {
    fn new(instances: Vec<InstanceManifest>, registry: WorkloadRegistry, settings: GlobalSettings) -> Self {
        let tutorial_topics = registry.all_tutorial_topics();
        let tutorial_filtered = (0..tutorial_topics.len()).collect();
        let instance_filtered = (0..instances.len()).collect();
        Self {
            instances,
            selected: 0,
            screen: Screen::InstanceList,
            message: None,
            should_quit: false,
            workload_checkboxes: Vec::new(),
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
            instance_search: String::new(),
            instance_search_active: false,
            instance_filtered,
            settings_cursor: 0,
            settings_editing: false,
            settings_edit_buffer: String::new(),
            marketplace_registry: None,
            marketplace_packages: Vec::new(),
            marketplace_cursor: 0,
            marketplace_category: None,
            marketplace_search: String::new(),
            marketplace_search_active: false,
            marketplace_selected: std::collections::HashSet::new(),
            marketplace_installed: Vec::new(),
            marketplace_status: None,
            create_name: String::new(),
            create_field_cursor: 0,
            create_preset_cursor: 1, // default to ide-core
            create_error: None,
        }
    }

    fn refresh_instances(&mut self) {
        self.instances = load_instances(&self.settings);
        self.update_instance_filter();
        if self.selected >= self.instance_filtered.len() && !self.instance_filtered.is_empty() {
            self.selected = self.instance_filtered.len() - 1;
        }
        if self.instance_filtered.is_empty() {
            self.selected = 0;
        }
    }

    fn selected_name(&self) -> Option<String> {
        self.instance_filtered
            .get(self.selected)
            .and_then(|&idx| self.instances.get(idx))
            .map(|i| i.name.clone())
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

    fn update_instance_filter(&mut self) {
        let query = self.instance_search.to_lowercase();
        if query.is_empty() {
            self.instance_filtered = (0..self.instances.len()).collect();
        } else {
            self.instance_filtered = self
                .instances
                .iter()
                .enumerate()
                .filter(|(_, inst)| {
                    inst.name.to_lowercase().contains(&query)
                        || inst.nvim_version.to_lowercase().contains(&query)
                        || inst.workloads.iter().any(|w| w.to_lowercase().contains(&query))
                })
                .map(|(i, _)| i)
                .collect();
        }
        if self.selected >= self.instance_filtered.len() {
            self.selected = 0;
        }
    }

    /// Returns the flat list of cursor-navigable items in the current feature view.
    pub fn visible_feature_items(&self) -> Vec<FeatureCursorItem> {
        let mut items = Vec::new();
        items.push(FeatureCursorItem::AllToggle);

        // Collect distinct categories in order: uncategorized ("General") first, then named categories
        let mut seen_categories: Vec<Option<String>> = Vec::new();
        for wc in &self.workload_checkboxes {
            if !seen_categories.contains(&wc.category) {
                seen_categories.push(wc.category.clone());
            }
        }
        // Sort: None (General) first, then alphabetical named categories
        seen_categories.sort_by(|a, b| match (a, b) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (Some(a), Some(b)) => a.cmp(b),
        });

        for cat in &seen_categories {
            let label = cat.as_deref().unwrap_or("General").to_string();
            items.push(FeatureCursorItem::GroupHeader(label));
            for (i, wc) in self.workload_checkboxes.iter().enumerate() {
                if wc.category != *cat {
                    continue;
                }
                items.push(FeatureCursorItem::Workload(i));
                if wc.expanded {
                    for (j, _) in wc.features.iter().enumerate() {
                        items.push(FeatureCursorItem::Feature(i, j));
                    }
                }
            }
        }
        items
    }

    fn enter_edit_features(&mut self, name: &str) {
        let instance = self.instances.iter().find(|i| i.name == name);
        let enabled_workloads = instance.map(|i| &i.workloads);

        self.workload_checkboxes = self
            .registry
            .optional()
            .iter()
            .map(|w| {
                let workload_on = enabled_workloads
                    .map(|wl| wl.contains(&w.id))
                    .unwrap_or(false);

                // Build description with dependency info
                let mut desc = w.description.clone();
                if !w.depends_on.is_empty() {
                    let dep_names: Vec<&str> = w.depends_on.iter()
                        .filter_map(|d| self.registry.find_by_id(d).map(|dw| dw.name.as_str()))
                        .collect();
                    desc = format!("{} [requires: {}]", desc, dep_names.join(", "));
                }
                let dependents = self.registry.dependents_of(&w.id);
                if !dependents.is_empty() {
                    let dep_names: Vec<&str> = dependents.iter()
                        .filter_map(|d| self.registry.find_by_id(d).map(|dw| dw.name.as_str()))
                        .collect();
                    desc = format!("{} [needed by: {}]", desc, dep_names.join(", "));
                }

                WorkloadCheckbox {
                    workload_id: w.id.clone(),
                    name: w.name.clone(),
                    description: desc,
                    category: w.category.clone(),
                    enabled: workload_on,
                    expanded: false,
                    features: w
                        .features
                        .iter()
                        .map(|f| FeatureCheckbox {
                            name: f.name.clone(),
                            enabled: workload_on && f.default_enabled,
                        })
                        .collect(),
                }
            })
            .collect();
        self.feature_cursor = 0;
        self.screen = Screen::EditFeatures {
            name: name.to_string(),
        };
        self.message = None;
    }

    fn toggle_feature(&mut self) {
        let items = self.visible_feature_items();
        let Some(item) = items.get(self.feature_cursor) else {
            return;
        };

        match item {
            FeatureCursorItem::AllToggle => {
                // Toggle all: if any are enabled, disable all; otherwise enable all
                let any_enabled = self.workload_checkboxes.iter().any(|wc| wc.enabled);
                let new_state = !any_enabled;
                for wc in &mut self.workload_checkboxes {
                    wc.enabled = new_state;
                    for fc in &mut wc.features {
                        fc.enabled = new_state;
                    }
                }
            }
            FeatureCursorItem::GroupHeader(_) => {
                // Group headers are not toggleable
            }
            FeatureCursorItem::Workload(wi) => {
                let wc = &mut self.workload_checkboxes[*wi];
                let new_state = !wc.enabled;
                wc.enabled = new_state;
                for fc in &mut wc.features {
                    fc.enabled = new_state;
                }

                if new_state {
                    // Enable dependencies
                    let workload_id = wc.workload_id.clone();
                    if let Some(workload) = self.registry.find_by_id(&workload_id) {
                        let deps: Vec<String> = workload.depends_on.clone();
                        for dep_id in &deps {
                            if let Some(dep_wc) = self
                                .workload_checkboxes
                                .iter_mut()
                                .find(|w| w.workload_id == *dep_id)
                            {
                                dep_wc.enabled = true;
                                for fc in &mut dep_wc.features {
                                    fc.enabled = true;
                                }
                            }
                        }
                    }
                } else {
                    // Disable dependents
                    let workload_id = wc.workload_id.clone();
                    let dependents = self.registry.dependents_of(&workload_id);
                    for dep_id in &dependents {
                        if let Some(dep_wc) = self
                            .workload_checkboxes
                            .iter_mut()
                            .find(|w| w.workload_id == *dep_id)
                        {
                            dep_wc.enabled = false;
                            for fc in &mut dep_wc.features {
                                fc.enabled = false;
                            }
                        }
                    }
                }
            }
            FeatureCursorItem::Feature(wi, fi) => {
                let wc = &mut self.workload_checkboxes[*wi];
                wc.features[*fi].enabled = !wc.features[*fi].enabled;
                // Update workload-level state based on feature states
                wc.enabled = wc.features.iter().any(|f| f.enabled);
            }
        }
    }

    fn toggle_expand(&mut self) {
        let items = self.visible_feature_items();
        if let Some(FeatureCursorItem::Workload(wi)) = items.get(self.feature_cursor) {
            self.workload_checkboxes[*wi].expanded = !self.workload_checkboxes[*wi].expanded;
        }
    }

    fn apply_features(&mut self, name: &str) {
        let features: Vec<String> = self
            .workload_checkboxes
            .iter()
            .filter(|wc| wc.enabled)
            .map(|wc| wc.workload_id.clone())
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
                    &manifest.workloads,
                    &manifest.leader_key,
                    &manifest.mason_packages,
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

    fn enter_marketplace(&mut self, instance_name: &str) {
        let dir = config::instance_dir(&self.settings, instance_name);
        let manifest_path = InstanceManifest::manifest_path(&dir);
        self.marketplace_installed = InstanceManifest::load(&manifest_path)
            .map(|m| m.mason_packages)
            .unwrap_or_default();

        self.marketplace_cursor = 0;
        self.marketplace_search.clear();
        self.marketplace_search_active = false;
        self.marketplace_selected.clear();
        self.marketplace_status = None;

        if let Ok(reg) = crate::mason_registry::load_from_cache() {
            self.marketplace_category = Some(crate::mason_registry::MasonCategory::Lsp);
            self.marketplace_registry = Some(reg);
            self.update_marketplace_filter();
        } else {
            self.marketplace_registry = None;
            self.marketplace_packages = Vec::new();
            self.marketplace_status =
                Some("No cached registry. Press R to fetch from GitHub.".to_string());
        }

        self.screen = Screen::Marketplace {
            instance_name: instance_name.to_string(),
        };
        self.message = None;
    }

    fn enter_create_instance(&mut self) {
        self.create_name.clear();
        self.create_field_cursor = 0;
        self.create_preset_cursor = 1; // default to ide-core
        self.create_error = None;
        self.screen = Screen::CreateInstance;
        self.message = None;
    }

    fn update_marketplace_filter(&mut self) {
        let Some(ref reg) = self.marketplace_registry else {
            self.marketplace_packages = Vec::new();
            return;
        };
        let query = self.marketplace_search.to_lowercase();
        self.marketplace_packages = reg
            .packages
            .iter()
            .enumerate()
            .filter(|(_, pkg)| {
                if let Some(ref cat) = self.marketplace_category {
                    if !pkg.is_category(cat) {
                        return false;
                    }
                }
                if !query.is_empty() {
                    let matches = pkg.name.to_lowercase().contains(&query)
                        || pkg.description.to_lowercase().contains(&query)
                        || pkg
                            .languages
                            .iter()
                            .any(|l| l.to_lowercase().contains(&query));
                    if !matches {
                        return false;
                    }
                }
                true
            })
            .map(|(i, _)| i)
            .collect();

        if self.marketplace_cursor >= self.marketplace_packages.len() {
            self.marketplace_cursor = 0;
        }
    }

    fn marketplace_toggle_selected(&mut self) {
        let Some(ref reg) = self.marketplace_registry else {
            return;
        };
        if let Some(&idx) = self.marketplace_packages.get(self.marketplace_cursor) {
            let name = reg.packages[idx].name.clone();
            if self.marketplace_selected.contains(&name) {
                self.marketplace_selected.remove(&name);
            } else {
                self.marketplace_selected.insert(name);
            }
        }
    }

    fn marketplace_apply(&mut self, instance_name: &str) -> Result<usize, String> {
        if self.marketplace_selected.is_empty() {
            return Ok(0);
        }
        let dir = config::instance_dir(&self.settings, instance_name);
        let manifest_path = InstanceManifest::manifest_path(&dir);
        let mut manifest = InstanceManifest::load(&manifest_path)
            .map_err(|e| format!("Failed to load manifest: {e}"))?;

        let mut added = 0;
        for pkg_name in &self.marketplace_selected {
            if !manifest.mason_packages.contains(pkg_name) {
                manifest.mason_packages.push(pkg_name.clone());
                added += 1;
            }
        }

        if added > 0 {
            manifest.updated_at = chrono::Utc::now();
            manifest
                .save(&manifest_path)
                .map_err(|e| format!("Failed to save manifest: {e}"))?;

            let data_dir = dir.join("data");
            let init_lua = crate::plugins::generate_init_lua_full(
                &data_dir,
                &self.registry,
                &manifest.workloads,
                &manifest.disabled_features,
                &manifest.extra_features,
                &manifest.leader_key,
                &manifest.mason_packages,
            );
            let init_lua_path = dir.join("config").join("nvim").join("init.lua");
            std::fs::write(&init_lua_path, init_lua)
                .map_err(|e| format!("Failed to write init.lua: {e}"))?;
        }

        self.marketplace_installed = manifest.mason_packages;
        self.marketplace_selected.clear();

        Ok(added)
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
                    Screen::ConfirmDelete { name } => {
                        let name = name.clone();
                        handle_confirm_delete_keys(&mut app, key.code, &name);
                    }
                    Screen::EditSettings => {
                        handle_settings_keys(&mut app, key.code);
                    }
                    Screen::TutorialList => {
                        handle_tutorial_list_keys(&mut app, key.code);
                    }
                    Screen::TutorialView { .. } => {
                        handle_tutorial_view_keys(&mut app, key.code);
                    }
                    Screen::Marketplace { instance_name } => {
                        let instance_name = instance_name.clone();
                        handle_marketplace_keys(&mut app, key.code, &instance_name).await;
                    }
                    Screen::CreateInstance => {
                        handle_create_keys(&mut app, key.code, &mut terminal).await;
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
            do_install_font(app, terminal).await;
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
            open_instance_dir(name, app);
        }
        KeyCode::Char('t') => {
            app.open_tutorial_list(Screen::InstanceDetail { name: name.to_string() });
        }
        KeyCode::Char('p') => {
            app.enter_marketplace(name);
        }
        _ => {}
    }
}

fn handle_features_keys(app: &mut App, code: KeyCode, name: &str) {
    let visible_count = app.visible_feature_items().len();
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.screen = Screen::InstanceDetail {
                name: name.to_string(),
            };
            app.message = None;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if visible_count > 0 {
                app.feature_cursor = (app.feature_cursor + 1) % visible_count;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if visible_count > 0 {
                app.feature_cursor = if app.feature_cursor == 0 {
                    visible_count - 1
                } else {
                    app.feature_cursor - 1
                };
            }
        }
        KeyCode::Char(' ') => {
            app.toggle_feature();
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.toggle_expand();
        }
        KeyCode::Left | KeyCode::Char('h') => {
            // Collapse: if on a feature, jump to its parent; if on workload, collapse it
            let items = app.visible_feature_items();
            if let Some(item) = items.get(app.feature_cursor) {
                match item {
                    FeatureCursorItem::AllToggle | FeatureCursorItem::GroupHeader(_) => {
                        // No-op
                    }
                    FeatureCursorItem::Feature(wi, _) => {
                        // Jump cursor to the parent workload
                        let wi = *wi;
                        if let Some(pos) = items.iter().position(|i| matches!(i, FeatureCursorItem::Workload(w) if *w == wi)) {
                            app.feature_cursor = pos;
                        }
                    }
                    FeatureCursorItem::Workload(wi) => {
                        app.workload_checkboxes[*wi].expanded = false;
                    }
                }
            }
        }
        KeyCode::Enter => {
            app.apply_features(name);
            app.screen = Screen::InstanceDetail {
                name: name.to_string(),
            };
        }
        KeyCode::Char('t') => {
            let items = app.visible_feature_items();
            if let Some(FeatureCursorItem::Workload(wi)) = items.get(app.feature_cursor) {
                let workload_id = &app.workload_checkboxes[*wi].workload_id;
                if let Some((title, content)) = app.registry.tutorial_content(workload_id) {
                    let return_to = Screen::EditFeatures { name: name.to_string() };
                    app.open_tutorial_view(title, content, return_to);
                } else {
                    app.message = Some("No tutorial available for this workload.".to_string());
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

/// Settings fields: instances_dir, default_leader_key, confirm_destructive
const SETTINGS_FIELD_COUNT: usize = 3;

fn handle_settings_keys(app: &mut App, code: KeyCode) {
    if app.settings_editing {
        match code {
            KeyCode::Esc => {
                app.settings_editing = false;
                app.settings_edit_buffer.clear();
            }
            KeyCode::Enter => {
                // Apply the edit
                match app.settings_cursor {
                    0 => {
                        // instances_dir
                        let new_path = std::path::PathBuf::from(&app.settings_edit_buffer);
                        if app.settings_edit_buffer.is_empty() {
                            app.message = Some("Path cannot be empty.".to_string());
                        } else {
                            app.settings.instances_dir = new_path;
                            app.message = Some("Instances directory updated.".to_string());
                        }
                    }
                    1 => {
                        // default_leader_key — validate against allowed keys
                        let key = &app.settings_edit_buffer;
                        if LEADER_KEY_OPTIONS.iter().any(|(v, _)| *v == key.as_str()) {
                            app.settings.default_leader_key = key.clone();
                            app.message = Some(format!(
                                "Default leader key set to '{}'.",
                                config::leader_key_display(key)
                            ));
                        } else {
                            app.message = Some("Invalid leader key. Use Space, comma, backslash, or semicolon.".to_string());
                        }
                    }
                    2 => {
                        // confirm_destructive — toggle handled separately
                    }
                    _ => {}
                }
                app.settings_editing = false;
                app.settings_edit_buffer.clear();

                // Save settings
                if let Err(e) = config::save_global_settings(&app.settings) {
                    app.message = Some(format!("Failed to save settings: {e}"));
                }
            }
            KeyCode::Backspace => {
                app.settings_edit_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.settings_edit_buffer.push(c);
            }
            _ => {}
        }
        return;
    }

    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.screen = Screen::InstanceList;
            app.message = None;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.settings_cursor = (app.settings_cursor + 1) % SETTINGS_FIELD_COUNT;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.settings_cursor = if app.settings_cursor == 0 {
                SETTINGS_FIELD_COUNT - 1
            } else {
                app.settings_cursor - 1
            };
        }
        KeyCode::Enter => {
            match app.settings_cursor {
                0 => {
                    // Edit instances_dir
                    app.settings_editing = true;
                    app.settings_edit_buffer = app.settings.instances_dir.to_string_lossy().to_string();
                }
                1 => {
                    // Cycle through leader key options
                    let current = &app.settings.default_leader_key;
                    let idx = LEADER_KEY_OPTIONS
                        .iter()
                        .position(|(v, _)| *v == current.as_str())
                        .unwrap_or(0);
                    let next = (idx + 1) % LEADER_KEY_OPTIONS.len();
                    app.settings.default_leader_key = LEADER_KEY_OPTIONS[next].0.to_string();
                    app.message = Some(format!(
                        "Default leader key: {}",
                        config::leader_key_display(LEADER_KEY_OPTIONS[next].0)
                    ));
                    if let Err(e) = config::save_global_settings(&app.settings) {
                        app.message = Some(format!("Failed to save: {e}"));
                    }
                }
                2 => {
                    // Toggle confirm_destructive
                    app.settings.confirm_destructive = !app.settings.confirm_destructive;
                    let state = if app.settings.confirm_destructive { "on" } else { "off" };
                    app.message = Some(format!("Confirm destructive actions: {state}"));
                    if let Err(e) = config::save_global_settings(&app.settings) {
                        app.message = Some(format!("Failed to save: {e}"));
                    }
                }
                _ => {}
            }
        }
        KeyCode::Char(' ') if app.settings_cursor == 2 => {
            // Quick toggle for boolean field
            app.settings.confirm_destructive = !app.settings.confirm_destructive;
            let state = if app.settings.confirm_destructive { "on" } else { "off" };
            app.message = Some(format!("Confirm destructive actions: {state}"));
            if let Err(e) = config::save_global_settings(&app.settings) {
                app.message = Some(format!("Failed to save: {e}"));
            }
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

async fn handle_marketplace_keys(app: &mut App, code: KeyCode, instance_name: &str) {
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

fn handle_confirm_delete_keys(app: &mut App, code: KeyCode, name: &str) {
    match code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            match crate::instance::delete(name, &app.settings) {
                Ok(()) => {
                    app.refresh_instances();
                    app.message = Some(format!("Deleted '{name}'."));
                    app.screen = Screen::InstanceList;
                }
                Err(e) => {
                    app.message = Some(format!("Delete failed: {e}"));
                    app.screen = Screen::InstanceList;
                }
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.message = Some("Delete cancelled.".to_string());
            // Return to detail if we came from there, otherwise list
            if app.instances.iter().any(|i| i.name == name) {
                app.screen = Screen::InstanceDetail { name: name.to_string() };
            } else {
                app.screen = Screen::InstanceList;
            }
        }
        _ => {}
    }
}

async fn handle_create_keys(
    app: &mut App,
    code: KeyCode,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) {
    // When on the name field (field 0), capture text input
    if app.create_field_cursor == 0 {
        match code {
            KeyCode::Esc => {
                app.screen = Screen::InstanceList;
                app.message = None;
                return;
            }
            KeyCode::Tab | KeyCode::Down => {
                app.create_field_cursor = 1;
                return;
            }
            KeyCode::Enter => {
                // Fall through to the create logic below
            }
            KeyCode::Backspace => {
                app.create_name.pop();
                // Live validation
                if !app.create_name.is_empty() {
                    app.create_error = crate::cli::validate_instance_name(&app.create_name).err();
                } else {
                    app.create_error = None;
                }
                return;
            }
            KeyCode::Char(c) => {
                app.create_name.push(c);
                // Live validation
                app.create_error = crate::cli::validate_instance_name(&app.create_name).err();
                return;
            }
            _ => return,
        }
    }

    // When on the preset field (field 1)
    if app.create_field_cursor == 1 && code != KeyCode::Enter {
        match code {
            KeyCode::Esc => {
                app.screen = Screen::InstanceList;
                app.message = None;
                return;
            }
            KeyCode::Tab | KeyCode::Up | KeyCode::BackTab => {
                app.create_field_cursor = 0;
                return;
            }
            KeyCode::Left | KeyCode::Char('h') => {
                let count = app.registry.presets.len();
                if count > 0 {
                    app.create_preset_cursor = if app.create_preset_cursor == 0 {
                        count - 1
                    } else {
                        app.create_preset_cursor - 1
                    };
                }
                return;
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let count = app.registry.presets.len();
                if count > 0 {
                    app.create_preset_cursor = (app.create_preset_cursor + 1) % count;
                }
                return;
            }
            _ => return,
        }
    }

    // Enter pressed — attempt creation
    if code == KeyCode::Enter {
        if app.create_name.is_empty() {
            app.create_error = Some("Name cannot be empty.".to_string());
            app.create_field_cursor = 0;
            return;
        }
        if let Err(e) = crate::cli::validate_instance_name(&app.create_name) {
            app.create_error = Some(e);
            app.create_field_cursor = 0;
            return;
        }

        // Check if instance already exists
        let instance_dir = config::instance_dir(&app.settings, &app.create_name);
        if instance_dir.exists() {
            app.create_error = Some(format!("Instance '{}' already exists.", app.create_name));
            app.create_field_cursor = 0;
            return;
        }

        // Resolve preset to feature list
        let features = if let Some(preset) = app.registry.presets.get(app.create_preset_cursor) {
            preset.workloads.clone()
        } else {
            Vec::new()
        };

        let name = app.create_name.clone();
        leave_tui(terminal);
        println!("Creating instance '{}' ...", name);

        match crate::instance::create(&name, None, features, &app.registry, &app.settings).await {
            Ok(()) => {
                app.refresh_instances();
                app.message = Some(format!("✓ Created instance '{name}'."));
            }
            Err(e) => {
                eprintln!("Create failed: {e}");
                app.message = Some(format!("Create failed: {e}"));
            }
        }

        println!("\nPress Enter to return to TUI...");
        let _ = std::io::stdin().read_line(&mut String::new());
        enter_tui(terminal);
        app.screen = Screen::InstanceList;
    }
}

// ── Operations ──────────────────────────────────────────────────────────────

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

    match crate::instance::update(name, None, &app.settings).await {
        Ok(()) => {
            app.refresh_instances();
            app.message = Some(format!("Updated '{name}'."));
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


