use std::fs;

use chrono::Utc;
use indicatif::{ProgressBar, ProgressStyle};

use crate::archive;
use crate::config::{self, GlobalSettings, InstanceManifest};
use crate::github;
use crate::neovim;
use crate::plugins::generate_init_lua;
use crate::workload::WorkloadRegistry;

// ---------------------------------------------------------------------------
// InstanceError
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum InstanceError {
    #[error(transparent)]
    Config(#[from] config::ConfigError),

    #[error(transparent)]
    Github(#[from] github::GithubError),

    #[error(transparent)]
    Archive(#[from] archive::ArchiveError),

    #[error(transparent)]
    Neovim(#[from] neovim::NeovimError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Instance '{0}' already exists")]
    AlreadyExists(String),

    #[error("Instance '{0}' not found")]
    NotFound(String),

    #[error("Instance '{0}' is already up to date ({1})")]
    AlreadyUpToDate(String, String),
}

// ---------------------------------------------------------------------------
// create
// ---------------------------------------------------------------------------

pub async fn create(
    name: &str,
    version: Option<&str>,
    features: Vec<String>,
    registry: &WorkloadRegistry,
    settings: &GlobalSettings,
) -> Result<(), InstanceError> {
    let instance_dir = config::instance_dir(settings, name);
    if instance_dir.exists() {
        return Err(InstanceError::AlreadyExists(name.to_string()));
    }

    let base = config::ensure_instance_dirs(settings, name)?;

    // Fetch the release
    let release = match version {
        Some(tag) => github::fetch_release_by_tag(tag).await?,
        None => github::fetch_latest_stable().await?,
    };

    let asset = github::select_asset(&release)?;

    println!("Downloading {} ({})", asset.name, format_bytes(asset.size));

    // Progress bar
    let pb = ProgressBar::new(asset.size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})",
            )
            .unwrap()
            .progress_chars("█▓░"),
    );

    let data = github::download_asset_with_progress(asset, |downloaded, total| {
        pb.set_length(total);
        pb.set_position(downloaded);
    })
    .await?;

    pb.finish_with_message("Download complete");

    // Extract to a temp dir inside the instance
    let tmp_dir = base.join("_tmp");
    fs::create_dir_all(&tmp_dir)?;

    println!("Extracting {}...", asset.name);
    archive::extract(&data, &tmp_dir, &asset.name)?;

    // Install the extracted neovim tree into bin/
    let bin_dir = base.join("bin");
    archive::install_nvim_binary(&tmp_dir, &bin_dir)?;

    // Clean up temp extraction dir
    fs::remove_dir_all(&tmp_dir)?;

    // Create and save manifest
    let manifest =
        InstanceManifest::new(name.to_string(), release.tag_name.clone(), features.clone());
    manifest.save(&InstanceManifest::manifest_path(&base))?;

    // Generate and write init.lua
    let data_dir = base.join("data");
    let init_lua = generate_init_lua(&data_dir, registry, &features, &manifest.leader_key);
    let init_lua_path = base.join("config").join("nvim").join("init.lua");
    fs::write(&init_lua_path, init_lua)?;

    println!(
        "✓ Instance '{}' created with Neovim {}",
        name, release.tag_name
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

pub fn list(settings: &GlobalSettings) -> Result<Vec<InstanceManifest>, InstanceError> {
    let instances = config::instances_dir(settings);
    if !instances.exists() {
        return Ok(Vec::new());
    }

    let mut manifests = Vec::new();
    for entry in fs::read_dir(&instances)? {
        let entry = entry?;
        if !entry.path().is_dir() {
            continue;
        }

        let manifest_path = InstanceManifest::manifest_path(&entry.path());
        match InstanceManifest::load(&manifest_path) {
            Ok(m) => manifests.push(m),
            Err(e) => {
                eprintln!(
                    "Warning: skipping instance '{}': {}",
                    entry.file_name().to_string_lossy(),
                    e,
                );
            }
        }
    }

    Ok(manifests)
}

// ---------------------------------------------------------------------------
// update
// ---------------------------------------------------------------------------

pub async fn update(name: &str, version: Option<&str>, settings: &GlobalSettings) -> Result<(), InstanceError> {
    let base = config::instance_dir(settings, name);
    if !base.exists() {
        return Err(InstanceError::NotFound(name.to_string()));
    }

    let manifest_path = InstanceManifest::manifest_path(&base);
    let mut manifest = InstanceManifest::load(&manifest_path)?;

    // Fetch target release
    let release = match version {
        Some(tag) => github::fetch_release_by_tag(tag).await?,
        None => github::fetch_latest_stable().await?,
    };

    if release.tag_name == manifest.nvim_version {
        println!(
            "Instance '{}' is already up to date ({})",
            name, manifest.nvim_version,
        );
        return Ok(());
    }

    let asset = github::select_asset(&release)?;

    println!(
        "Updating '{}': {} → {}",
        name, manifest.nvim_version, release.tag_name,
    );
    println!("Downloading {} ({})", asset.name, format_bytes(asset.size));

    let pb = ProgressBar::new(asset.size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})",
            )
            .unwrap()
            .progress_chars("█▓░"),
    );

    let data = github::download_asset_with_progress(asset, |downloaded, total| {
        pb.set_length(total);
        pb.set_position(downloaded);
    })
    .await?;

    pb.finish_with_message("Download complete");

    // Extract to temp dir
    let tmp_dir = base.join("_tmp");
    fs::create_dir_all(&tmp_dir)?;

    println!("Extracting {}...", asset.name);
    archive::extract(&data, &tmp_dir, &asset.name)?;

    // Remove old bin/ contents before installing new
    let bin_dir = base.join("bin");
    if bin_dir.exists() {
        fs::remove_dir_all(&bin_dir)?;
        fs::create_dir_all(&bin_dir)?;
    }

    // Install new binary
    archive::install_nvim_binary(&tmp_dir, &bin_dir)?;

    // Clean up
    fs::remove_dir_all(&tmp_dir)?;

    // Update manifest
    manifest.nvim_version = release.tag_name.clone();
    manifest.updated_at = Utc::now();
    manifest.save(&manifest_path)?;

    println!(
        "✓ Instance '{}' updated to Neovim {}",
        name, release.tag_name,
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// delete
// ---------------------------------------------------------------------------

pub fn delete(name: &str, settings: &GlobalSettings) -> Result<(), InstanceError> {
    let base = config::instance_dir(settings, name);
    if !base.exists() {
        return Err(InstanceError::NotFound(name.to_string()));
    }

    fs::remove_dir_all(&base)?;
    println!("✓ Instance '{}' deleted", name);

    Ok(())
}

// ---------------------------------------------------------------------------
// update_features
// ---------------------------------------------------------------------------

pub fn update_features(
    name: &str,
    features: Vec<String>,
    registry: &WorkloadRegistry,
    settings: &GlobalSettings,
) -> Result<(), InstanceError> {
    let base = config::instance_dir(settings, name);
    if !base.exists() {
        return Err(InstanceError::NotFound(name.to_string()));
    }

    let manifest_path = InstanceManifest::manifest_path(&base);
    let mut manifest = InstanceManifest::load(&manifest_path)?;

    manifest.features = features.clone();
    manifest.updated_at = Utc::now();

    // Regenerate init.lua with updated features
    let data_dir = base.join("data");
    let init_lua = generate_init_lua(&data_dir, registry, &features, &manifest.leader_key);
    let init_lua_path = base.join("config").join("nvim").join("init.lua");
    fs::write(&init_lua_path, init_lua)?;

    manifest.save(&manifest_path)?;

    println!("✓ Features updated for instance '{}'", name);

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
