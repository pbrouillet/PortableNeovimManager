use std::path::PathBuf;
use std::process::Command;

use super::terminal::{TerminalInstallation, TerminalKind, TerminalProfile};

const ITERM2_PLIST: &str = "com.googlecode.iterm2";
const PLIST_BUDDY: &str = "/usr/libexec/PlistBuddy";

/// Detect iTerm2 by checking for the app bundle and plist.
pub fn detect() -> Vec<TerminalInstallation> {
    let app_path = PathBuf::from("/Applications/iTerm.app");
    if !app_path.exists() {
        return Vec::new();
    }

    let plist_path = plist_path();
    let Some(plist_path) = plist_path else {
        return Vec::new();
    };

    if !plist_path.exists() {
        return Vec::new();
    }

    let profiles = read_profiles(&plist_path);

    vec![TerminalInstallation {
        kind: TerminalKind::ITerm2,
        label: "iTerm2".to_string(),
        config_path: plist_path,
        profiles,
        defaults_font: None,
        supports_defaults: false,
        read_only: false,
    }]
}

/// Apply font to a specific iTerm2 profile by name.
pub fn apply_font_to_profile(
    profile_name: &str,
    font_face: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(plist_path) = plist_path() else {
        return Err("Could not determine iTerm2 plist path".into());
    };

    // Find the profile index
    let profiles = read_profiles(&plist_path);
    let Some(index) = profiles.iter().position(|p| p.name == profile_name) else {
        return Err(format!("Profile '{}' not found", profile_name).into());
    };

    // Font name in iTerm2 format: "FontName Size"
    // Normal Font entry path
    let font_entry = format!(
        ":New Bookmarks:{index}:Normal Font"
    );

    // iTerm2 stores font as "FontName Size" — use size 12 as default
    let font_value = format!("{font_face} 12");

    let status = Command::new(PLIST_BUDDY)
        .args([
            "-c",
            &format!("Set {font_entry} {font_value}"),
            plist_path.to_str().unwrap_or(""),
        ])
        .status()?;

    if status.success() {
        Ok(())
    } else {
        // Try Add if Set fails (entry might not exist)
        let status = Command::new(PLIST_BUDDY)
            .args([
                "-c",
                &format!("Add {font_entry} string {font_value}"),
                plist_path.to_str().unwrap_or(""),
            ])
            .status()?;

        if status.success() {
            Ok(())
        } else {
            Err("Failed to set font in iTerm2 plist".into())
        }
    }
}

fn plist_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| {
        h.join("Library")
            .join("Preferences")
            .join(format!("{ITERM2_PLIST}.plist"))
    })
}

fn read_profiles(plist_path: &PathBuf) -> Vec<TerminalProfile> {
    let mut profiles = Vec::new();

    // Use PlistBuddy to read profile count and names
    let output = Command::new(PLIST_BUDDY)
        .args([
            "-c",
            "Print :New Bookmarks",
            plist_path.to_str().unwrap_or(""),
        ])
        .output();

    let Ok(output) = output else {
        return profiles;
    };

    if !output.status.success() {
        return profiles;
    }

    let text = String::from_utf8_lossy(&output.stdout);

    // Parse PlistBuddy output to find profile names and fonts
    // The output is an array of dicts — we look for Name and Normal Font
    let mut current_name: Option<String> = None;
    let mut current_font: Option<String> = None;
    let mut index = 0;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Name = ") {
            current_name = Some(trimmed.strip_prefix("Name = ").unwrap().to_string());
        } else if trimmed.starts_with("Normal Font = ") {
            current_font = Some(
                trimmed
                    .strip_prefix("Normal Font = ")
                    .unwrap()
                    .to_string(),
            );
        } else if trimmed == "}" {
            // End of a profile dict
            if let Some(name) = current_name.take() {
                profiles.push(TerminalProfile {
                    id: index.to_string(),
                    name,
                    current_font: current_font.take(),
                });
                index += 1;
            }
            current_font = None;
        }
    }

    profiles
}
