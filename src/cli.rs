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
    Create {
        /// Name for the instance
        name: String,
        /// Neovim version tag (e.g., "v0.10.4", "nightly"). Defaults to latest stable.
        #[arg(short, long)]
        version: Option<String>,
        /// Features to enable (lsp, dap, treeview, tabs). Comma-separated.
        #[arg(short, long, value_delimiter = ',')]
        features: Option<Vec<String>>,
    },
    /// List all portable Neovim instances
    List,
    /// Show detailed info about an instance
    Info {
        /// Instance name
        name: String,
    },
    /// Launch a portable Neovim instance
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
    Delete {
        /// Instance name
        name: String,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Toggle features on an instance
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
    Tutorial {
        /// Topic to show (e.g. "leader-key", "python", "lsp"). Lists all if omitted.
        topic: Option<String>,
    },
    /// Open the interactive TUI
    Tui,
}

/// Resolves feature/workload identifiers to workload IDs.
/// Supports:
///   - Workload aliases (e.g. "lsp", "dap", "treeview")
///   - @preset syntax (e.g. "@ide-core" → all workloads in that preset)
///   - Direct workload IDs (e.g. "Lsp")
pub fn parse_features(features: &[String], registry: &WorkloadRegistry) -> Vec<String> {
    features
        .iter()
        .flat_map(|f| {
            if let Some(preset_id) = f.strip_prefix('@') {
                if let Some(preset) = registry.find_preset(preset_id) {
                    return preset.workloads.clone();
                } else {
                    eprintln!("Warning: unknown preset '@{preset_id}', skipping");
                    return vec![];
                }
            }
            if let Some(workload) = registry.find_by_alias(f) {
                vec![workload.id.clone()]
            } else {
                eprintln!("Warning: unknown feature '{f}', skipping");
                vec![]
            }
        })
        .collect()
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
                "TreeView",
                "Tabs",
                "Tabs",
                "OmniSharp",
                "OmniSharp",
                "Node",
                "Node",
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
        assert_eq!(result, vec!["Lsp", "Lsp", "Lsp"]);
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
}
