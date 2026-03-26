use std::fs;
use std::path::{Path, PathBuf};

use super::terminal::{TerminalInstallation, TerminalKind, TerminalProfile};

/// Detect Alacritty installations by checking known config paths.
pub fn detect() -> Vec<TerminalInstallation> {
    let mut results = Vec::new();

    for path in config_paths() {
        if path.exists() {
            let current_font = read_font_family(&path);
            results.push(TerminalInstallation {
                kind: TerminalKind::Alacritty,
                label: "Alacritty".to_string(),
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
            break; // Only one Alacritty config is active
        }
    }

    results
}

/// Apply the font to an Alacritty configuration file.
/// Creates a `.pnm-backup` before modifying.
pub fn apply_font(config_path: &Path, font_face: &str) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(config_path)?;

    // Backup before modifying
    let backup_path = config_path.with_extension(
        config_path
            .extension()
            .map(|e| format!("{}.pnm-backup", e.to_string_lossy()))
            .unwrap_or_else(|| "pnm-backup".to_string()),
    );
    if !backup_path.exists() {
        fs::copy(config_path, &backup_path)?;
    }

    let mut doc: toml_edit::DocumentMut = content.parse()?;

    // Ensure [font.normal] section exists
    if doc.get("font").is_none() {
        doc["font"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    if doc["font"].get("normal").is_none() {
        doc["font"]["normal"] = toml_edit::Item::Table(toml_edit::Table::new());
    }

    doc["font"]["normal"]["family"] = toml_edit::value(font_face);

    fs::write(config_path, doc.to_string())?;
    Ok(())
}

/// Read the current font family from an Alacritty TOML config.
fn read_font_family(config_path: &Path) -> Option<String> {
    let content = fs::read_to_string(config_path).ok()?;
    let doc: toml_edit::DocumentMut = content.parse().ok()?;
    doc.get("font")?
        .get("normal")?
        .get("family")?
        .as_str()
        .map(String::from)
}

/// Return candidate Alacritty config file paths (in priority order).
fn config_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // XDG_CONFIG_HOME or platform default
    if let Some(config_dir) = dirs::config_dir() {
        paths.push(config_dir.join("alacritty").join("alacritty.toml"));
        // Legacy YAML format (pre-0.13)
        paths.push(config_dir.join("alacritty").join("alacritty.yml"));
    }

    // macOS: ~/.config/alacritty/ is also common
    #[cfg(target_os = "macos")]
    if let Some(home) = dirs::home_dir() {
        let alt = home.join(".config").join("alacritty").join("alacritty.toml");
        if !paths.contains(&alt) {
            paths.push(alt);
        }
    }

    #[cfg(target_os = "windows")]
    if let Ok(appdata) = std::env::var("APPDATA") {
        paths.push(
            PathBuf::from(appdata)
                .join("alacritty")
                .join("alacritty.toml"),
        );
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_read_font_family() {
        let tmp = std::env::temp_dir().join("pnm_test_alacritty_read");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let config = tmp.join("alacritty.toml");
        fs::write(
            &config,
            r#"
[font]
size = 12.0

[font.normal]
family = "FiraCode Nerd Font"
style = "Regular"
"#,
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
        let tmp = std::env::temp_dir().join("pnm_test_alacritty_missing");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let config = tmp.join("alacritty.toml");
        fs::write(&config, "[window]\npadding = { x = 5, y = 5 }\n").unwrap();

        assert_eq!(read_font_family(&config), None);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_apply_font() {
        let tmp = std::env::temp_dir().join("pnm_test_alacritty_apply");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let config = tmp.join("alacritty.toml");
        fs::write(
            &config,
            r#"# My Alacritty config
[window]
padding = { x = 5, y = 5 }

[font]
size = 12.0

[font.normal]
family = "Cascadia Mono"
"#,
        )
        .unwrap();

        apply_font(&config, "JetBrainsMono Nerd Font").unwrap();

        // Verify font was changed
        assert_eq!(
            read_font_family(&config),
            Some("JetBrainsMono Nerd Font".to_string())
        );

        // Verify backup was created
        assert!(tmp.join("alacritty.toml.pnm-backup").exists());

        // Verify original content is in backup
        let backup = fs::read_to_string(tmp.join("alacritty.toml.pnm-backup")).unwrap();
        assert!(backup.contains("Cascadia Mono"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_apply_font_creates_sections() {
        let tmp = std::env::temp_dir().join("pnm_test_alacritty_create");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let config = tmp.join("alacritty.toml");
        fs::write(&config, "# Empty config\n").unwrap();

        apply_font(&config, "JetBrainsMono Nerd Font").unwrap();

        assert_eq!(
            read_font_family(&config),
            Some("JetBrainsMono Nerd Font".to_string())
        );

        let _ = fs::remove_dir_all(&tmp);
    }
}
