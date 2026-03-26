use std::io::{self, stdout};
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::config::{self, GlobalSettings, InstanceManifest, LEADER_KEY_OPTIONS};
use crate::workload::WorkloadRegistry;

use super::menu::bar::MenuBar;
use super::menu::definitions::build_menu_bar;
use super::state::{
    LayoutCache, Screen, FeatureCheckbox, WorkloadCheckbox, FeatureCursorItem, TerminalProfileEntry,
};
use super::ui;

// ── App state ───────────────────────────────────────────────────────────────

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
    /// Cached memory snapshot for the Monitor screen
    pub monitor_snapshot: Option<crate::monitor::InstanceMemorySnapshot>,
    /// Error message when monitoring fails
    pub monitor_error: Option<String>,
    /// Tick counter for live refresh (refreshes every N ticks)
    pub monitor_tick: u8,
    /// Init config: focused panel (0 = pre, 1 = post)
    pub(crate) init_config_panel: usize,
    /// Init config: scroll offset for pre-plugins panel
    pub(crate) init_config_pre_scroll: u16,
    /// Init config: scroll offset for post-plugins panel
    pub(crate) init_config_post_scroll: u16,
    /// Init config: whether we're in edit mode
    pub(crate) init_config_editing: bool,
    /// Init config: editor buffer (lines)
    pub(crate) init_config_buffer: Vec<String>,
    /// Init config: editor cursor row
    pub(crate) init_config_cursor_row: usize,
    /// Init config: editor cursor col
    pub(crate) init_config_cursor_col: usize,
    /// Init config: editor scroll offset
    pub(crate) init_config_editor_scroll: u16,
    /// Init config: undo stack (buffer snapshot, row, col)
    pub(crate) init_config_undo: Vec<(Vec<String>, usize, usize)>,
    /// Init config: redo stack
    pub(crate) init_config_redo: Vec<(Vec<String>, usize, usize)>,
    /// Init config: buffer snapshot at edit start (for dirty detection)
    pub(crate) init_config_original_buffer: Vec<String>,
    /// Init config: whether the buffer has unsaved changes
    pub(crate) init_config_dirty: bool,
    /// Init config: whether we're showing the "discard changes?" prompt
    pub(crate) init_config_confirm_discard: bool,
    /// Terminal font config: profile entries from discovered installations
    pub(crate) terminal_entries: Vec<TerminalProfileEntry>,
    /// Terminal font config: cursor position
    pub(crate) terminal_cursor: usize,
    /// Terminal font config: whether "apply to all profiles (defaults)" is toggled
    pub(crate) terminal_apply_defaults: bool,
    /// Menu bar widget with pulldown menus
    pub menu_bar: MenuBar,
    /// Cached widget positions from last draw for mouse hit testing
    pub layout_cache: LayoutCache,
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
            monitor_snapshot: None,
            monitor_error: None,
            monitor_tick: 0,
            init_config_panel: 0,
            init_config_pre_scroll: 0,
            init_config_post_scroll: 0,
            init_config_editing: false,
            init_config_buffer: vec![String::new()],
            init_config_cursor_row: 0,
            init_config_cursor_col: 0,
            init_config_editor_scroll: 0,
            init_config_undo: Vec::new(),
            init_config_redo: Vec::new(),
            init_config_original_buffer: vec![String::new()],
            init_config_dirty: false,
            init_config_confirm_discard: false,
            terminal_entries: Vec::new(),
            terminal_cursor: 0,
            terminal_apply_defaults: true,
            menu_bar: MenuBar::new(build_menu_bar()),
            layout_cache: LayoutCache::default(),
        }
    }

    /// Clamp all cursor and scroll positions to valid bounds.
    /// Called on terminal resize to prevent out-of-bounds indices.
    pub fn clamp_cursors(&mut self) {
        let inst_len = self.instance_filtered.len();
        if inst_len == 0 {
            self.selected = 0;
        } else if self.selected >= inst_len {
            self.selected = inst_len - 1;
        }

        let tut_len = self.tutorial_filtered.len();
        if tut_len == 0 {
            self.tutorial_cursor = 0;
        } else if self.tutorial_cursor >= tut_len {
            self.tutorial_cursor = tut_len - 1;
        }

        // tutorial_scroll is clamped dynamically during render, but reset to
        // a safe upper bound here to avoid a stale large value.
        self.tutorial_scroll = self.tutorial_scroll;

        let mp_len = self.marketplace_packages.len();
        if mp_len == 0 {
            self.marketplace_cursor = 0;
        } else if self.marketplace_cursor >= mp_len {
            self.marketplace_cursor = mp_len - 1;
        }
    }

    /// Dispatch a high-level command from the menu bar or function bar.
    /// Returns without action if the command doesn't apply to the current screen.
    async fn dispatch(
        &mut self,
        cmd: super::command::Command,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) {
        use super::command::Command as C;
        match cmd {
            C::Quit => self.should_quit = true,
            C::Back => match &self.screen {
                Screen::InstanceList => self.should_quit = true,
                Screen::InstanceDetail { .. } => {
                    self.screen = Screen::InstanceList;
                    self.message = None;
                }
                Screen::EditFeatures { name } => {
                    let n = name.clone();
                    self.screen = Screen::InstanceDetail { name: n };
                    self.message = None;
                }
                Screen::TutorialList | Screen::TutorialView { .. } => self.tutorial_return(),
                _ => {
                    self.screen = Screen::InstanceList;
                    self.message = None;
                }
            },
            C::CreateInstance => self.enter_create_instance(),
            C::LaunchInstance => {
                if let Some(name) = self.current_instance_name() {
                    do_launch(&name, self, terminal);
                }
            }
            C::UpdateInstance => {
                if let Some(name) = self.current_instance_name() {
                    do_update(&name, self, terminal).await;
                }
            }
            C::DeleteInstance => {
                if let Some(name) = self.current_instance_name() {
                    self.screen = Screen::ConfirmDelete { name };
                    self.message = None;
                }
            }
            C::EditFeatures => {
                if let Some(name) = self.current_instance_name() {
                    self.enter_edit_features(&name);
                }
            }
            C::EditLeaderKey => {
                if let Some(name) = self.current_instance_name() {
                    self.enter_edit_leader(&name);
                }
            }
            C::OpenSettings => {
                self.settings_cursor = 0;
                self.settings_editing = false;
                self.settings_edit_buffer.clear();
                self.screen = Screen::EditSettings;
                self.message = None;
            }
            C::OpenTutorials => {
                let current = self.screen.clone();
                self.open_tutorial_list(current);
            }
            C::OpenMarketplace => {
                if let Some(name) = self.current_instance_name() {
                    self.enter_marketplace(&name);
                }
            }
            C::OpenMonitor => {
                if let Some(name) = self.current_instance_name() {
                    self.enter_monitor(&name);
                }
            }
            C::OpenInitConfig => {
                if let Some(name) = self.current_instance_name() {
                    self.enter_init_config(&name);
                }
            }
            C::OpenTerminalFont => {
                let installations = crate::font::find_terminals();
                if !installations.is_empty() {
                    self.enter_configure_terminal_font(installations);
                } else {
                    self.message = Some("No supported terminals found.".to_string());
                }
            }
            C::InstallNerdFont => {
                do_install_font(self, terminal).await;
            }
            C::Refresh => {
                self.refresh_instances();
                self.message = Some("Refreshed instance list.".to_string());
            }
            C::ActivateMenuBar => {
                self.menu_bar.state.is_active = true;
                self.menu_bar.state.is_dropped = false;
            }
            // Navigation and toggle commands are handled by per-screen key handlers
            _ => {}
        }
    }

    /// Get the instance name relevant to the current screen context.
    fn current_instance_name(&self) -> Option<String> {
        match &self.screen {
            Screen::InstanceList => self.selected_name(),
            Screen::InstanceDetail { name }
            | Screen::EditFeatures { name }
            | Screen::EditLeaderKey { name }
            | Screen::ConfirmDelete { name }
            | Screen::Marketplace { instance_name: name }
            | Screen::Monitor { name }
            | Screen::InitConfig { name } => Some(name.clone()),
            _ => None,
        }
    }

    /// Handle scroll wheel: move cursor up/down in the current list.
    fn handle_scroll(&mut self, delta: i32) {
        match &self.screen {
            Screen::InstanceList => {
                let len = self.instance_filtered.len();
                if len > 0 {
                    if delta > 0 {
                        self.selected = (self.selected + 1).min(len - 1);
                    } else if self.selected > 0 {
                        self.selected -= 1;
                    }
                }
            }
            Screen::EditFeatures { .. } => {
                let len = self.visible_feature_items().len();
                if len > 0 {
                    if delta > 0 {
                        self.feature_cursor = (self.feature_cursor + 1).min(len - 1);
                    } else if self.feature_cursor > 0 {
                        self.feature_cursor -= 1;
                    }
                }
            }
            Screen::Marketplace { .. } => {
                let len = self.marketplace_packages.len();
                if len > 0 {
                    if delta > 0 {
                        self.marketplace_cursor = (self.marketplace_cursor + 1).min(len - 1);
                    } else if self.marketplace_cursor > 0 {
                        self.marketplace_cursor -= 1;
                    }
                }
            }
            Screen::TutorialList => {
                let len = self.tutorial_filtered.len();
                if len > 0 {
                    if delta > 0 {
                        self.tutorial_cursor = (self.tutorial_cursor + 1).min(len - 1);
                    } else if self.tutorial_cursor > 0 {
                        self.tutorial_cursor -= 1;
                    }
                }
            }
            Screen::TutorialView { .. } => {
                if delta > 0 {
                    self.tutorial_scroll = self.tutorial_scroll.saturating_add(3);
                } else {
                    self.tutorial_scroll = self.tutorial_scroll.saturating_sub(3);
                }
            }
            Screen::EditSettings => {
                // settings has a small fixed list
                let len = 3; // root_dir, auto_update, auto_cleanup
                if delta > 0 {
                    self.settings_cursor = (self.settings_cursor + 1).min(len - 1);
                } else if self.settings_cursor > 0 {
                    self.settings_cursor -= 1;
                }
            }
            Screen::ConfigureTerminalFont => {
                let len = self.terminal_entries.len();
                if len > 0 {
                    if delta > 0 {
                        self.terminal_cursor = (self.terminal_cursor + 1).min(len - 1);
                    } else if self.terminal_cursor > 0 {
                        self.terminal_cursor -= 1;
                    }
                }
            }
            _ => {}
        }
    }

    /// Handle a mouse click on the content area using the layout cache.
    fn handle_click(&mut self, pos: ratatui::layout::Position) {
        // Check cached list item positions
        for &(rect, idx) in &self.layout_cache.list_items {
            if rect.contains(pos) {
                match &self.screen {
                    Screen::InstanceList => {
                        if idx < self.instance_filtered.len() {
                            self.selected = idx;
                        }
                    }
                    Screen::EditFeatures { .. } => {
                        let len = self.visible_feature_items().len();
                        if idx < len {
                            self.feature_cursor = idx;
                        }
                    }
                    Screen::Marketplace { .. } => {
                        if idx < self.marketplace_packages.len() {
                            self.marketplace_cursor = idx;
                        }
                    }
                    Screen::TutorialList => {
                        if idx < self.tutorial_filtered.len() {
                            self.tutorial_cursor = idx;
                        }
                    }
                    Screen::ConfigureTerminalFont => {
                        if idx < self.terminal_entries.len() {
                            self.terminal_cursor = idx;
                        }
                    }
                    _ => {}
                }
                return;
            }
        }
    }

    pub(crate) fn refresh_instances(&mut self) {
        self.instances = load_instances(&self.settings);
        self.update_instance_filter();
        if self.selected >= self.instance_filtered.len() && !self.instance_filtered.is_empty() {
            self.selected = self.instance_filtered.len() - 1;
        }
        if self.instance_filtered.is_empty() {
            self.selected = 0;
        }
    }

    pub(crate) fn selected_name(&self) -> Option<String> {
        self.instance_filtered
            .get(self.selected)
            .and_then(|&idx| self.instances.get(idx))
            .map(|i| i.name.clone())
    }

    pub(crate) fn open_tutorial_list(&mut self, return_to: Screen) {
        self.tutorial_return_screen = Some(Box::new(return_to));
        self.tutorial_cursor = 0;
        self.tutorial_search.clear();
        self.tutorial_search_active = false;
        self.update_tutorial_filter();
        self.screen = Screen::TutorialList;
        self.message = None;
    }

    pub(crate) fn open_tutorial_view(&mut self, title: String, content: String, return_to: Screen) {
        self.tutorial_return_screen = Some(Box::new(return_to));
        self.tutorial_scroll = 0;
        self.screen = Screen::TutorialView { title, content };
    }

    pub(crate) fn tutorial_return(&mut self) {
        if let Some(screen) = self.tutorial_return_screen.take() {
            self.screen = *screen;
        } else {
            self.screen = Screen::InstanceList;
        }
        self.message = None;
    }

    pub(crate) fn update_tutorial_filter(&mut self) {
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

    pub(crate) fn update_instance_filter(&mut self) {
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

    pub(crate) fn enter_edit_features(&mut self, name: &str) {
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

    pub(crate) fn toggle_feature(&mut self) {
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

    pub(crate) fn toggle_expand(&mut self) {
        let items = self.visible_feature_items();
        if let Some(FeatureCursorItem::Workload(wi)) = items.get(self.feature_cursor) {
            self.workload_checkboxes[*wi].expanded = !self.workload_checkboxes[*wi].expanded;
        }
    }

    pub(crate) fn apply_features(&mut self, name: &str) {
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

    pub(crate) fn enter_edit_leader(&mut self, name: &str) {
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

    pub(crate) fn apply_leader_key(&mut self, name: &str) {
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
                    manifest.init_lua_pre.as_deref(),
                    manifest.init_lua_post.as_deref(),
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

    pub(crate) fn enter_marketplace(&mut self, instance_name: &str) {
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

    pub(crate) fn enter_create_instance(&mut self) {
        self.create_name.clear();
        self.create_field_cursor = 0;
        self.create_preset_cursor = 1; // default to ide-core
        self.create_error = None;
        self.screen = Screen::CreateInstance;
        self.message = None;
    }

    pub(crate) fn enter_monitor(&mut self, instance_name: &str) {
        self.monitor_snapshot = None;
        self.monitor_error = None;
        self.monitor_tick = 0;
        self.refresh_monitor(instance_name);
        self.screen = Screen::Monitor {
            name: instance_name.to_string(),
        };
        self.message = None;
    }

    pub(crate) fn refresh_monitor(&mut self, instance_name: &str) {
        let dir = config::instance_dir(&self.settings, instance_name);
        let nvim_binary = crate::neovim::find_nvim_binary(&dir).ok();
        match crate::monitor::full_snapshot(&dir, nvim_binary.as_deref()) {
            Ok(snap) => {
                self.monitor_snapshot = Some(snap);
                self.monitor_error = None;
            }
            Err(e) => {
                self.monitor_snapshot = None;
                self.monitor_error = Some(e.to_string());
            }
        }
    }

    pub(crate) fn enter_init_config(&mut self, name: &str) {
        self.init_config_panel = 0;
        self.init_config_pre_scroll = 0;
        self.init_config_post_scroll = 0;
        self.screen = Screen::InitConfig { name: name.to_string() };
        self.message = None;
    }

    pub fn enter_configure_terminal_font(
        &mut self,
        installations: Vec<crate::font::TerminalInstallation>,
    ) {
        let mut entries = Vec::new();
        for install in &installations {
            for profile in &install.profiles {
                entries.push(TerminalProfileEntry {
                    kind: install.kind.clone(),
                    install_label: install.label.clone(),
                    config_path: install.config_path.clone(),
                    profile_id: profile.id.clone(),
                    name: profile.name.clone(),
                    current_font: profile.current_font.clone(),
                    selected: false,
                    supports_defaults: install.supports_defaults,
                    read_only: install.read_only,
                });
            }
        }
        // Default to "apply to defaults" if any installation supports it and
        // doesn't already have the font set
        self.terminal_apply_defaults = installations
            .iter()
            .any(|i| {
                i.supports_defaults
                    && i.defaults_font.as_deref() != Some(crate::font::NERD_FONT_FACE)
            });
        self.terminal_entries = entries;
        self.terminal_cursor = 0;
        self.screen = Screen::ConfigureTerminalFont;
        self.message = None;
    }

    pub(crate) fn update_marketplace_filter(&mut self) {
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

    pub(crate) fn marketplace_toggle_selected(&mut self) {
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

    pub(crate) fn marketplace_apply(&mut self, instance_name: &str) -> Result<usize, String> {
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
                manifest.init_lua_pre.as_deref(),
                manifest.init_lua_post.as_deref(),
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

pub(crate) fn load_instances(settings: &GlobalSettings) -> Vec<InstanceManifest> {
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
pub(crate) fn leave_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();
}

pub(crate) fn enter_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    let _ = enable_raw_mode();
    let _ = execute!(stdout(), EnterAlternateScreen);
    let _ = terminal.hide_cursor();
    let _ = terminal.clear();
}

// ── Main loop ───────────────────────────────────────────────────────────────

pub async fn run(settings: GlobalSettings) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    execute!(
        stdout(),
        EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let instances = load_instances(&settings);
    let registry = crate::workload::load_workloads();
    let mut app = App::new(instances, registry, settings);

    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        if event::poll(Duration::from_millis(200))? {
            let ev = event::read()?;
            match ev {
                Event::Resize(_, _) => {
                    app.clamp_cursors();
                    continue;
                }
                Event::Mouse(mouse) => {
                    use crossterm::event::{MouseEventKind, MouseButton};
                    // Menu bar gets first crack at mouse events
                    if let Some(cmd) = app.menu_bar.handle_mouse(mouse) {
                        app.dispatch(cmd, &mut terminal).await;
                        continue;
                    }

                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            app.handle_scroll(-1);
                        }
                        MouseEventKind::ScrollDown => {
                            app.handle_scroll(1);
                        }
                        MouseEventKind::Down(MouseButton::Left) => {
                            let pos = ratatui::layout::Position::new(mouse.column, mouse.row);
                            app.handle_click(pos);
                        }
                        _ => {}
                    }
                }
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    // F10 / F9 toggle menu bar
                    if matches!(key.code, KeyCode::F(10) | KeyCode::F(9)) && !app.menu_bar.is_active() {
                        app.menu_bar.state.is_active = true;
                        app.menu_bar.state.is_dropped = false;
                        continue;
                    }

                    // Menu bar gets first crack at key events when active (or Alt+letter)
                    if app.menu_bar.is_active() || key.modifiers.contains(KeyModifiers::ALT) {
                        if let Some(cmd) = app.menu_bar.handle_key(key.code, key.modifiers) {
                            app.dispatch(cmd, &mut terminal).await;
                            continue;
                        }
                        if app.menu_bar.is_active() {
                            continue; // menu consumed the key (navigation)
                        }
                    }

                    // Screen-specific key handlers (unchanged)
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
                        Screen::Monitor { name } => {
                            let name = name.clone();
                            handle_monitor_keys(&mut app, key.code, &name);
                        }
                        Screen::InitConfig { name } => {
                            let name = name.clone();
                            handle_init_config_keys(&mut app, key, &name);
                        }
                        Screen::ConfigureTerminalFont => {
                            handle_configure_terminal_font_keys(&mut app, key.code);
                        }
                    }
                }
                _ => {}
            }
        }

        // Auto-refresh monitor screen every ~2 seconds (10 ticks × 200ms)
        if let Screen::Monitor { ref name } = app.screen {
            app.monitor_tick = app.monitor_tick.wrapping_add(1);
            if app.monitor_tick % 10 == 0 {
                let name = name.clone();
                app.refresh_monitor(&name);
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        stdout(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

// ── Key handlers ────────────────────────────────────────────────────────────

async fn handle_list_keys(
    app: &mut App,
    code: KeyCode,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) {
    crate::tui::screens::instance_list::handle_keys(app, code, terminal).await;
}

async fn handle_detail_keys(
    app: &mut App,
    code: KeyCode,
    name: &str,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) {
    crate::tui::screens::instance_detail::handle_keys(app, code, name, terminal).await;
}

fn handle_features_keys(app: &mut App, code: KeyCode, name: &str) {
    crate::tui::screens::edit_features::handle_keys(app, code, name);
}

fn handle_leader_keys(app: &mut App, code: KeyCode, name: &str) {
    crate::tui::screens::edit_leader::handle_keys(app, code, name);
}

fn handle_settings_keys(app: &mut App, code: KeyCode) {
    crate::tui::screens::settings::handle_keys(app, code);
}

fn handle_tutorial_list_keys(app: &mut App, code: KeyCode) {
    crate::tui::screens::tutorial::handle_list_keys(app, code);
}

fn handle_tutorial_view_keys(app: &mut App, code: KeyCode) {
    crate::tui::screens::tutorial::handle_view_keys(app, code);
}

async fn handle_marketplace_keys(app: &mut App, code: KeyCode, instance_name: &str) {
    crate::tui::screens::marketplace::handle_keys(app, code, instance_name).await;
}

fn handle_confirm_delete_keys(app: &mut App, code: KeyCode, name: &str) {
    crate::tui::screens::confirm_delete::handle_keys(app, code, name);
}

async fn handle_create_keys(
    app: &mut App,
    code: KeyCode,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) {
    crate::tui::screens::create::handle_keys(app, code, terminal).await;
}

fn handle_monitor_keys(app: &mut App, code: KeyCode, name: &str) {
    crate::tui::screens::monitor::handle_keys(app, code, name);
}

fn handle_init_config_keys(app: &mut App, key: crossterm::event::KeyEvent, name: &str) {
    crate::tui::screens::init_config::handle_keys(app, key, name);
}

// ── Operations ──────────────────────────────────────────────────────────────

pub(crate) fn toggle_bun_runtime(name: &str, app: &mut App) {
    let dir = config::instance_dir(&app.settings, name);
    let manifest_path = config::InstanceManifest::manifest_path(&dir);
    let mut manifest = match config::InstanceManifest::load(&manifest_path) {
        Ok(m) => m,
        Err(e) => {
            app.message = Some(format!("Failed to load manifest: {e}"));
            return;
        }
    };

    let currently_bun = manifest
        .js_runtime
        .as_deref()
        .is_some_and(|v| v.eq_ignore_ascii_case("bun"));

    if currently_bun {
        // Toggle off → revert to system Node
        manifest.js_runtime = None;
        manifest.updated_at = chrono::Utc::now();
        if let Err(e) = manifest.save(&manifest_path) {
            app.message = Some(format!("Failed to save: {e}"));
            return;
        }
        // Update cached instance
        if let Some(inst) = app.instances.iter_mut().find(|i| i.name == name) {
            inst.js_runtime = None;
        }
        app.message = Some(format!("Disabled Bun for '{name}'. Using system Node."));
    } else {
        // Validate bun is available before enabling
        if let Err(e) = crate::runtime::find_runtime_binary("bun") {
            app.message = Some(format!("Cannot enable Bun: {e}"));
            return;
        }
        manifest.js_runtime = Some("bun".to_string());
        manifest.updated_at = chrono::Utc::now();
        if let Err(e) = manifest.save(&manifest_path) {
            app.message = Some(format!("Failed to save: {e}"));
            return;
        }
        // Update cached instance
        if let Some(inst) = app.instances.iter_mut().find(|i| i.name == name) {
            inst.js_runtime = Some("bun".to_string());
        }
        app.message = Some(format!("Enabled Bun for '{name}'. Takes effect on next launch."));
    }

    // Regenerate init.lua so copilot/node_host_prog overrides are up to date
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
        app.message = Some(format!("Failed to regenerate init.lua: {e}"));
    }
}

pub(crate) fn open_instance_dir(name: &str, app: &mut App) {
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

fn handle_configure_terminal_font_keys(app: &mut App, code: KeyCode) {
    crate::tui::screens::terminal_font::handle_keys(app, code);
}

pub(crate) fn do_apply_terminal_font(app: &mut App) {
    let font_face = crate::font::NERD_FONT_FACE;
    let mut applied = Vec::new();
    let mut errors = Vec::new();
    let mut instructions = Vec::new();

    // Collect read-only terminals for manual instructions
    let read_only_kinds: std::collections::HashSet<_> = app
        .terminal_entries
        .iter()
        .filter(|e| e.read_only)
        .map(|e| e.install_label.clone())
        .collect();
    for label in &read_only_kinds {
        instructions.push(format!("{label}: manual configuration required"));
    }

    if app.terminal_apply_defaults {
        // Group by (kind, config_path) — apply defaults for each terminal installation
        let mut seen = std::collections::HashSet::new();
        for entry in &app.terminal_entries {
            if entry.read_only {
                continue;
            }
            let key = entry.config_path.clone();
            if seen.insert(key) {
                let install = crate::font::TerminalInstallation {
                    kind: entry.kind.clone(),
                    label: entry.install_label.clone(),
                    config_path: entry.config_path.clone(),
                    profiles: app
                        .terminal_entries
                        .iter()
                        .filter(|e| e.config_path == entry.config_path)
                        .map(|e| crate::font::TerminalProfile {
                            id: e.profile_id.clone(),
                            name: e.name.clone(),
                            current_font: e.current_font.clone(),
                        })
                        .collect(),
                    defaults_font: None,
                    supports_defaults: entry.supports_defaults,
                    read_only: false,
                };
                match crate::font::apply_terminal_font_to_defaults(&install, font_face) {
                    Ok(()) => applied.push(format!("{} defaults", entry.install_label)),
                    Err(e) => errors.push(format!("{}: {e}", entry.install_label)),
                }
            }
        }
    }

    // Apply to individually selected profiles, grouped by config path
    let selected: Vec<_> = app
        .terminal_entries
        .iter()
        .filter(|e| e.selected && !e.read_only)
        .cloned()
        .collect();

    if !selected.is_empty() {
        let mut by_path: std::collections::HashMap<std::path::PathBuf, Vec<_>> =
            std::collections::HashMap::new();
        for entry in &selected {
            by_path
                .entry(entry.config_path.clone())
                .or_default()
                .push(entry.clone());
        }

        for (path, entries) in &by_path {
            let first = &entries[0];
            let profile_ids: Vec<String> = entries.iter().map(|e| e.profile_id.clone()).collect();
            let install = crate::font::TerminalInstallation {
                kind: first.kind.clone(),
                label: first.install_label.clone(),
                config_path: path.clone(),
                profiles: entries
                    .iter()
                    .map(|e| crate::font::TerminalProfile {
                        id: e.profile_id.clone(),
                        name: e.name.clone(),
                        current_font: e.current_font.clone(),
                    })
                    .collect(),
                defaults_font: None,
                supports_defaults: first.supports_defaults,
                read_only: false,
            };
            match crate::font::apply_terminal_font_to_profiles(&install, font_face, &profile_ids) {
                Ok(()) => {
                    let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
                    applied.push(names.join(", "));
                }
                Err(e) => errors.push(format!("{}: {e}", first.install_label)),
            }
        }
    }

    let msg = if errors.is_empty() {
        let mut parts = Vec::new();
        if applied.is_empty() && instructions.is_empty() {
            return {
                app.screen = Screen::InstanceList;
                app.message =
                    Some("No changes applied. Select profiles or enable 'Apply to all'.".to_string());
            };
        }
        if !applied.is_empty() {
            parts.push(format!("✓ Configured font for: {}", applied.join("; ")));
        }
        if !instructions.is_empty() {
            parts.push(format!("ℹ {}", instructions.join("; ")));
        }
        parts.join("\n")
    } else {
        format!("Partially applied. Errors: {}", errors.join("; "))
    };

    app.screen = Screen::InstanceList;
    app.message = Some(msg);
}

pub(crate) async fn do_install_font(app: &mut App, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    leave_tui(terminal);
    let result = crate::font::install_nerd_font().await;
    println!("{}", result.message);
    println!("\nPress Enter to return to TUI...");
    let _ = std::io::stdin().read_line(&mut String::new());
    enter_tui(terminal);

    // After font install, check for terminal installations and offer configuration
    let installations = crate::font::find_terminals();
    if !installations.is_empty()
        && (result.installed_count > 0 || result.already_installed)
    {
        app.enter_configure_terminal_font(installations);
    } else {
        app.message = Some(result.message.clone());
    }
}

pub(crate) fn do_launch(name: &str, app: &mut App, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    let instance_dir = config::instance_dir(&app.settings, name);
    leave_tui(terminal);

    // Resolve JS runtime shim
    let manifest_path = config::InstanceManifest::manifest_path(&instance_dir);
    let js_runtime_path = config::InstanceManifest::load(&manifest_path)
        .ok()
        .and_then(|m| {
            crate::runtime::setup_runtime_shims(&instance_dir, &m, &app.settings)
                .map_err(|e| eprintln!("Warning: runtime shim setup failed: {e}"))
                .ok()
                .flatten()
        });

    match crate::neovim::launch(&instance_dir, name, &[], js_runtime_path) {
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

pub(crate) async fn do_update(
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


