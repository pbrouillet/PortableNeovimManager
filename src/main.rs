pub mod archive;
pub mod cli;
pub mod config;
pub mod font;
pub mod github;
pub mod instance;
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
                        let features_str = inst.features.join(", ");
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
                    println!("Features: {}", inst.features.join(", "));
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
                    let mut features: Vec<String> = manifest.features.clone();
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
