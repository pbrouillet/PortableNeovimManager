use crate::tui::command::Command;

// ── Menu tree types ─────────────────────────────────────────────────────────

pub struct Menu {
    pub label: String,
    pub hotkey: char,
    pub items: Vec<MenuItem>,
}

pub enum MenuItem {
    Action {
        label: String,
        hotkey: char,
        shortcut_display: Option<String>,
        command: Command,
    },
    Separator,
}

// ── Menu definitions ────────────────────────────────────────────────────────

pub fn build_menu_bar() -> Vec<Menu> {
    vec![
        Menu {
            label: "Instance".into(),
            hotkey: 'I',
            items: vec![
                MenuItem::Action {
                    label: "Create".into(),
                    hotkey: 'C',
                    shortcut_display: Some("Ctrl+N".into()),
                    command: Command::CreateInstance,
                },
                MenuItem::Action {
                    label: "Launch".into(),
                    hotkey: 'L',
                    shortcut_display: Some("F2".into()),
                    command: Command::LaunchInstance,
                },
                MenuItem::Action {
                    label: "Update".into(),
                    hotkey: 'U',
                    shortcut_display: None,
                    command: Command::UpdateInstance,
                },
                MenuItem::Separator,
                MenuItem::Action {
                    label: "Features".into(),
                    hotkey: 'F',
                    shortcut_display: Some("F6".into()),
                    command: Command::EditFeatures,
                },
                MenuItem::Action {
                    label: "Leader Key".into(),
                    hotkey: 'K',
                    shortcut_display: None,
                    command: Command::EditLeaderKey,
                },
                MenuItem::Action {
                    label: "Monitor".into(),
                    hotkey: 'M',
                    shortcut_display: None,
                    command: Command::OpenMonitor,
                },
                MenuItem::Action {
                    label: "Init Config".into(),
                    hotkey: 'I',
                    shortcut_display: None,
                    command: Command::OpenInitConfig,
                },
                MenuItem::Separator,
                MenuItem::Action {
                    label: "Delete".into(),
                    hotkey: 'D',
                    shortcut_display: Some("F8".into()),
                    command: Command::DeleteInstance,
                },
            ],
        },
        Menu {
            label: "Actions".into(),
            hotkey: 'A',
            items: vec![
                MenuItem::Action {
                    label: "Marketplace".into(),
                    hotkey: 'M',
                    shortcut_display: None,
                    command: Command::OpenMarketplace,
                },
                MenuItem::Action {
                    label: "Install Nerd Font".into(),
                    hotkey: 'N',
                    shortcut_display: None,
                    command: Command::InstallNerdFont,
                },
                MenuItem::Action {
                    label: "Terminal Font".into(),
                    hotkey: 'T',
                    shortcut_display: None,
                    command: Command::OpenTerminalFont,
                },
                MenuItem::Separator,
                MenuItem::Action {
                    label: "Refresh".into(),
                    hotkey: 'R',
                    shortcut_display: Some("Ctrl+R".into()),
                    command: Command::Refresh,
                },
            ],
        },
        Menu {
            label: "Settings".into(),
            hotkey: 'S',
            items: vec![
                MenuItem::Action {
                    label: "Edit Settings".into(),
                    hotkey: 'E',
                    shortcut_display: None,
                    command: Command::OpenSettings,
                },
            ],
        },
        Menu {
            label: "Help".into(),
            hotkey: 'H',
            items: vec![
                MenuItem::Action {
                    label: "Tutorials".into(),
                    hotkey: 'T',
                    shortcut_display: Some("F1".into()),
                    command: Command::OpenTutorials,
                },
            ],
        },
    ]
}
