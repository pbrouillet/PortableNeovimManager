pub mod archive;
pub mod cli;
pub mod config;
pub mod font;
pub mod github;
pub mod instance;
pub mod mason_registry;
pub mod neovim;
pub mod plugins;
pub mod tui;
pub mod workload;

use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let settings = config::load_global_settings();
    let registry = workload::load_workloads();

    let result = match cli.command {
        Commands::Create {
            name,
            version,
            features,
        } => {
            if let Err(e) = cli::validate_instance_name(&name) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
            let feats = features
                .map(|f| cli::parse_features(&f, &registry))
                .unwrap_or_default();
            instance::create(&name, version.as_deref(), feats, &registry, &settings).await
        }
        Commands::List => match instance::list(&settings) {
            Ok(instances) => {
                if instances.is_empty() {
                    println!("No instances found. Create one with: pnm create <name>");
                } else {
                    println!(
                        "{:<20} {:<15} {:<30} UPDATED",
                        "NAME", "VERSION", "FEATURES"
                    );
                    println!("{}", "-".repeat(80));
                    for inst in &instances {
                        let features_str = inst.workloads.join(", ");
                        println!(
                            "{:<20} {:<15} {:<30} {}",
                            inst.name,
                            inst.nvim_version,
                            features_str,
                            inst.updated_at.format("%Y-%m-%d %H:%M"),
                        );
                    }
                }
                Ok(())
            }
            Err(e) => Err(e),
        },
        Commands::Info { name } => {
            let dir = config::instance_dir(&settings, &name);
            if !dir.exists() {
                eprintln!("Instance '{name}' not found.");
                std::process::exit(1);
            }
            let manifest_path = config::InstanceManifest::manifest_path(&dir);
            match config::InstanceManifest::load(&manifest_path) {
                Ok(inst) => {
                    println!("Name:     {}", inst.name);
                    println!("Version:  {}", inst.nvim_version);
                    println!("Features: {}", inst.workloads.join(", "));
                    println!(
                        "Created:  {}",
                        inst.created_at.format("%Y-%m-%d %H:%M:%S UTC")
                    );
                    println!(
                        "Updated:  {}",
                        inst.updated_at.format("%Y-%m-%d %H:%M:%S UTC")
                    );
                    println!("Path:     {}", dir.display());
                    if let Ok(ver) = neovim::get_version(&dir) {
                        println!("Binary:   {ver}");
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to load instance '{name}': {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Launch { name, args } => {
            let dir = config::instance_dir(&settings, &name);
            match neovim::launch(&dir, &args) {
                Ok(status) => {
                    std::process::exit(status.code().unwrap_or(0));
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Update { name, version } => instance::update(&name, version.as_deref(), &settings).await,
        Commands::Delete { name, yes } => {
            if !yes {
                eprint!("Delete instance '{name}' and all its data? [y/N] ");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Cancelled.");
                    return;
                }
            }
            instance::delete(&name, &settings)
        }
        Commands::Features {
            name,
            enable,
            disable,
        } => {
            let dir = config::instance_dir(&settings, &name);
            let manifest_path = config::InstanceManifest::manifest_path(&dir);
            match config::InstanceManifest::load(&manifest_path) {
                Ok(manifest) => {
                    let mut features: Vec<String> = manifest.workloads.clone();
                    if let Some(to_enable) = enable {
                        for f in cli::parse_features(&to_enable, &registry) {
                            if !features.contains(&f) {
                                features.push(f);
                            }
                        }
                    }
                    if let Some(to_disable) = disable {
                        let to_remove = cli::parse_features(&to_disable, &registry);
                        features.retain(|f| !to_remove.contains(f));
                    }
                    instance::update_features(&name, features, &registry, &settings)
                }
                Err(e) => {
                    eprintln!("Failed to load instance '{name}': {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Init => {
            match config::init_global_settings() {
                Ok(true) => {
                    let path = config::settings_json_path();
                    println!("✓ Created default settings.json at {}", path.display());
                    Ok(())
                }
                Ok(false) => {
                    let path = config::settings_json_path();
                    println!("settings.json already exists at {}", path.display());
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to create settings.json: {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Tutorial { topic } => {
            match topic {
                None => {
                    let topics = registry.all_tutorial_topics();
                    if topics.is_empty() {
                        println!("No tutorials available.");
                    } else {
                        println!("Available tutorials:\n");
                        for (id, title) in &topics {
                            println!("  {:<20} {}", id, title);
                        }
                        println!("\nRun: pnm tutorial <topic>");
                    }
                    Ok(())
                }
                Some(ref topic_id) => {
                    match registry.tutorial_content(topic_id) {
                        Some((_, content)) => {
                            println!("{content}");
                            Ok(())
                        }
                        None => {
                            eprintln!("Unknown tutorial topic '{topic_id}'.");
                            eprintln!("Run 'pnm tutorial' to see available topics.");
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
        Commands::Marketplace { action } => {
            handle_marketplace(action, &settings, &registry).await
        }
        Commands::Tui => match tui::run(settings).await {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!("TUI error: {e}");
                std::process::exit(1);
            }
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn handle_marketplace(
    action: cli::MarketplaceAction,
    settings: &config::GlobalSettings,
    registry: &workload::WorkloadRegistry,
) -> Result<(), instance::InstanceError> {
    use cli::MarketplaceAction;
    use mason_registry::fetch_registry;

    match action {
        MarketplaceAction::Search { query, category } => {
            let reg = fetch_registry(false).await.map_err(|e| {
                instance::InstanceError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;
            let mut results = reg.search(&query);
            if let Some(ref cat_str) = category {
                if let Some(ref c) = parse_mason_category(cat_str) {
                    results.retain(|p| p.is_category(c));
                }
            }
            if results.is_empty() {
                println!("No packages found matching '{query}'.");
            } else {
                println!("{:<30} {:<12} {:<20} {}", "NAME", "CATEGORY", "LANGUAGES", "DESCRIPTION");
                println!("{}", "-".repeat(90));
                for pkg in &results {
                    let cats = pkg.categories.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",");
                    let langs = pkg.languages.join(", ");
                    let desc = if pkg.description.len() > 40 {
                        format!("{}...", &pkg.description[..37])
                    } else {
                        pkg.description.clone()
                    };
                    println!("{:<30} {:<12} {:<20} {}", pkg.name, cats, langs, desc);
                }
                println!("\n{} package(s) found.", results.len());
            }
            Ok(())
        }
        MarketplaceAction::List { category, language } => {
            let reg = fetch_registry(false).await.map_err(|e| {
                instance::InstanceError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;
            let mut packages: Vec<&mason_registry::MasonPackage> = reg.packages.iter().collect();
            if let Some(ref cat_str) = category {
                if let Some(cat) = parse_mason_category(cat_str) {
                    packages.retain(|p| p.is_category(&cat));
                }
            }
            if let Some(ref lang) = language {
                let lower = lang.to_lowercase();
                packages.retain(|p| p.languages.iter().any(|l| l.to_lowercase() == lower));
            }
            if packages.is_empty() {
                println!("No packages found.");
            } else {
                println!("{:<30} {:<12} {:<20} {}", "NAME", "CATEGORY", "LANGUAGES", "DESCRIPTION");
                println!("{}", "-".repeat(90));
                for pkg in &packages {
                    let cats = pkg.categories.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",");
                    let langs = pkg.languages.join(", ");
                    let desc = if pkg.description.len() > 40 {
                        format!("{}...", &pkg.description[..37])
                    } else {
                        pkg.description.clone()
                    };
                    println!("{:<30} {:<12} {:<20} {}", pkg.name, cats, langs, desc);
                }
                println!("\n{} package(s).", packages.len());
            }
            Ok(())
        }
        MarketplaceAction::Install { name, packages } => {
            let dir = config::instance_dir(settings, &name);
            if !dir.exists() {
                eprintln!("Instance '{name}' not found.");
                std::process::exit(1);
            }
            let manifest_path = config::InstanceManifest::manifest_path(&dir);
            let mut manifest = config::InstanceManifest::load(&manifest_path).map_err(|e| {
                eprintln!("Failed to load instance '{name}': {e}");
                instance::InstanceError::Config(e)
            })?;

            let reg = fetch_registry(false).await.map_err(|e| {
                instance::InstanceError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;
            let mut added = Vec::new();
            let mut unknown = Vec::new();
            for pkg_name in &packages {
                if reg.find_by_name(pkg_name).is_some() {
                    if !manifest.mason_packages.contains(pkg_name) {
                        manifest.mason_packages.push(pkg_name.clone());
                        added.push(pkg_name.as_str());
                    }
                } else {
                    unknown.push(pkg_name.as_str());
                }
            }
            if !unknown.is_empty() {
                eprintln!("Warning: unknown package(s) skipped: {}", unknown.join(", "));
            }
            if added.is_empty() {
                println!("No new packages to add.");
                return Ok(());
            }

            manifest.updated_at = chrono::Utc::now();
            manifest.save(&manifest_path)?;

            // Regenerate init.lua
            let data_dir = dir.join("data");
            let init_lua = plugins::generate_init_lua_full(
                &data_dir,
                registry,
                &manifest.workloads,
                &manifest.disabled_features,
                &manifest.extra_features,
                &manifest.leader_key,
                &manifest.mason_packages,
            );
            let init_lua_path = dir.join("config").join("nvim").join("init.lua");
            std::fs::write(&init_lua_path, init_lua)?;

            println!("✓ Added {} package(s) to '{}': {}", added.len(), name, added.join(", "));
            println!("  Launch the instance to auto-install them.");
            Ok(())
        }
        MarketplaceAction::Remove { name, packages } => {
            let dir = config::instance_dir(settings, &name);
            if !dir.exists() {
                eprintln!("Instance '{name}' not found.");
                std::process::exit(1);
            }
            let manifest_path = config::InstanceManifest::manifest_path(&dir);
            let mut manifest = config::InstanceManifest::load(&manifest_path).map_err(|e| {
                eprintln!("Failed to load instance '{name}': {e}");
                instance::InstanceError::Config(e)
            })?;

            let before = manifest.mason_packages.len();
            manifest.mason_packages.retain(|p| !packages.contains(p));
            let removed = before - manifest.mason_packages.len();

            if removed == 0 {
                println!("No matching packages to remove.");
                return Ok(());
            }

            manifest.updated_at = chrono::Utc::now();
            manifest.save(&manifest_path)?;

            // Regenerate init.lua
            let data_dir = dir.join("data");
            let init_lua = plugins::generate_init_lua_full(
                &data_dir,
                registry,
                &manifest.workloads,
                &manifest.disabled_features,
                &manifest.extra_features,
                &manifest.leader_key,
                &manifest.mason_packages,
            );
            let init_lua_path = dir.join("config").join("nvim").join("init.lua");
            std::fs::write(&init_lua_path, init_lua)?;

            println!("✓ Removed {removed} package(s) from '{name}'.");
            println!("  Note: The tools remain installed in the Neovim data dir until manually removed via :Mason.");
            Ok(())
        }
        MarketplaceAction::Refresh => {
            println!("Fetching mason registry from GitHub...");
            let reg = fetch_registry(true).await.map_err(|e| {
                instance::InstanceError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;
            println!("✓ Registry refreshed. {} packages available.", reg.len());
            Ok(())
        }
        MarketplaceAction::Info { name: pkg_name } => {
            let reg = fetch_registry(false).await.map_err(|e| {
                instance::InstanceError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;
            match reg.find_by_name(&pkg_name) {
                Some(pkg) => {
                    println!("Name:        {}", pkg.name);
                    println!("Description: {}", pkg.description);
                    println!("Homepage:    {}", pkg.homepage);
                    println!("Categories:  {}", pkg.categories.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", "));
                    println!("Languages:   {}", pkg.languages.join(", "));
                    println!("Licenses:    {}", pkg.licenses.join(", "));
                    if let Some(lsp_name) = pkg.lspconfig_name() {
                        println!("LSP config:  {}", lsp_name);
                    }
                    Ok(())
                }
                None => {
                    eprintln!("Package '{pkg_name}' not found in the registry.");
                    eprintln!("Try: pnm marketplace search {pkg_name}");
                    std::process::exit(1);
                }
            }
        }
    }
}

fn parse_mason_category(s: &str) -> Option<mason_registry::MasonCategory> {
    match s.to_lowercase().as_str() {
        "lsp" => Some(mason_registry::MasonCategory::Lsp),
        "dap" => Some(mason_registry::MasonCategory::Dap),
        "formatter" => Some(mason_registry::MasonCategory::Formatter),
        "linter" => Some(mason_registry::MasonCategory::Linter),
        _ => {
            eprintln!("Warning: unknown category '{s}'. Valid: lsp, dap, formatter, linter");
            None
        }
    }
}
