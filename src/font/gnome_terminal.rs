use std::path::PathBuf;
use std::process::Command;

use super::terminal::{TerminalInstallation, TerminalKind, TerminalProfile};

const SCHEMA_BASE: &str = "org.gnome.Terminal.Legacy.Profile:";
const SCHEMA_LIST: &str = "org.gnome.Terminal.ProfilesList";
const DCONF_PREFIX: &str = "/org/gnome/terminal/legacy/profiles:/";

/// Detect GNOME Terminal by checking for gsettings and the schema.
pub fn detect() -> Vec<TerminalInstallation> {
    if !has_gsettings() || !has_schema() {
        return Vec::new();
    }

    let profile_uuids = list_profile_uuids();
    if profile_uuids.is_empty() {
        return Vec::new();
    }

    let profiles: Vec<TerminalProfile> = profile_uuids
        .iter()
        .map(|uuid| {
            let name = get_profile_name(uuid).unwrap_or_else(|| uuid.clone());
            let current_font = get_profile_font(uuid);
            TerminalProfile {
                id: uuid.clone(),
                name,
                current_font,
            }
        })
        .collect();

    vec![TerminalInstallation {
        kind: TerminalKind::GnomeTerminal,
        label: "GNOME Terminal".to_string(),
        config_path: PathBuf::from(DCONF_PREFIX),
        profiles,
        defaults_font: None,
        supports_defaults: false,
        read_only: false,
    }]
}

/// Apply font to a specific GNOME Terminal profile by UUID.
pub fn apply_font_to_profile(
    uuid: &str,
    font_face: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = format!("{DCONF_PREFIX}:{uuid}/");
    let font_value = format!("{font_face} 12");

    // Disable system font first
    let status = Command::new("gsettings")
        .args(["set", SCHEMA_BASE, "use-system-font", "false"])
        .env("GSETTINGS_SCHEMA_DIR", "")
        .arg(format!("--schemadir={path}"))
        .status();

    // Try the dconf path approach instead (more reliable)
    let _ = Command::new("dconf")
        .args(["write", &format!("{DCONF_PREFIX}:{uuid}/use-system-font"), "false"])
        .status();

    let result = Command::new("dconf")
        .args(["write", &format!("{DCONF_PREFIX}:{uuid}/font"), &format!("'{font_value}'")])
        .status();

    match result {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => {
            // Fallback to gsettings
            let gs = Command::new("gsettings")
                .args(["set", &format!("{SCHEMA_BASE}:{path}"), "font", &font_value])
                .status()?;
            if gs.success() {
                Ok(())
            } else {
                Err(format!("gsettings exited with status {gs}").into())
            }
        }
        Err(e) => {
            // dconf not available, try gsettings
            let _ = status; // consume the earlier gsettings result
            Err(format!("Failed to run dconf: {e}").into())
        }
    }
}

fn has_gsettings() -> bool {
    Command::new("gsettings")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn has_schema() -> bool {
    Command::new("gsettings")
        .args(["list-schemas"])
        .output()
        .is_ok_and(|o| {
            String::from_utf8_lossy(&o.stdout).contains(SCHEMA_LIST)
        })
}

fn list_profile_uuids() -> Vec<String> {
    let output = Command::new("gsettings")
        .args(["get", SCHEMA_LIST, "list"])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    // Output format: ['uuid1', 'uuid2']
    let text = String::from_utf8_lossy(&output.stdout);
    text.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .filter_map(|s| {
            let trimmed = s.trim().trim_matches('\'').trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

fn get_profile_name(uuid: &str) -> Option<String> {
    let output = Command::new("gsettings")
        .args([
            "get",
            &format!("{SCHEMA_BASE}:{DCONF_PREFIX}:{uuid}/"),
            "visible-name",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let name = text.trim().trim_matches('\'').to_string();
    if name.is_empty() { None } else { Some(name) }
}

fn get_profile_font(uuid: &str) -> Option<String> {
    let output = Command::new("gsettings")
        .args([
            "get",
            &format!("{SCHEMA_BASE}:{DCONF_PREFIX}:{uuid}/"),
            "font",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let font = text.trim().trim_matches('\'').to_string();
    if font.is_empty() { None } else { Some(font) }
}
