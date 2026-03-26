use std::fs;
use std::path::{Path, PathBuf};

use super::terminal::{TerminalInstallation, TerminalKind, TerminalProfile};

// ── WT-specific types (kept for backward compatibility) ─────────────────────

/// A discovered Windows Terminal profile.
#[derive(Clone, Debug)]
pub struct WtProfile {
    pub guid: String,
    pub name: String,
    pub current_font: Option<String>,
}

/// A discovered Windows Terminal installation.
#[derive(Clone, Debug)]
pub struct WtInstallation {
    pub label: String,
    pub settings_path: PathBuf,
    pub profiles: Vec<WtProfile>,
    pub defaults_font: Option<String>,
}

impl WtInstallation {
    /// Convert to a generic [`TerminalInstallation`].
    pub fn to_terminal_installation(&self) -> TerminalInstallation {
        TerminalInstallation {
            kind: TerminalKind::WindowsTerminal,
            label: self.label.clone(),
            config_path: self.settings_path.clone(),
            profiles: self
                .profiles
                .iter()
                .map(|p| TerminalProfile {
                    id: p.guid.clone(),
                    name: p.name.clone(),
                    current_font: p.current_font.clone(),
                })
                .collect(),
            defaults_font: self.defaults_font.clone(),
            supports_defaults: true,
            read_only: false,
        }
    }
}

// ── Detection ───────────────────────────────────────────────────────────────

/// Detect Windows Terminal installations and return them as generic
/// [`TerminalInstallation`] values.
pub fn detect() -> Vec<TerminalInstallation> {
    find_wt_installations()
        .into_iter()
        .map(|i| i.to_terminal_installation())
        .collect()
}

/// Discover all Windows Terminal installations by checking known settings.json paths.
pub fn find_wt_installations() -> Vec<WtInstallation> {
    let Ok(localappdata) = std::env::var("LOCALAPPDATA") else {
        return Vec::new();
    };

    let candidates: Vec<(&str, PathBuf)> = vec![
        (
            "Windows Terminal",
            PathBuf::from(&localappdata)
                .join("Packages")
                .join("Microsoft.WindowsTerminal_8wekyb3d8bbwe")
                .join("LocalState")
                .join("settings.json"),
        ),
        (
            "Windows Terminal Preview",
            PathBuf::from(&localappdata)
                .join("Packages")
                .join("Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe")
                .join("LocalState")
                .join("settings.json"),
        ),
        (
            "Windows Terminal (unpackaged)",
            PathBuf::from(&localappdata)
                .join("Microsoft")
                .join("Windows Terminal")
                .join("settings.json"),
        ),
    ];

    let mut installations = Vec::new();

    for (label, path) in candidates {
        if !path.exists() {
            continue;
        }

        match parse_wt_settings(&path) {
            Ok((defaults_font, profiles)) => {
                installations.push(WtInstallation {
                    label: label.to_string(),
                    settings_path: path,
                    profiles,
                    defaults_font,
                });
            }
            Err(e) => {
                eprintln!("Warning: failed to parse {}: {e}", path.display());
            }
        }
    }

    installations
}

// ── Parsing ─────────────────────────────────────────────────────────────────

/// Parse a Windows Terminal settings.json, stripping JSONC comments.
/// Returns (defaults_font_face, profile_list).
fn parse_wt_settings(
    path: &Path,
) -> Result<(Option<String>, Vec<WtProfile>), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let reader = json_comments::StripComments::new(content.as_bytes());
    let settings: serde_json::Value = serde_json::from_reader(reader)?;

    // Extract defaults font face
    let defaults_font = settings
        .pointer("/profiles/defaults/font/face")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| {
            settings
                .pointer("/profiles/defaults/fontFace")
                .and_then(|v| v.as_str())
                .map(String::from)
        });

    // Extract profile list
    let mut profiles = Vec::new();
    if let Some(list) = settings.pointer("/profiles/list").and_then(|v| v.as_array()) {
        for entry in list {
            let guid = entry
                .get("guid")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name = entry
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("(unnamed)")
                .to_string();
            let current_font = entry
                .pointer("/font/face")
                .and_then(|v| v.as_str())
                .map(String::from)
                .or_else(|| {
                    entry
                        .get("fontFace")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                });
            profiles.push(WtProfile {
                guid,
                name,
                current_font,
            });
        }
    }

    Ok((defaults_font, profiles))
}

// ── Configuration ───────────────────────────────────────────────────────────

/// Apply the Nerd Font to the `profiles.defaults` section of a WT settings.json.
/// Creates a `.pnm-backup` before modifying.
pub fn apply_font_to_wt_defaults(
    settings_path: &Path,
    font_face: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(settings_path)?;
    let reader = json_comments::StripComments::new(content.as_bytes());
    let mut settings: serde_json::Value = serde_json::from_reader(reader)?;

    backup_wt_settings(settings_path)?;

    let profiles = settings
        .as_object_mut()
        .ok_or("settings.json root is not an object")?
        .entry("profiles")
        .or_insert_with(|| serde_json::json!({"defaults": {}, "list": []}));
    let defaults = profiles
        .as_object_mut()
        .ok_or("profiles is not an object")?
        .entry("defaults")
        .or_insert_with(|| serde_json::json!({}));
    let font = defaults
        .as_object_mut()
        .ok_or("defaults is not an object")?
        .entry("font")
        .or_insert_with(|| serde_json::json!({}));
    font.as_object_mut()
        .ok_or("font is not an object")?
        .insert("face".to_string(), serde_json::json!(font_face));

    if let Some(def_obj) = defaults.as_object_mut() {
        def_obj.remove("fontFace");
    }

    let output = serde_json::to_string_pretty(&settings)?;
    fs::write(settings_path, output)?;

    Ok(())
}

