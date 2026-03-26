use std::fs;
use std::path::{Path, PathBuf};

use super::terminal::{TerminalInstallation, TerminalKind, TerminalProfile};

/// Detect Konsole by checking for profile files.
pub fn detect() -> Vec<TerminalInstallation> {
    let profile_dir = profile_dir();
    let Some(profile_dir) = profile_dir else {
        return Vec::new();
    };

    if !profile_dir.exists() {
        return Vec::new();
    }

    let Ok(entries) = fs::read_dir(&profile_dir) else {
        return Vec::new();
    };

    let mut profiles = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("profile") {
            continue;
        }

        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "(unnamed)".to_string());

        let current_font = read_font(&path);

        profiles.push(TerminalProfile {
            id: path.to_string_lossy().to_string(),
            name,
            current_font,
        });
    }

    if profiles.is_empty() {
        return Vec::new();
    }

    vec![TerminalInstallation {
        kind: TerminalKind::Konsole,
        label: "Konsole".to_string(),
        config_path: profile_dir,
        profiles,
        defaults_font: None,
        supports_defaults: false,
        read_only: false,
    }]
}

/// Apply font to a Konsole profile file.
/// Creates a `.pnm-backup` before modifying.
pub fn apply_font(
    profile_path: &Path,
    font_face: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(profile_path)?;

    // Backup
    let backup_path = profile_path.with_extension("profile.pnm-backup");
    if !backup_path.exists() {
        fs::copy(profile_path, &backup_path)?;
    }

    // Konsole Font= format: "FontName,Size,-1,5,50,0,0,0,0,0"
    // Minimal working format: "FontName,12,-1,5,50,0,0,0,0,0"
    let font_value = format!("{font_face},12,-1,5,50,0,0,0,0,0");

    let mut new_lines = Vec::new();
    let mut found = false;
    let mut in_appearance = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') {
            in_appearance = trimmed == "[Appearance]";
        }

        if in_appearance && trimmed.starts_with("Font=") {
            new_lines.push(format!("Font={font_value}"));
            found = true;
        } else {
            new_lines.push(line.to_string());
        }
    }

    if !found {
        // Add [Appearance] section if not present
        let has_appearance = content.lines().any(|l| l.trim() == "[Appearance]");
        if has_appearance {
            // Insert Font= after [Appearance]
            let mut result = Vec::new();
            for line in &new_lines {
                result.push(line.clone());
                if line.trim() == "[Appearance]" {
                    result.push(format!("Font={font_value}"));
                }
            }
            new_lines = result;
        } else {
            new_lines.push(String::new());
            new_lines.push("[Appearance]".to_string());
            new_lines.push(format!("Font={font_value}"));
        }
    }

    let mut output = new_lines.join("\n");
    if content.ends_with('\n') && !output.ends_with('\n') {
        output.push('\n');
    }

    fs::write(profile_path, output)?;
    Ok(())
}

/// Read the current font from a Konsole profile file.
fn read_font(profile_path: &Path) -> Option<String> {
    let content = fs::read_to_string(profile_path).ok()?;
    let mut in_appearance = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_appearance = trimmed == "[Appearance]";
        }
        if in_appearance && trimmed.starts_with("Font=") {
            let value = trimmed.strip_prefix("Font=")?;
            // Extract font name (first comma-separated field)
            let font_name = value.split(',').next()?.trim();
            if !font_name.is_empty() {
                return Some(font_name.to_string());
            }
        }
    }
    None
}

fn profile_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("konsole"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_font() {
        let tmp = std::env::temp_dir().join("pnm_test_konsole_read");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let profile = tmp.join("Test.profile");
        fs::write(
            &profile,
            "[General]\nName=Test\n\n[Appearance]\nFont=Cascadia Mono,12,-1,5,50,0,0,0,0,0\n",
        )
        .unwrap();

        assert_eq!(read_font(&profile), Some("Cascadia Mono".to_string()));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_read_font_no_appearance() {
        let tmp = std::env::temp_dir().join("pnm_test_konsole_no_appearance");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let profile = tmp.join("Test.profile");
        fs::write(&profile, "[General]\nName=Test\n").unwrap();

        assert_eq!(read_font(&profile), None);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_apply_font_replace() {
        let tmp = std::env::temp_dir().join("pnm_test_konsole_apply");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let profile = tmp.join("Test.profile");
        fs::write(
            &profile,
            "[General]\nName=Test\n\n[Appearance]\nFont=Cascadia Mono,12,-1,5,50,0,0,0,0,0\n",
        )
        .unwrap();

        apply_font(&profile, "JetBrainsMono Nerd Font").unwrap();

        assert_eq!(
            read_font(&profile),
            Some("JetBrainsMono Nerd Font".to_string())
        );
        assert!(tmp.join("Test.profile.pnm-backup").exists());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_apply_font_create_section() {
        let tmp = std::env::temp_dir().join("pnm_test_konsole_create");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let profile = tmp.join("Test.profile");
        fs::write(&profile, "[General]\nName=Test\n").unwrap();

        apply_font(&profile, "JetBrainsMono Nerd Font").unwrap();

        let content = fs::read_to_string(&profile).unwrap();
        assert!(content.contains("[Appearance]"));
        assert!(content.contains("Font=JetBrainsMono Nerd Font,12"));

        let _ = fs::remove_dir_all(&tmp);
    }
}
