use crossterm::event::{KeyCode, KeyModifiers};

use super::state::Screen;

/// Every action the TUI can perform, triggered by menu items, keyboard
/// shortcuts, function keys, or mouse clicks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Command {
    // Navigation
    Quit,
    Back,
    FocusUp,
    FocusDown,
    PageUp,
    PageDown,
    Select,

    // Instance operations
    CreateInstance,
    LaunchInstance,
    UpdateInstance,
    DeleteInstance,
    EditFeatures,
    EditLeaderKey,

    // Views
    OpenSettings,
    OpenTutorials,
    OpenMarketplace,
    OpenMonitor,
    OpenInitConfig,
    OpenTerminalFont,
    InstallNerdFont,
    OpenInstanceDir,
    ToggleBunRuntime,

    // Editing
    ToggleItem,
    ExpandItem,
    Search,
    Refresh,
    Save,
    Cancel,

    // Menu bar
    ActivateMenuBar,
}

#[derive(Clone, Debug)]
struct Binding {
    code: KeyCode,
    modifiers: KeyModifiers,
    command: Command,
}

/// Maps key events to commands, with global and per-screen layers.
pub struct KeyMap {
    global: Vec<Binding>,
    per_screen: Vec<(ScreenId, Vec<Binding>)>,
}

/// Lightweight screen discriminant for keymap lookup (avoids cloning Screen).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenId {
    InstanceList,
    InstanceDetail,
    EditFeatures,
    EditLeaderKey,
    ConfirmDelete,
    EditSettings,
    TutorialList,
    TutorialView,
    Marketplace,
    CreateInstance,
    Monitor,
    InitConfig,
    ConfigureTerminalFont,
}

impl From<&Screen> for ScreenId {
    fn from(screen: &Screen) -> Self {
        match screen {
            Screen::InstanceList => ScreenId::InstanceList,
            Screen::InstanceDetail { .. } => ScreenId::InstanceDetail,
            Screen::EditFeatures { .. } => ScreenId::EditFeatures,
            Screen::EditLeaderKey { .. } => ScreenId::EditLeaderKey,
            Screen::ConfirmDelete { .. } => ScreenId::ConfirmDelete,
            Screen::EditSettings => ScreenId::EditSettings,
            Screen::TutorialList => ScreenId::TutorialList,
            Screen::TutorialView { .. } => ScreenId::TutorialView,
            Screen::Marketplace { .. } => ScreenId::Marketplace,
            Screen::CreateInstance => ScreenId::CreateInstance,
            Screen::Monitor { .. } => ScreenId::Monitor,
            Screen::InitConfig { .. } => ScreenId::InitConfig,
            Screen::ConfigureTerminalFont => ScreenId::ConfigureTerminalFont,
        }
    }
}

impl KeyMap {
    pub fn new() -> Self {
        let mut km = Self {
            global: Vec::new(),
            per_screen: Vec::new(),
        };
        km.build_global();
        km.build_per_screen();
        km
    }

    /// Try to resolve a key press into a command. Checks per-screen bindings
    /// first (more specific), then global bindings.
    pub fn resolve(&self, code: KeyCode, modifiers: KeyModifiers, screen: &Screen) -> Option<Command> {
        let screen_id = ScreenId::from(screen);

        // Per-screen bindings take priority
        for (sid, bindings) in &self.per_screen {
            if *sid == screen_id {
                for b in bindings {
                    if b.code == code && b.modifiers == modifiers {
                        return Some(b.command);
                    }
                }
                break;
            }
        }

        // Global bindings
        for b in &self.global {
            if b.code == code && b.modifiers == modifiers {
                return Some(b.command);
            }
        }

        None
    }

    fn bind(code: KeyCode, modifiers: KeyModifiers, command: Command) -> Binding {
        Binding { code, modifiers, command }
    }

    fn build_global(&mut self) {
        let none = KeyModifiers::NONE;
        let ctrl = KeyModifiers::CONTROL;

        self.global = vec![
            Self::bind(KeyCode::Char('q'), ctrl, Command::Quit),
            Self::bind(KeyCode::F(1), none, Command::OpenTutorials),
            Self::bind(KeyCode::F(10), none, Command::ActivateMenuBar),
            Self::bind(KeyCode::F(9), none, Command::ActivateMenuBar),
            Self::bind(KeyCode::Up, none, Command::FocusUp),
            Self::bind(KeyCode::Down, none, Command::FocusDown),
            Self::bind(KeyCode::PageUp, none, Command::PageUp),
            Self::bind(KeyCode::PageDown, none, Command::PageDown),
            Self::bind(KeyCode::Enter, none, Command::Select),
            Self::bind(KeyCode::Esc, none, Command::Back),
        ];
    }

