pub mod archive;
pub mod cli;
pub mod config;
pub mod font;
pub mod github;
pub mod instance;
pub mod mason_registry;
pub mod monitor;
pub mod neovim;
pub mod plugins;
pub mod runtime;
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
            js_runtime,
        } => {
            if let Err(e) = cli::validate_instance_name(&name) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
            // Validate runtime binary exists if specified
            if let Some(ref rt) = js_runtime {
                if let Err(e) = runtime::find_runtime_binary(rt) {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
            let feats = features
                .map(|f| cli::parse_features(&f, &registry))
                .unwrap_or_default();
            let result = instance::create(&name, version.as_deref(), feats, &registry, &settings).await;
            // Set js_runtime on the newly created manifest if specified
            if result.is_ok() {
                if let Some(rt) = js_runtime {
                    let dir = config::instance_dir(&settings, &name);
                    let manifest_path = config::InstanceManifest::manifest_path(&dir);
                    if let Ok(mut manifest) = config::InstanceManifest::load(&manifest_path) {
                        manifest.js_runtime = Some(rt);
                        manifest.updated_at = chrono::Utc::now();
                        let _ = manifest.save(&manifest_path);
                    }
                }
            }
            result
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
            let manifest_path = config::InstanceManifest::manifest_path(&dir);
            let js_runtime_path = config::InstanceManifest::load(&manifest_path)
                .ok()
                .and_then(|m| {
                    runtime::setup_runtime_shims(&dir, &m, &settings)
                        .map_err(|e| eprintln!("Warning: runtime shim setup failed: {e}"))
                        .ok()
                        .flatten()
                });
            match neovim::launch(&dir, &name, &args, js_runtime_path) {
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
        Commands::Monitor { name, no_lua } => {
            let dir = config::instance_dir(&settings, &name);
            if !dir.exists() {
                eprintln!("Instance '{name}' not found.");
                std::process::exit(1);
            }
            let nvim_binary = if no_lua {
                None
            } else {
                neovim::find_nvim_binary(&dir).ok()
            };
            match monitor::full_snapshot(&dir, nvim_binary.as_deref()) {
                Ok(snap) => {
                    println!("Instance: {} (PID {})", name, snap.nvim_process.pid);
                    println!("{}", "─".repeat(50));
                    println!("Neovim process:");
                    println!("  Working Set:    {}", monitor::format_bytes(snap.nvim_process.working_set_bytes));
                    println!("  Virtual Memory: {}", monitor::format_bytes(snap.nvim_process.virtual_memory_bytes));
                    println!("  CPU:            {:.1}%", snap.nvim_process.cpu_percent);
                    println!();
                    if let Some(lua_bytes) = snap.lua_memory_bytes {
                        println!("Lua Heap:         {}", monitor::format_bytes(lua_bytes));
                        println!();
                    }
                    if snap.child_processes.is_empty() {
                        println!("No child processes.");
                    } else {
                        println!("Child Processes ({}):", snap.child_processes.len());
                        println!("  {:<8} {:<25} {:<15} {}", "PID", "NAME", "WORKING SET", "VIRTUAL");
                        for child in &snap.child_processes {
                            let name_display = if child.name.len() > 24 {
                                format!("{}…", &child.name[..23])
                            } else {
                                child.name.clone()
                            };
                            println!(
                                "  {:<8} {:<25} {:<15} {}",
                                child.pid,
                                name_display,
                                monitor::format_bytes(child.working_set_bytes),
                                monitor::format_bytes(child.virtual_memory_bytes),
                            );
                        }
                    }
                    println!();
                    println!("Total Working Set:  {}", monitor::format_bytes(snap.total_working_set_bytes));
                    println!("Total Virtual:      {}", monitor::format_bytes(snap.total_virtual_memory_bytes));
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Monitor error: {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Runtime { name, set, unset } => {
            let dir = config::instance_dir(&settings, &name);
            if !dir.exists() {
                eprintln!("Instance '{name}' not found.");
                std::process::exit(1);
            }
            let manifest_path = config::InstanceManifest::manifest_path(&dir);
            let mut manifest = match config::InstanceManifest::load(&manifest_path) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Failed to load instance '{name}': {e}");
                    std::process::exit(1);
                }
            };

            if unset {
                manifest.js_runtime = None;
                manifest.updated_at = chrono::Utc::now();
                if let Err(e) = manifest.save(&manifest_path) {
                    eprintln!("Failed to save manifest: {e}");
                    std::process::exit(1);
                }
                // Regenerate init.lua so copilot/node_host_prog block is up to date
                let data_dir = dir.join("data");
                let init_lua = plugins::generate_init_lua_full(
                    &data_dir,
                    &registry,
                    &manifest.workloads,
                    &manifest.disabled_features,
                    &manifest.extra_features,
                    &manifest.leader_key,
                    &manifest.mason_packages,
                    manifest.init_lua_pre.as_deref(),
                    manifest.init_lua_post.as_deref(),
                );
                let init_lua_path = dir.join("config").join("nvim").join("init.lua");
                if let Err(e) = std::fs::write(&init_lua_path, init_lua) {
                    eprintln!("Warning: failed to regenerate init.lua: {e}");
                }
                println!("✓ Cleared JS runtime override for '{name}'. Using global default.");
                let effective = runtime::resolve_js_runtime(&manifest, &settings);
                println!("  Effective runtime: {}", runtime::runtime_display_name(effective.as_deref()));
                Ok(())
            } else if let Some(rt) = set {
                // Validate the runtime binary exists
                match runtime::find_runtime_binary(&rt) {
                    Ok(path) => {
                        println!("Found runtime: {}", path.display());
                    }
                    Err(e) => {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                }
                manifest.js_runtime = Some(rt.clone());
                manifest.updated_at = chrono::Utc::now();
                if let Err(e) = manifest.save(&manifest_path) {
                    eprintln!("Failed to save manifest: {e}");
                    std::process::exit(1);
                }
                // Regenerate init.lua so copilot/node_host_prog block is up to date
                let data_dir = dir.join("data");
                let init_lua = plugins::generate_init_lua_full(
                    &data_dir,
                    &registry,
                    &manifest.workloads,
                    &manifest.disabled_features,
                    &manifest.extra_features,
                    &manifest.leader_key,
                    &manifest.mason_packages,
                    manifest.init_lua_pre.as_deref(),
                    manifest.init_lua_post.as_deref(),
                );
                let init_lua_path = dir.join("config").join("nvim").join("init.lua");
                if let Err(e) = std::fs::write(&init_lua_path, init_lua) {
                    eprintln!("Warning: failed to regenerate init.lua: {e}");
                }
                println!("✓ Set JS runtime for '{name}' to '{rt}'.");
                println!("  Plugins will use this runtime on next launch.");
                Ok(())
            } else {
                // Show current runtime
                let effective = runtime::resolve_js_runtime(&manifest, &settings);
                let display = runtime::runtime_display_name(effective.as_deref());
                println!("Instance:          {name}");
                println!("Per-instance:      {}", manifest.js_runtime.as_deref().unwrap_or("(not set)"));
                println!("Global default:    {}", settings.default_js_runtime.as_deref().unwrap_or("(not set)"));
                println!("Effective runtime: {display}");
                if let Some(ref val) = effective {
                    match runtime::find_runtime_binary(val) {
                        Ok(path) => println!("Binary path:       {}", path.display()),
                        Err(e) => println!("Binary path:       ⚠ {e}"),
                    }
                }
                Ok(())
            }
        }
        Commands::InitConfig {
            name,
            edit_pre,
            edit_post,
            reset,
        } => {
            let dir = config::instance_dir(&settings, &name);
            if !dir.exists() {
                eprintln!("Instance '{name}' not found.");
                std::process::exit(1);
            }
            let manifest_path = config::InstanceManifest::manifest_path(&dir);
            let mut manifest = match config::InstanceManifest::load(&manifest_path) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Failed to load instance '{name}': {e}");
                    std::process::exit(1);
                }
            };

            if reset {
                manifest.init_lua_pre = plugins::generate_default_pre(&manifest.workloads);
                manifest.init_lua_post = plugins::generate_default_post(&manifest.workloads);
                manifest.updated_at = chrono::Utc::now();
                if let Err(e) = manifest.save(&manifest_path) {
                    eprintln!("Failed to save manifest: {e}");
                    std::process::exit(1);
                }
                // Regenerate init.lua
                let data_dir = dir.join("data");
                let init_lua = plugins::generate_init_lua_full(
                    &data_dir,
                    &registry,
                    &manifest.workloads,
                    &manifest.disabled_features,
                    &manifest.extra_features,
                    &manifest.leader_key,
                    &manifest.mason_packages,
                    manifest.init_lua_pre.as_deref(),
                    manifest.init_lua_post.as_deref(),
                );
                let init_lua_path = dir.join("config").join("nvim").join("init.lua");
                if let Err(e) = std::fs::write(&init_lua_path, init_lua) {
                    eprintln!("Warning: failed to regenerate init.lua: {e}");
                }
                println!("✓ Reset init overrides to smart defaults for '{name}'.");
                if let Some(ref pre) = manifest.init_lua_pre {
                    println!("\nPre-plugins:\n{pre}");
                }
                if let Some(ref post) = manifest.init_lua_post {
                    println!("\nPost-plugins:\n{post}");
                }
                if manifest.init_lua_pre.is_none() && manifest.init_lua_post.is_none() {
                    println!("  (no defaults for current features)");
                }
                Ok(())
            } else if edit_pre || edit_post {
                let label = if edit_pre { "pre-plugins" } else { "post-plugins" };
                let current = if edit_pre {
                    manifest.init_lua_pre.as_deref().unwrap_or("")
                } else {
                    manifest.init_lua_post.as_deref().unwrap_or("")
                };

                // Write to temp file, open editor, read back
                let tmp_dir = std::env::temp_dir();
                let tmp_path = tmp_dir.join(format!("pnm-{name}-{label}.lua"));
                if let Err(e) = std::fs::write(&tmp_path, current) {
                    eprintln!("Failed to create temp file: {e}");
                    std::process::exit(1);
                }

                let editor = std::env::var("EDITOR")
                    .or_else(|_| std::env::var("VISUAL"))
                    .unwrap_or_else(|_| {
                        if cfg!(windows) {
                            "notepad".to_string()
                        } else {
                            "vi".to_string()
                        }
                    });

                println!("Opening {label} overrides in {editor}...");
                let status = std::process::Command::new(&editor)
                    .arg(&tmp_path)
                    .status();

                match status {
                    Ok(s) if s.success() => {}
                    Ok(s) => {
                        eprintln!("Editor exited with status: {s}");
                        let _ = std::fs::remove_file(&tmp_path);
                        std::process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("Failed to launch editor '{editor}': {e}");
                        let _ = std::fs::remove_file(&tmp_path);
                        std::process::exit(1);
                    }
                }

                let new_content = match std::fs::read_to_string(&tmp_path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Failed to read temp file: {e}");
                        std::process::exit(1);
                    }
                };
                let _ = std::fs::remove_file(&tmp_path);

                let new_value = if new_content.trim().is_empty() {
                    None
                } else {
                    Some(new_content)
                };

                if edit_pre {
                    manifest.init_lua_pre = new_value;
                } else {
                    manifest.init_lua_post = new_value;
                }
                manifest.updated_at = chrono::Utc::now();
                if let Err(e) = manifest.save(&manifest_path) {
                    eprintln!("Failed to save manifest: {e}");
                    std::process::exit(1);
                }

                // Regenerate init.lua
                let data_dir = dir.join("data");
                let init_lua = plugins::generate_init_lua_full(
                    &data_dir,
                    &registry,
                    &manifest.workloads,
                    &manifest.disabled_features,
                    &manifest.extra_features,
                    &manifest.leader_key,
                    &manifest.mason_packages,
                    manifest.init_lua_pre.as_deref(),
                    manifest.init_lua_post.as_deref(),
                );
                let init_lua_path = dir.join("config").join("nvim").join("init.lua");
                if let Err(e) = std::fs::write(&init_lua_path, init_lua) {
                    eprintln!("Warning: failed to regenerate init.lua: {e}");
                }
                println!("✓ Updated {label} overrides for '{name}'.");
                Ok(())
            } else {
                // Show current overrides
                let eff_pre = plugins::resolve_init_lua_pre(&manifest, &settings);
                let eff_post = plugins::resolve_init_lua_post(&manifest, &settings);

                println!("Instance: {name}");
                println!();
                println!("── Pre-plugins Lua ──");
                match eff_pre {
                    Some(ref s) if !s.trim().is_empty() => println!("{s}"),
                    _ => println!("  (empty)"),
                }
                println!();
                println!("── Post-plugins Lua ──");
                match eff_post {
                    Some(ref s) if !s.trim().is_empty() => println!("{s}"),
                    _ => println!("  (empty)"),
                }

                // Show source info
                println!();
                println!(
                    "Pre source:  {}",
                    if manifest.init_lua_pre.is_some() {
                        "instance"
                    } else if settings.default_init_lua_pre.is_some() {
                        "global default"
                    } else {
                        "(none)"
                    }
                );
                println!(
                    "Post source: {}",
                    if manifest.init_lua_post.is_some() {
                        "instance"
                    } else if settings.default_init_lua_post.is_some() {
                        "global default"
                    } else {
                        "(none)"
                    }
                );
                Ok(())
            }
        }
        Commands::Tui => match tui::run(settings).await {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!("TUI error: {e}");
                std::process::exit(1);
            }
        },
        Commands::Font { action } => {
            handle_font(action).await;
            Ok(())
        }
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
                manifest.init_lua_pre.as_deref(),
                manifest.init_lua_post.as_deref(),
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
                manifest.init_lua_pre.as_deref(),
                manifest.init_lua_post.as_deref(),
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

async fn handle_font(action: cli::FontAction) {
    use cli::FontAction;

    match action {
        FontAction::Install { no_terminal } => {
            let result = font::install_nerd_font().await;
            println!("{}", result.message);

            if no_terminal {
                return;
            }

            // Configure detected terminal emulators
            let installations = font::find_terminals();
            if installations.is_empty() {
                println!(
                    "\nNo supported terminal emulator detected.\n\
                     Set your terminal font to \"{}\" manually.",
                    font::NERD_FONT_FACE
                );
                return;
            }

            for install in &installations {
                if install.read_only {
                    if let Some(instructions) =
                        font::terminal::manual_instructions(&install.kind, font::NERD_FONT_FACE)
                    {
                        println!("\n{}: {instructions}", install.label);
                    }
                    continue;
                }
                print!("Configuring {}... ", install.label);
                match font::apply_terminal_font_to_defaults(install, font::NERD_FONT_FACE) {
                    Ok(()) => println!("✓"),
                    Err(e) => println!("✗ {e}"),
                }
            }
            println!("\nDone. Restart your terminal to see the new font.");
        }
        FontAction::Status => {
            let installed = font::is_font_installed();
            println!(
                "Nerd Font installed:           {}",
                if installed { "✓ yes" } else { "✗ no" }
            );

            let terminals = font::find_terminals();
            if terminals.is_empty() {
                println!("Terminal emulators detected:    none");
            } else {
                for install in &terminals {
                    let status = if install.read_only {
                        "manual config only".to_string()
                    } else if install.defaults_font.as_deref() == Some(font::NERD_FONT_FACE) {
                        "✓ configured".to_string()
                    } else if install
                        .profiles
                        .iter()
                        .any(|p| p.current_font.as_deref() == Some(font::NERD_FONT_FACE))
                    {
                        "✓ partially configured".to_string()
                    } else {
                        "✗ not configured".to_string()
                    };
                    println!("{:<35}{status}", format!("{}:", install.label));
                }
            }

            if !installed {
                println!("\nRun `pnm font install` to install the font.");
            } else if !font::is_any_terminal_configured(font::NERD_FONT_FACE) {
                println!("\nRun `pnm font configure-terminal` to configure your terminal.");
            }
        }
        FontAction::ConfigureTerminal => {
            if !font::is_font_installed() {
                eprintln!("Nerd Font is not installed. Run `pnm font install` first.");
                std::process::exit(1);
            }

            let installations = font::find_terminals();
            if installations.is_empty() {
                eprintln!("No supported terminal emulator detected.");
                std::process::exit(1);
            }

            for install in &installations {
                if install.read_only {
                    if let Some(instructions) =
                        font::terminal::manual_instructions(&install.kind, font::NERD_FONT_FACE)
                    {
                        println!("\n{}: {instructions}", install.label);
                    }
                    continue;
                }
                print!("Configuring {}... ", install.label);
                match font::apply_terminal_font_to_defaults(install, font::NERD_FONT_FACE) {
                    Ok(()) => println!("✓"),
                    Err(e) => println!("✗ {e}"),
                }
            }
            println!("\nDone. Restart your terminal to see the new font.");
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