/// Apply the Nerd Font to specific profiles (by GUID) in a WT settings.json.
/// Creates a `.pnm-backup` before modifying.
pub fn apply_font_to_wt_profiles(
    settings_path: &Path,
    font_face: &str,
    guids: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if guids.is_empty() {
        return Ok(());
    }

    let content = fs::read_to_string(settings_path)?;
    let reader = json_comments::StripComments::new(content.as_bytes());
    let mut settings: serde_json::Value = serde_json::from_reader(reader)?;

    backup_wt_settings(settings_path)?;

    if let Some(list) = settings
        .pointer_mut("/profiles/list")
        .and_then(|v| v.as_array_mut())
    {
        for entry in list.iter_mut() {
            let entry_guid = entry
                .get("guid")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if guids.iter().any(|g| g == entry_guid) {
                let font = entry
                    .as_object_mut()
                    .unwrap()
                    .entry("font")
                    .or_insert_with(|| serde_json::json!({}));
                font.as_object_mut()
                    .unwrap()
                    .insert("face".to_string(), serde_json::json!(font_face));
                entry.as_object_mut().unwrap().remove("fontFace");
            }
        }
    }

    let output = serde_json::to_string_pretty(&settings)?;
    fs::write(settings_path, output)?;

    Ok(())
}

/// Check whether any WT installation already has the Nerd Font configured.
pub fn is_wt_configured(font_face: &str) -> bool {
    for install in find_wt_installations() {
        if install.defaults_font.as_deref() == Some(font_face) {
            return true;
        }
    }
    false
}

fn backup_wt_settings(settings_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let backup_path = settings_path.with_extension("json.pnm-backup");
    if !backup_path.exists() {
        fs::copy(settings_path, &backup_path)?;
    }
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wt_settings_minimal() {
        let tmp = std::env::temp_dir().join("pnm_test_wt_parse");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let settings_path = tmp.join("settings.json");
        fs::write(
            &settings_path,
            r#"{
                // This is a comment
                "profiles": {
                    "defaults": {
                        "font": {
                            "face": "Cascadia Mono",
                            "size": 12
                        }
                    },
                    "list": [
                        {
                            "guid": "{61c54bbd-c2c6-5271-96e7-009a87ff44bf}",
                            "name": "Windows PowerShell"
                        },
                        {
                            "guid": "{574e775e-4f2a-5b96-ac1e-a2962a402336}",
                            "name": "PowerShell",
                            "font": { "face": "FiraCode Nerd Font" }
                        }
                    ]
                }
            }"#,
        )
        .unwrap();

        let (defaults_font, profiles) = parse_wt_settings(&settings_path).unwrap();
        assert_eq!(defaults_font.as_deref(), Some("Cascadia Mono"));
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].name, "Windows PowerShell");
        assert!(profiles[0].current_font.is_none());
        assert_eq!(
            profiles[1].current_font.as_deref(),
            Some("FiraCode Nerd Font")
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_apply_font_to_defaults() {
        let tmp = std::env::temp_dir().join("pnm_test_wt_defaults");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let settings_path = tmp.join("settings.json");
        fs::write(
            &settings_path,
            r#"{
                "profiles": {
                    "defaults": {},
                    "list": [
                        { "guid": "{abc}", "name": "PowerShell" }
                    ]
                }
            }"#,
        )
        .unwrap();

        apply_font_to_wt_defaults(&settings_path, "JetBrainsMono Nerd Font").unwrap();

        let content = fs::read_to_string(&settings_path).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(
            settings
                .pointer("/profiles/defaults/font/face")
                .unwrap()
                .as_str()
                .unwrap(),
            "JetBrainsMono Nerd Font"
        );

        assert!(tmp.join("settings.json.pnm-backup").exists());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_apply_font_to_specific_profiles() {
        let tmp = std::env::temp_dir().join("pnm_test_wt_profiles");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let settings_path = tmp.join("settings.json");
        fs::write(
            &settings_path,
            r#"{
                "profiles": {
                    "defaults": {},
                    "list": [
                        { "guid": "{abc}", "name": "PowerShell" },
                        { "guid": "{def}", "name": "CMD" }
                    ]
                }
            }"#,
        )
        .unwrap();

        apply_font_to_wt_profiles(
            &settings_path,
            "JetBrainsMono Nerd Font",
            &["{abc}".to_string()],
        )
        .unwrap();

        let content = fs::read_to_string(&settings_path).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        let list = settings
            .pointer("/profiles/list")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(
            list[0].pointer("/font/face").unwrap().as_str().unwrap(),
            "JetBrainsMono Nerd Font"
        );
        assert!(list[1].pointer("/font/face").is_none());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_wt_to_terminal_installation() {
        let wt = WtInstallation {
            label: "Windows Terminal".to_string(),
            settings_path: PathBuf::from("C:\\test\\settings.json"),
            profiles: vec![WtProfile {
                guid: "{abc}".to_string(),
                name: "PowerShell".to_string(),
                current_font: Some("Cascadia Mono".to_string()),
            }],
            defaults_font: Some("Cascadia Mono".to_string()),
        };

        let ti = wt.to_terminal_installation();
        assert_eq!(ti.kind, TerminalKind::WindowsTerminal);
        assert!(ti.supports_defaults);
        assert!(!ti.read_only);
        assert_eq!(ti.profiles.len(), 1);
        assert_eq!(ti.profiles[0].id, "{abc}");
    }
}
