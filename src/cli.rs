use clap::{Parser, Subcommand};

use crate::workload::WorkloadRegistry;

#[derive(Parser, Debug)]
#[command(
    name = "pnm",
    about = "Portable Neovim Manager — manage multiple self-contained Neovim installations"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a new portable Neovim instance
    #[command(after_long_help = "EXAMPLES:\n  pnm create my-env\n  pnm create my-env --version v0.10.4\n  pnm create my-env --features lsp,treeview,tabs\n  pnm create my-env --features @ide-core\n  pnm create my-env --features @ide-full,python")]
    Create {
        /// Name for the instance
        name: String,
        /// Neovim version tag (e.g., "v0.10.4", "nightly"). Defaults to latest stable.
        #[arg(short, long)]
        version: Option<String>,
        /// Features to enable (lsp, dap, treeview, tabs). Comma-separated.
        #[arg(short, long, value_delimiter = ',')]
        features: Option<Vec<String>>,
        /// JavaScript runtime ("bun" or path). Overrides system Node for plugins.
        #[arg(long)]
        js_runtime: Option<String>,
    },
    /// List all portable Neovim instances
    List,
    /// Show detailed info about an instance
    Info {
        /// Instance name
        name: String,
    },
    /// Launch a portable Neovim instance
    #[command(after_long_help = "EXAMPLES:\n  pnm launch my-env\n  pnm launch my-env -- --clean\n  pnm launch my-env -- file.txt")]
    Launch {
        /// Instance name
        name: String,
        /// Extra arguments to pass to nvim
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Update an instance to the latest Neovim release
    Update {
        /// Instance name
        name: String,
        /// Specific version tag to update to
        #[arg(short, long)]
        version: Option<String>,
    },
    /// Delete a portable Neovim instance
    #[command(after_long_help = "EXAMPLES:\n  pnm delete my-env\n  pnm delete my-env -y     # Skip confirmation")]
    Delete {
        /// Instance name
        name: String,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Toggle features on an instance
    #[command(after_long_help = "EXAMPLES:\n  pnm features my-env --enable lsp,treeview\n  pnm features my-env --enable @ide-core\n  pnm features my-env --disable dap\n  pnm features my-env --enable @ide-core --disable dap")]
    Features {
        /// Instance name
        name: String,
        /// Features to enable (comma-separated). Use @preset for presets (e.g. @ide-core).
        #[arg(long, value_delimiter = ',')]
        enable: Option<Vec<String>>,
        /// Features to disable (comma-separated)
        #[arg(long, value_delimiter = ',')]
        disable: Option<Vec<String>>,
    },
    /// Initialize a default settings.json next to the executable
    Init,
    /// Show tutorials for features and workflows
    #[command(after_long_help = "EXAMPLES:\n  pnm tutorial              # List all topics\n  pnm tutorial lsp          # Show LSP tutorial\n  pnm tutorial python       # Show Python setup guide\n  pnm tutorial leader-key   # Understand the leader key")]
    Tutorial {
        /// Topic to show (e.g. "leader-key", "python", "lsp"). Lists all if omitted.
        topic: Option<String>,
    },
    /// Browse and install LSP servers, DAP adapters, formatters, and linters
    #[command(after_long_help = "EXAMPLES:\n  pnm marketplace search python\n  pnm marketplace list --category lsp\n  pnm marketplace install myenv pyright debugpy\n  pnm marketplace remove myenv pyright\n  pnm marketplace refresh")]
    Marketplace {
        #[command(subcommand)]
        action: MarketplaceAction,
    },
    /// Open the interactive TUI
    Tui,
    /// Show memory usage of a running Neovim instance
    #[command(after_long_help = "EXAMPLES:\n  pnm monitor my-env              # Snapshot memory usage\n  pnm monitor my-env --no-lua     # Skip Lua heap query")]
    Monitor {
        /// Instance name
        name: String,
        /// Skip Lua heap memory query via RPC
        #[arg(long)]
        no_lua: bool,
    },
    /// Get or set the JavaScript runtime for an instance
    #[command(after_long_help = "EXAMPLES:\n  pnm runtime my-env              # Show current runtime\n  pnm runtime my-env --set bun    # Use Bun instead of Node\n  pnm runtime my-env --set /path/to/bun  # Custom path\n  pnm runtime my-env --unset      # Revert to system Node")]
    Runtime {
        /// Instance name
        name: String,
        /// Set the JavaScript runtime ("bun" or an absolute path)
        #[arg(long)]
        set: Option<String>,
        /// Clear per-instance runtime override (use global default)
        #[arg(long)]
        unset: bool,
    },
    /// View, edit, or reset the init.lua configuration overrides for an instance
    #[command(name = "init-config", after_long_help = "EXAMPLES:\n  pnm init-config my-env                # Show current overrides\n  pnm init-config my-env --edit-pre     # Edit pre-plugins Lua in $EDITOR\n  pnm init-config my-env --edit-post    # Edit post-plugins Lua in $EDITOR\n  pnm init-config my-env --reset        # Reset to smart defaults based on features")]
    InitConfig {
        /// Instance name
        name: String,
        /// Open $EDITOR to edit pre-plugins Lua
        #[arg(long)]
        edit_pre: bool,
        /// Open $EDITOR to edit post-plugins Lua
        #[arg(long)]
        edit_post: bool,
        /// Reset overrides to smart defaults based on current features
        #[arg(long)]
        reset: bool,
    },
    /// Install and configure Nerd Font for Neovim
    #[command(after_long_help = "EXAMPLES:\n  pnm font install              # Download, install, and configure Windows Terminal\n  pnm font install --no-terminal # Install font only, skip terminal configuration\n  pnm font status                # Check if font is installed and terminal configured\n  pnm font configure-terminal    # Configure Windows Terminal without reinstalling\n  pnm font reset                 # Remove installed font files and registry entries")]
    Font {
        #[command(subcommand)]
        action: FontAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum FontAction {
    /// Download, install, and configure JetBrainsMono Nerd Font
    Install {
        /// Skip Windows Terminal configuration
        #[arg(long)]
        no_terminal: bool,
    },
    /// Check if Nerd Font is installed and terminal is configured
    Status,
    /// Configure Windows Terminal to use the installed Nerd Font
    ConfigureTerminal,
    /// Remove installed Nerd Font files and clean up registry entries
    Reset,
}

#[derive(Subcommand, Debug)]
pub enum MarketplaceAction {
    /// Search packages by name, language, or description
    Search {
        /// Search query
        query: String,
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
    },
    /// List available packages
    List {
        /// Filter by category (lsp, dap, formatter, linter)
        #[arg(short, long)]
        category: Option<String>,
        /// Filter by language
        #[arg(short, long)]
        language: Option<String>,
    },
    /// Install mason packages on an instance
    Install {
        /// Instance name
        name: String,
        /// Package names to install
        #[arg(required = true)]
        packages: Vec<String>,
    },
    /// Remove mason packages from an instance
    Remove {
        /// Instance name
        name: String,
        /// Package names to remove
        #[arg(required = true)]
        packages: Vec<String>,
    },
    /// Force refresh the cached package registry
    Refresh,
    /// Show detailed info about a package
    Info {
        /// Package name
        name: String,
    },
}

/// Resolves feature/workload identifiers to workload IDs.
/// Supports:
///   - Workload aliases (e.g. "lsp", "dap", "treeview")
///   - @preset syntax (e.g. "@ide-core" → all workloads in that preset)
///   - Direct workload IDs (e.g. "Lsp")
pub fn parse_features(features: &[String], registry: &WorkloadRegistry) -> Vec<String> {
    let mut parsed: Vec<String> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();

    for f in features {
        if let Some(preset_id) = f.strip_prefix('@') {
            if let Some(preset) = registry.find_preset(preset_id) {
                for wl in &preset.workloads {
                    if !parsed.contains(wl) {
                        parsed.push(wl.clone());
                    }
                }
            } else {
                unknown.push(format!("@{preset_id}"));
            }
            continue;
        }
        if let Some(workload) = registry.find_by_alias(f) {
            if !parsed.contains(&workload.id) {
                parsed.push(workload.id.clone());
            }
        } else {
            unknown.push(f.clone());
        }
    }

    if !unknown.is_empty() {
        eprintln!(
            "Warning: unknown feature(s) skipped: {}",
            unknown.join(", ")
        );
    }

    // Resolve transitive dependencies
    let resolved = registry.resolve_dependencies(&parsed);

    // Report auto-added dependencies
    for id in &resolved {
        if !parsed.contains(id) {
            if let Some(w) = registry.find_by_id(id) {
                eprintln!("Auto-enabled {} (required by dependency)", w.name);
            }
        }
    }

    resolved
}

/// Validates an instance name. Returns Ok(()) if valid, Err with message if not.
pub fn validate_instance_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Instance name cannot be empty.".to_string());
    }
    if name.len() > 64 {
        return Err("Instance name must be 64 characters or fewer.".to_string());
    }
    if name.starts_with('.') || name.starts_with('-') {
        return Err(format!(
            "Instance name '{}' cannot start with '.' or '-'.",
            name
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(format!(
            "Instance name '{}' contains invalid characters. Use only letters, digits, hyphens, underscores, and dots.",
            name
        ));
    }
    // Reserved names on Windows
    let reserved = [
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7",
        "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ];
    if reserved.contains(&name.to_lowercase().as_str()) {
        return Err(format!("Instance name '{}' is a reserved name.", name));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workload::default_registry;

    #[test]
    fn test_parse_features_known() {
        let reg = default_registry();
        let input: Vec<String> = vec![
            "lsp",
            "dap",
            "treeview",
            "tabs",
            "omnisharp",
            "node",
            "python",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let result = parse_features(&input, &reg);
        assert_eq!(
            result,
            vec![
                "Lsp",
                "Dap",
                "TreeView",
                "Tabs",
                "OmniSharp",
                "Node",
                "Python"
            ]
        );
    }

    #[test]
    fn test_parse_features_aliases() {
        let reg = default_registry();
        let aliases: Vec<String> = vec![
            "tree-view",
            "tree",
            "tabline",
            "bufferline",
            "csharp",
            "cs",
            "typescript",
            "ts",
            "js",
            "py",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let result = parse_features(&aliases, &reg);
        assert_eq!(
            result,
            vec![
                "TreeView",
                "Tabs",
                "Lsp",
                "OmniSharp",
                "Node",
                "Python",
            ]
        );
    }

    #[test]
    fn test_parse_features_unknown_skipped() {
        let reg = default_registry();
        let input: Vec<String> = vec!["lsp", "unknown", "magic", "dap"]
            .into_iter()
            .map(String::from)
            .collect();
        let result = parse_features(&input, &reg);
        assert_eq!(result, vec!["Lsp", "Dap"]);
    }

    #[test]
    fn test_parse_features_case_insensitive() {
        let reg = default_registry();
        let input: Vec<String> = vec!["LSP", "Lsp", "lsp"]
            .into_iter()
            .map(String::from)
            .collect();
        let result = parse_features(&input, &reg);
        assert_eq!(result, vec!["Lsp"]);
    }

    #[test]
    fn test_parse_features_preset() {
        let reg = default_registry();
        let input: Vec<String> = vec!["@ide-core"]
            .into_iter()
            .map(String::from)
            .collect();
        let result = parse_features(&input, &reg);
        assert_eq!(result, vec!["Lsp", "Completion", "Git", "TreeView", "Tabs", "Editing", "Statusline"]);
    }

    #[test]
    fn test_parse_features_preset_minimal_is_empty() {
        let reg = default_registry();
        let input: Vec<String> = vec!["@minimal"]
            .into_iter()
            .map(String::from)
            .collect();
        let result = parse_features(&input, &reg);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_features_mixed_preset_and_aliases() {
        let reg = default_registry();
        let input: Vec<String> = vec!["@ide-core", "python"]
            .into_iter()
            .map(String::from)
            .collect();
        let result = parse_features(&input, &reg);
        assert_eq!(result, vec!["Lsp", "Completion", "Git", "TreeView", "Tabs", "Editing", "Statusline", "Python"]);
    }

    #[test]
    fn test_validate_instance_name_valid() {
        assert!(validate_instance_name("my-env").is_ok());
        assert!(validate_instance_name("test_setup").is_ok());
        assert!(validate_instance_name("env.1").is_ok());
        assert!(validate_instance_name("a").is_ok());
    }

    #[test]
    fn test_validate_instance_name_invalid() {
        assert!(validate_instance_name("").is_err());
        assert!(validate_instance_name(".hidden").is_err());
        assert!(validate_instance_name("-start").is_err());
        assert!(validate_instance_name("has space").is_err());
        assert!(validate_instance_name("has/slash").is_err());
        assert!(validate_instance_name("CON").is_err());
        assert!(validate_instance_name("nul").is_err());
    }
}
