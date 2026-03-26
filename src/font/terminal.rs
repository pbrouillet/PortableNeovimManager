use std::path::PathBuf;

/// Identifies which terminal emulator a [`TerminalInstallation`] represents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalKind {
    WindowsTerminal,
    Alacritty,
    Kitty,
    GnomeTerminal,
    ITerm2,
    Konsole,
    WezTerm,
}

impl std::fmt::Display for TerminalKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WindowsTerminal => write!(f, "Windows Terminal"),
            Self::Alacritty => write!(f, "Alacritty"),
            Self::Kitty => write!(f, "Kitty"),
            Self::GnomeTerminal => write!(f, "GNOME Terminal"),
            Self::ITerm2 => write!(f, "iTerm2"),
            Self::Konsole => write!(f, "Konsole"),
            Self::WezTerm => write!(f, "WezTerm"),
        }
    }
}

/// A single profile within a terminal emulator (e.g. a WT profile, GNOME
/// Terminal profile, Konsole profile file).
#[derive(Clone, Debug)]
pub struct TerminalProfile {
    /// Opaque identifier (GUID for WT, UUID for GNOME, filename for Konsole, etc.)
    pub id: String,
    /// Human-readable display name
    pub name: String,
    /// Currently configured font face, if any
    pub current_font: Option<String>,
}

/// A discovered terminal emulator installation that can (potentially) be
/// configured to use the Nerd Font.
#[derive(Clone, Debug)]
pub struct TerminalInstallation {
    pub kind: TerminalKind,
    /// Human-readable label (e.g. "Windows Terminal", "Windows Terminal Preview")
    pub label: String,
    /// Path to the configuration file (for display and backup purposes)
    pub config_path: PathBuf,
    /// Individual profiles within this terminal
    pub profiles: Vec<TerminalProfile>,
    /// Font currently configured in the terminal's "defaults" / global section
    pub defaults_font: Option<String>,
    /// Whether this terminal supports a global default font setting
    pub supports_defaults: bool,
    /// If true, we can only print manual instructions (not auto-configure)
    pub read_only: bool,
}

// ── Discovery ───────────────────────────────────────────────────────────────

/// Discover all configurable terminal emulators on the current platform.
pub fn find_terminals() -> Vec<TerminalInstallation> {
    let mut terminals = Vec::new();

    #[cfg(target_os = "windows")]
    {
        terminals.extend(super::wt::detect());
    }

    terminals.extend(super::alacritty::detect());

    #[cfg(not(target_os = "windows"))]
    {
        terminals.extend(super::kitty::detect());
    }

    #[cfg(target_os = "linux")]
    {
        terminals.extend(super::gnome_terminal::detect());
        terminals.extend(super::konsole::detect());
    }

    #[cfg(target_os = "macos")]
    {
        terminals.extend(super::iterm2::detect());
    }

    terminals
}

// ── Generic apply ───────────────────────────────────────────────────────────

/// Apply the font to a terminal's global defaults.
pub fn apply_font_to_defaults(
    install: &TerminalInstallation,
    font_face: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if install.read_only {
        return Err(format!(
            "{} does not support automatic font configuration. Please set it manually.",
            install.label
        )
        .into());
    }
    match install.kind {
        #[cfg(target_os = "windows")]
        TerminalKind::WindowsTerminal => {
            super::wt::apply_font_to_wt_defaults(&install.config_path, font_face)?;
        }
        TerminalKind::Alacritty => {
            super::alacritty::apply_font(&install.config_path, font_face)?;
        }
        #[cfg(not(target_os = "windows"))]
        TerminalKind::Kitty => {
            super::kitty::apply_font(&install.config_path, font_face)?;
        }
        #[cfg(target_os = "linux")]
        TerminalKind::GnomeTerminal => {
            // GNOME Terminal doesn't have a global default — apply to all profiles
            for profile in &install.profiles {
                super::gnome_terminal::apply_font_to_profile(&profile.id, font_face)?;
            }
        }
        #[cfg(target_os = "linux")]
        TerminalKind::Konsole => {
            for profile in &install.profiles {
                super::konsole::apply_font(&std::path::PathBuf::from(&profile.id), font_face)?;
            }
        }
        #[cfg(target_os = "macos")]
        TerminalKind::ITerm2 => {
            for profile in &install.profiles {
                super::iterm2::apply_font_to_profile(&profile.name, font_face)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Apply the font to specific profiles within a terminal installation.
pub fn apply_font_to_profiles(
    install: &TerminalInstallation,
    font_face: &str,
    profile_ids: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if install.read_only || profile_ids.is_empty() {
        return Ok(());
    }
    match install.kind {
        #[cfg(target_os = "windows")]
        TerminalKind::WindowsTerminal => {
            super::wt::apply_font_to_wt_profiles(&install.config_path, font_face, profile_ids)?;
        }
        TerminalKind::Alacritty => {
            // Alacritty has no profiles — apply globally
            super::alacritty::apply_font(&install.config_path, font_face)?;
        }
        #[cfg(not(target_os = "windows"))]
        TerminalKind::Kitty => {
            super::kitty::apply_font(&install.config_path, font_face)?;
        }
        #[cfg(target_os = "linux")]
        TerminalKind::GnomeTerminal => {
            for pid in profile_ids {
                super::gnome_terminal::apply_font_to_profile(pid, font_face)?;
            }
        }
        #[cfg(target_os = "linux")]
        TerminalKind::Konsole => {
            for pid in profile_ids {
                super::konsole::apply_font(&std::path::PathBuf::from(pid), font_face)?;
            }
        }
        #[cfg(target_os = "macos")]
        TerminalKind::ITerm2 => {
            for pid in profile_ids {
                super::iterm2::apply_font_to_profile(pid, font_face)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Check if any detected terminal already has the font configured.
pub fn is_any_terminal_configured(font_face: &str) -> bool {
    for install in find_terminals() {
        if install.defaults_font.as_deref() == Some(font_face) {
            return true;
        }
        if install
            .profiles
            .iter()
            .any(|p| p.current_font.as_deref() == Some(font_face))
        {
            return true;
        }
    }
    false
}

/// Return manual configuration instructions for read-only terminals.
pub fn manual_instructions(kind: &TerminalKind, font_face: &str) -> Option<String> {
    match kind {
        TerminalKind::WezTerm => Some(format!(
            "Add to your ~/.wezterm.lua:\n  config.font = wezterm.font('{font_face}')"
        )),
        _ => None,
    }
}
