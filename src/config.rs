use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ConfigError
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// Leader key helpers
// ---------------------------------------------------------------------------

/// Common leader key options with display names.
pub const LEADER_KEY_OPTIONS: &[(&str, &str)] = &[
    (" ", "Space"),
    (",", "Comma"),
    ("\\", "Backslash"),
    (";", "Semicolon"),
];

/// Returns the display name for a leader key value.
pub fn leader_key_display(key: &str) -> &str {
    LEADER_KEY_OPTIONS
        .iter()
        .find(|(v, _)| *v == key)
        .map(|(_, name)| *name)
        .unwrap_or(key)
}

fn default_leader_key() -> String {
    " ".to_string()
}

// ---------------------------------------------------------------------------
// InstanceManifest
// ---------------------------------------------------------------------------

/// Instance configuration: enabled workloads and feature overrides.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstanceManifest {
    pub name: String,
    pub nvim_version: String,
    /// Enabled workload IDs (e.g. ["Lsp", "Dap", "TreeView"]).
    /// Alias "features" provides backward compat with old manifests.
    #[serde(alias = "features")]
    pub workloads: Vec<String>,
    /// Individual features to disable within enabled workloads ("WorkloadId/FeatureId").
    #[serde(default)]
    pub disabled_features: Vec<String>,
    /// Individual features to enable even when their parent workload is off.
    #[serde(default)]
    pub extra_features: Vec<String>,
    #[serde(default = "default_leader_key")]
    pub leader_key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl InstanceManifest {
    pub fn new(name: String, nvim_version: String, workloads: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            name,
            nvim_version,
            workloads,
            disabled_features: vec![],
            extra_features: vec![],
            leader_key: default_leader_key(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let data = fs::read_to_string(path)?;
        let manifest = serde_json::from_str(&data)?;
        Ok(manifest)
    }

    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn manifest_path(instance_dir: &Path) -> PathBuf {
        instance_dir.join("manifest.json")
    }
}

// ---------------------------------------------------------------------------
// GlobalSettings
// ---------------------------------------------------------------------------

fn default_instances_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap()
        .join(".portable-nvim")
        .join("instances")
}

/// Global application settings loaded from `settings.json` next to the
/// executable.  Every field uses `#[serde(default)]` so new fields can be
/// added without breaking existing files.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GlobalSettings {
    #[serde(default = "default_instances_dir")]
    pub instances_dir: PathBuf,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            instances_dir: default_instances_dir(),
        }
    }
}

/// Returns the path to `settings.json` next to the running executable.
pub fn settings_json_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("settings.json")
}

/// Loads global settings from `settings.json` next to the executable.
/// Returns defaults when the file does not exist.
pub fn load_global_settings() -> GlobalSettings {
    let path = settings_json_path();
    if !path.exists() {
        return GlobalSettings::default();
    }
    match fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_else(|e| {
            eprintln!(
                "Warning: failed to parse {}: {e}. Using defaults.",
                path.display()
            );
            GlobalSettings::default()
        }),
        Err(e) => {
            eprintln!(
                "Warning: failed to read {}: {e}. Using defaults.",
                path.display()
            );
            GlobalSettings::default()
        }
    }
}

/// Writes settings to `settings.json` next to the executable.
pub fn save_global_settings(settings: &GlobalSettings) -> Result<(), ConfigError> {
    let path = settings_json_path();
    let json = serde_json::to_string_pretty(settings)?;
    fs::write(path, json)?;
    Ok(())
}

/// Creates a default `settings.json` next to the executable if one does not
/// already exist.  Returns `true` if the file was created.
pub fn init_global_settings() -> Result<bool, ConfigError> {
    let path = settings_json_path();
    if path.exists() {
        return Ok(false);
    }
    save_global_settings(&GlobalSettings::default())?;
    Ok(true)
}

// ---------------------------------------------------------------------------
// Directory helpers
// ---------------------------------------------------------------------------

pub fn instances_dir(settings: &GlobalSettings) -> PathBuf {
    settings.instances_dir.clone()
}

pub fn instance_dir(settings: &GlobalSettings, name: &str) -> PathBuf {
    instances_dir(settings).join(name)
}

pub fn ensure_instance_dirs(settings: &GlobalSettings, name: &str) -> Result<PathBuf, ConfigError> {
    let base = instance_dir(settings, name);
    fs::create_dir_all(base.join("bin"))?;
    fs::create_dir_all(base.join("config").join("nvim"))?;
    fs::create_dir_all(base.join("data"))?;
    fs::create_dir_all(base.join("cache"))?;
    fs::create_dir_all(base.join("state"))?;
    Ok(base)
}

