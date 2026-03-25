use std::fs;
use std::io::Read;
use std::path::PathBuf;

use crate::mason_registry::model::{MasonPackage, MasonRegistry};

const MASON_REGISTRY_API: &str =
    "https://api.github.com/repos/mason-org/mason-registry/releases/latest";
const USER_AGENT: &str = "portable-neovim-manager";
const CACHE_FILE: &str = "mason_registry_cache.json";

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    #[error("No registry.json.zip asset found in the latest release")]
    NoAssetFound,
    #[error("registry.json not found inside the zip archive")]
    NoRegistryInZip,
    #[error("No cached registry available (run `pnm marketplace refresh` first)")]
    NoCacheAvailable,
}

/// Returns the cache directory (next to the executable).
fn cache_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn cache_path() -> PathBuf {
    cache_dir().join(CACHE_FILE)
}

/// GitHub release API response (minimal fields we need).
#[derive(serde::Deserialize)]
struct GhRelease {
    assets: Vec<GhAsset>,
}

#[derive(serde::Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

/// Fetch the mason registry, using cache if available.
/// If `force_refresh` is true, always fetch from GitHub regardless of cache.
/// Otherwise, uses the cached version if it exists, and fetches only if no cache.
pub async fn fetch_registry(force_refresh: bool) -> Result<MasonRegistry, RegistryError> {
    if !force_refresh {
        if let Ok(reg) = load_from_cache() {
            return Ok(reg);
        }
    }

    // Fetch from GitHub
    let packages = fetch_from_github().await?;
    let registry = MasonRegistry::new(packages);

    // Cache to disk
    if let Err(e) = save_to_cache(&registry) {
        eprintln!("Warning: failed to cache mason registry: {e}");
    }

    Ok(registry)
}

/// Load the registry from disk cache.
pub fn load_from_cache() -> Result<MasonRegistry, RegistryError> {
    let path = cache_path();
    if !path.exists() {
        return Err(RegistryError::NoCacheAvailable);
    }
    let data = fs::read_to_string(&path)?;
    let packages: Vec<MasonPackage> = serde_json::from_str(&data)?;
    Ok(MasonRegistry::new(packages))
}

/// Save the registry to disk cache.
fn save_to_cache(registry: &MasonRegistry) -> Result<(), RegistryError> {
    let path = cache_path();
    let json = serde_json::to_string(&registry.packages)?;
    fs::write(path, json)?;
    Ok(())
}

/// Fetch fresh registry data from GitHub.
async fn fetch_from_github() -> Result<Vec<MasonPackage>, RegistryError> {
    let client = reqwest::Client::new();

    // 1. Get the latest release metadata
    let release: GhRelease = client
        .get(MASON_REGISTRY_API)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    // 2. Find the registry.json.zip asset
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == "registry.json.zip")
        .ok_or(RegistryError::NoAssetFound)?;

    // 3. Download the zip
    let zip_bytes = client
        .get(&asset.browser_download_url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    // 4. Extract registry.json from the zip
    let cursor = std::io::Cursor::new(zip_bytes.as_ref());
    let mut archive = zip::ZipArchive::new(cursor)?;

    let mut json_data = String::new();
    let mut found = false;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.name().ends_with("registry.json") {
            file.read_to_string(&mut json_data)?;
            found = true;
            break;
        }
    }

    if !found {
        return Err(RegistryError::NoRegistryInZip);
    }

    // 5. Parse the JSON array of packages
    let packages: Vec<MasonPackage> = serde_json::from_str(&json_data)?;
    Ok(packages)
}

/// Check if a cached registry exists on disk.
pub fn has_cache() -> bool {
    cache_path().exists()
}
