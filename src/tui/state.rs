use ratatui::layout::Rect;

use super::command::Command;

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
    Monitor { name: String },
    InitConfig { name: String },
    ConfigureTerminalFont,
}

// ── Feature editing types ───────────────────────────────────────────────────

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

// ── Terminal font configuration types ───────────────────────────────────────

/// A terminal profile entry shown in the ConfigureTerminalFont screen.
#[derive(Clone, Debug)]
pub struct TerminalProfileEntry {
    pub kind: crate::font::TerminalKind,
    pub install_label: String,
    pub config_path: std::path::PathBuf,
    pub profile_id: String,
    pub name: String,
    pub current_font: Option<String>,
    pub selected: bool,
    pub supports_defaults: bool,
    pub read_only: bool,
}

// ── Layout cache for mouse hit testing ──────────────────────────────────────

/// Stores widget positions from the last draw pass so mouse clicks
/// can be mapped to UI elements.
#[derive(Default)]
pub struct LayoutCache {
    /// Clickable list rows: (rect, item_index)
    pub list_items: Vec<(Rect, usize)>,
    /// Clickable action buttons: (rect, command)
    pub action_buttons: Vec<(Rect, Command)>,
}

impl LayoutCache {
    pub fn reset(&mut self) {
        self.list_items.clear();
        self.action_buttons.clear();
    }
}
