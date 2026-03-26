use std::fs;
use std::path::{Path, PathBuf};

use super::terminal::{TerminalInstallation, TerminalKind, TerminalProfile};

/// Detect Kitty installations by checking the config file.
pub fn detect() -> Vec<TerminalInstallation> {
    let mut results = Vec::new();

    for path in config_paths() {
        if path.exists() {
            let current_font = read_font_family(&path);
            results.push(TerminalInstallation {
                kind: TerminalKind::Kitty,
                label: "Kitty".to_string(),
                config_path: path.clone(),
                profiles: vec![TerminalProfile {
                    id: "default".to_string(),
                    name: "Default".to_string(),
                    current_font: current_font.clone(),
                }],
                defaults_font: current_font,
                supports_defaults: true,
                read_only: false,
            });
            break;
        }
    }

    results
}

/// Apply the font to a Kitty configuration file.
/// Creates a `.pnm-backup` before modifying.
pub fn apply_font(config_path: &Path, font_face: &str) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(config_path)?;

    // Backup
    let backup_path = config_path.with_extension("conf.pnm-backup");
    if !backup_path.exists() {
        fs::copy(config_path, &backup_path)?;
    }

    let mut new_lines = Vec::new();
    let mut found = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("font_family") && !trimmed.starts_with("font_family_") {
            // Replace existing font_family line (but not font_family_bold etc.)
            new_lines.push(format!("font_family {font_face}"));
            found = true;
        } else {
            new_lines.push(line.to_string());
        }
    }

    if !found {
        // Append font_family if not present
        new_lines.push(String::new());
        new_lines.push(format!("# Added by pnm"));
        new_lines.push(format!("font_family {font_face}"));
    }

    // Preserve trailing newline
    let mut output = new_lines.join("\n");
    if content.ends_with('\n') && !output.ends_with('\n') {
        output.push('\n');
    }

    fs::write(config_path, output)?;
    Ok(())
}

/// Read the current font_family from a Kitty config.
fn read_font_family(config_path: &Path) -> Option<String> {
    let content = fs::read_to_string(config_path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("font_family") && !trimmed.starts_with("font_family_") {
            // Format: "font_family <value>"
            let value = trimmed.strip_prefix("font_family")?.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn config_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(config_dir) = dirs::config_dir() {
        paths.push(config_dir.join("kitty").join("kitty.conf"));
    }

    // macOS: also check ~/.config/kitty/kitty.conf
    #[cfg(target_os = "macos")]
    if let Some(home) = dirs::home_dir() {
        let alt = home.join(".config").join("kitty").join("kitty.conf");
        if !paths.contains(&alt) {
            paths.push(alt);
        }
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_font_family() {
        let tmp = std::env::temp_dir().join("pnm_test_kitty_read");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let config = tmp.join("kitty.conf");
        fs::write(
            &config,
            "# kitty config\nfont_family FiraCode Nerd Font\nfont_size 14\n",
        )
        .unwrap();

        assert_eq!(
            read_font_family(&config),
            Some("FiraCode Nerd Font".to_string())
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_read_font_family_missing() {
        let tmp = std::env::temp_dir().join("pnm_test_kitty_missing");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let config = tmp.join("kitty.conf");
        fs::write(&config, "# kitty config\nfont_size 14\n").unwrap();

        assert_eq!(read_font_family(&config), None);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_read_does_not_match_variants() {
        let tmp = std::env::temp_dir().join("pnm_test_kitty_variants");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let config = tmp.join("kitty.conf");
        fs::write(
            &config,
            "font_family_bold auto\nfont_family_italic auto\n",
        )
        .unwrap();

        assert_eq!(read_font_family(&config), None);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_apply_font_replace() {
        let tmp = std::env::temp_dir().join("pnm_test_kitty_apply");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let config = tmp.join("kitty.conf");
        fs::write(
            &config,
            "# my config\nfont_family Cascadia Mono\nfont_size 14\n",
        )
        .unwrap();

        apply_font(&config, "JetBrainsMono Nerd Font").unwrap();

        assert_eq!(
            read_font_family(&config),
            Some("JetBrainsMono Nerd Font".to_string())
        );

        // Backup created
        assert!(tmp.join("kitty.conf.pnm-backup").exists());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_apply_font_append() {
        let tmp = std::env::temp_dir().join("pnm_test_kitty_append");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let config = tmp.join("kitty.conf");
        fs::write(&config, "# my config\nfont_size 14\n").unwrap();

        apply_font(&config, "JetBrainsMono Nerd Font").unwrap();

        let content = fs::read_to_string(&config).unwrap();
        assert!(content.contains("font_family JetBrainsMono Nerd Font"));
        assert!(content.contains("# Added by pnm"));

        let _ = fs::remove_dir_all(&tmp);
    }
}