/// Like `ensure_instance_dirs` but rooted at an arbitrary base path (for testing).
pub fn ensure_instance_dirs_at(base: &Path) -> Result<PathBuf, ConfigError> {
    fs::create_dir_all(base.join("bin"))?;
    fs::create_dir_all(base.join("config").join("nvim"))?;
    fs::create_dir_all(base.join("data"))?;
    fs::create_dir_all(base.join("cache"))?;
    fs::create_dir_all(base.join("state"))?;
    Ok(base.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_save_load_roundtrip() {
        let tmp = std::env::temp_dir().join("pnm_test_manifest_roundtrip");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let manifest_path = tmp.join("manifest.json");
        let original = InstanceManifest::new(
            "my-instance".to_string(),
            "v0.10.4".to_string(),
            vec!["Lsp".to_string(), "Telescope".to_string()],
        );
        original.save(&manifest_path).unwrap();

        let loaded = InstanceManifest::load(&manifest_path).unwrap();
        assert_eq!(loaded.name, original.name);
        assert_eq!(loaded.nvim_version, original.nvim_version);
        assert_eq!(loaded.workloads, original.workloads);
        assert_eq!(loaded.leader_key, " ");
        assert_eq!(loaded.created_at, original.created_at);
        assert_eq!(loaded.updated_at, original.updated_at);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_manifest_backward_compat_no_leader_key() {
        let tmp = std::env::temp_dir().join("pnm_test_manifest_compat");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let json = r#"{
            "name": "old-instance",
            "nvim_version": "v0.10.0",
            "features": ["Lsp"],
            "created_at": "2025-01-01T00:00:00Z",
            "updated_at": "2025-01-01T00:00:00Z"
        }"#;
        let manifest_path = tmp.join("manifest.json");
        fs::write(&manifest_path, json).unwrap();

        let loaded = InstanceManifest::load(&manifest_path).unwrap();
        assert_eq!(
            loaded.leader_key, " ",
            "missing leader_key should default to space"
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_manifest_roundtrip_with_custom_leader_key() {
        let tmp = std::env::temp_dir().join("pnm_test_manifest_leader");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let manifest_path = tmp.join("manifest.json");
        let mut original =
            InstanceManifest::new("custom-leader".to_string(), "v0.10.4".to_string(), vec![]);
        original.leader_key = ",".to_string();
        original.save(&manifest_path).unwrap();

        let loaded = InstanceManifest::load(&manifest_path).unwrap();
        assert_eq!(loaded.leader_key, ",");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_leader_key_display() {
        assert_eq!(leader_key_display(" "), "Space");
        assert_eq!(leader_key_display(","), "Comma");
        assert_eq!(leader_key_display("\\"), "Backslash");
        assert_eq!(leader_key_display(";"), "Semicolon");
        assert_eq!(leader_key_display("x"), "x");
    }

    #[test]
    fn test_instance_dir_contains_name() {
        let settings = GlobalSettings::default();
        let dir = instance_dir(&settings, "test");
        let dir_str = dir.to_string_lossy();
        assert!(
            dir_str.ends_with("test"),
            "instance_dir should end with the name, got: {dir_str}"
        );
    }

    #[test]
    fn test_ensure_instance_dirs_creates_structure() {
        let tmp = std::env::temp_dir().join("pnm_test_instance_dirs");
        let _ = fs::remove_dir_all(&tmp);

        let base = ensure_instance_dirs_at(&tmp).unwrap();
        assert!(base.join("bin").is_dir());
        assert!(base.join("config").join("nvim").is_dir());
        assert!(base.join("data").is_dir());
        assert!(base.join("cache").is_dir());
        assert!(base.join("state").is_dir());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_global_settings_default() {
        let settings = GlobalSettings::default();
        let expected = dirs::home_dir()
            .unwrap()
            .join(".portable-nvim")
            .join("instances");
        assert_eq!(settings.instances_dir, expected);
    }

    #[test]
    fn test_global_settings_roundtrip() {
        let tmp = std::env::temp_dir().join("pnm_test_global_settings_rt");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let settings = GlobalSettings {
            instances_dir: tmp.join("my_instances"),
        };
        let path = tmp.join("settings.json");
        let json = serde_json::to_string_pretty(&settings).unwrap();
        fs::write(&path, &json).unwrap();

        let data = fs::read_to_string(&path).unwrap();
        let loaded: GlobalSettings = serde_json::from_str(&data).unwrap();
        assert_eq!(loaded.instances_dir, settings.instances_dir);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_global_settings_fallback_missing_field() {
        let json = "{}";
        let loaded: GlobalSettings = serde_json::from_str(json).unwrap();
        assert_eq!(loaded.instances_dir, GlobalSettings::default().instances_dir);
    }

    #[test]
    fn test_instances_dir_uses_settings() {
        let settings = GlobalSettings {
            instances_dir: PathBuf::from("/custom/path"),
        };
        assert_eq!(instances_dir(&settings), PathBuf::from("/custom/path"));
        assert_eq!(
            instance_dir(&settings, "myinst"),
            PathBuf::from("/custom/path/myinst")
        );
    }
}
