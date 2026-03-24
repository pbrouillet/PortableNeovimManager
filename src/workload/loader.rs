use std::path::{Path, PathBuf};

use super::defaults::default_registry;
use super::model::WorkloadRegistry;

/// Returns the path to workloads.json next to the executable.
pub fn workloads_json_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("workloads.json")
}

/// Loads the workload registry. If workloads.json doesn't exist, generates it
/// from built-in defaults first.
pub fn load_workloads() -> WorkloadRegistry {
    let path = workloads_json_path();
    if !path.exists() {
        let registry = default_registry();
        if let Err(e) = write_workloads_file(&path, &registry) {
            eprintln!("Warning: could not write default workloads.json: {e}");
        }
        return registry;
    }
    match load_workloads_from(&path) {
        Ok(registry) => registry,
        Err(e) => {
            eprintln!(
                "Warning: failed to load {}: {e}. Using built-in defaults.",
                path.display()
            );
            default_registry()
        }
    }
}

pub fn load_workloads_from(path: &Path) -> Result<WorkloadRegistry, Box<dyn std::error::Error>> {
    let data = std::fs::read_to_string(path)?;
    let mut registry: WorkloadRegistry = serde_json::from_str(&data)?;
    registry.normalize();
    Ok(registry)
}

pub fn write_workloads_file(
    path: &Path,
    registry: &WorkloadRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(registry)?;
    std::fs::write(path, json)?;
    Ok(())
}