    fn build_per_screen(&mut self) {
        let none = KeyModifiers::NONE;
        let ctrl = KeyModifiers::CONTROL;

        // InstanceList — matches current single-letter shortcuts
        self.per_screen.push((ScreenId::InstanceList, vec![
            Self::bind(KeyCode::Char('q'), none, Command::Quit),
            Self::bind(KeyCode::Char('j'), none, Command::FocusDown),
            Self::bind(KeyCode::Char('k'), none, Command::FocusUp),
            Self::bind(KeyCode::Char('c'), none, Command::CreateInstance),
            Self::bind(KeyCode::Char('r'), none, Command::Refresh),
            Self::bind(KeyCode::Char('n'), none, Command::InstallNerdFont),
            Self::bind(KeyCode::Char('s'), none, Command::OpenSettings),
            Self::bind(KeyCode::Char('t'), none, Command::OpenTutorials),
            Self::bind(KeyCode::Char('/'), none, Command::Search),
            Self::bind(KeyCode::F(2), none, Command::LaunchInstance),
            Self::bind(KeyCode::F(5), none, Command::CreateInstance),
            Self::bind(KeyCode::F(6), none, Command::EditFeatures),
            Self::bind(KeyCode::F(8), none, Command::DeleteInstance),
            Self::bind(KeyCode::Char('n'), ctrl, Command::CreateInstance),
        ]));

        // InstanceDetail
        self.per_screen.push((ScreenId::InstanceDetail, vec![
            Self::bind(KeyCode::Char('q'), none, Command::Back),
            Self::bind(KeyCode::Char('l'), none, Command::LaunchInstance),
            Self::bind(KeyCode::Char('u'), none, Command::UpdateInstance),
            Self::bind(KeyCode::Char('d'), none, Command::DeleteInstance),
            Self::bind(KeyCode::Char('f'), none, Command::EditFeatures),
            Self::bind(KeyCode::Char('m'), none, Command::EditLeaderKey),
            Self::bind(KeyCode::Char('o'), none, Command::OpenInstanceDir),
            Self::bind(KeyCode::Char('t'), none, Command::OpenTutorials),
            Self::bind(KeyCode::Char('p'), none, Command::OpenMarketplace),
            Self::bind(KeyCode::Char('M'), none, Command::OpenMonitor),
            Self::bind(KeyCode::Char('B'), none, Command::ToggleBunRuntime),
            Self::bind(KeyCode::Char('I'), none, Command::OpenInitConfig),
            Self::bind(KeyCode::F(2), none, Command::LaunchInstance),
            Self::bind(KeyCode::F(6), none, Command::EditFeatures),
            Self::bind(KeyCode::F(8), none, Command::DeleteInstance),
        ]));

        // EditFeatures
        self.per_screen.push((ScreenId::EditFeatures, vec![
            Self::bind(KeyCode::Char('q'), none, Command::Back),
            Self::bind(KeyCode::Char('j'), none, Command::FocusDown),
            Self::bind(KeyCode::Char('k'), none, Command::FocusUp),
            Self::bind(KeyCode::Char(' '), none, Command::ToggleItem),
            Self::bind(KeyCode::Tab, none, Command::ExpandItem),
            Self::bind(KeyCode::Char('s'), none, Command::Save),
            Self::bind(KeyCode::F(2), none, Command::Save),
        ]));

        // EditLeaderKey
        self.per_screen.push((ScreenId::EditLeaderKey, vec![
            Self::bind(KeyCode::Char('q'), none, Command::Back),
            Self::bind(KeyCode::Char('j'), none, Command::FocusDown),
            Self::bind(KeyCode::Char('k'), none, Command::FocusUp),
        ]));

        // EditSettings
        self.per_screen.push((ScreenId::EditSettings, vec![
            Self::bind(KeyCode::Char('q'), none, Command::Back),
            Self::bind(KeyCode::Char('j'), none, Command::FocusDown),
            Self::bind(KeyCode::Char('k'), none, Command::FocusUp),
        ]));

        // TutorialList
        self.per_screen.push((ScreenId::TutorialList, vec![
            Self::bind(KeyCode::Char('q'), none, Command::Back),
            Self::bind(KeyCode::Char('j'), none, Command::FocusDown),
            Self::bind(KeyCode::Char('k'), none, Command::FocusUp),
            Self::bind(KeyCode::Char('/'), none, Command::Search),
        ]));

        // TutorialView
        self.per_screen.push((ScreenId::TutorialView, vec![
            Self::bind(KeyCode::Char('q'), none, Command::Back),
            Self::bind(KeyCode::Char('j'), none, Command::FocusDown),
            Self::bind(KeyCode::Char('k'), none, Command::FocusUp),
        ]));

        // Marketplace
        self.per_screen.push((ScreenId::Marketplace, vec![
            Self::bind(KeyCode::Char('q'), none, Command::Back),
            Self::bind(KeyCode::Char('j'), none, Command::FocusDown),
            Self::bind(KeyCode::Char('k'), none, Command::FocusUp),
            Self::bind(KeyCode::Char(' '), none, Command::ToggleItem),
            Self::bind(KeyCode::Char('s'), none, Command::Save),
            Self::bind(KeyCode::Char('r'), none, Command::Refresh),
            Self::bind(KeyCode::Char('/'), none, Command::Search),
        ]));

        // CreateInstance
        self.per_screen.push((ScreenId::CreateInstance, vec![
            Self::bind(KeyCode::Char('q'), none, Command::Back),
        ]));

        // Monitor
        self.per_screen.push((ScreenId::Monitor, vec![
            Self::bind(KeyCode::Char('q'), none, Command::Back),
            Self::bind(KeyCode::Char('r'), none, Command::Refresh),
        ]));

        // ConfirmDelete
        self.per_screen.push((ScreenId::ConfirmDelete, vec![
            Self::bind(KeyCode::Char('q'), none, Command::Back),
        ]));
    }
}
