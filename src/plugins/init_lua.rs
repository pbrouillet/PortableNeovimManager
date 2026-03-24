use crate::workload::WorkloadRegistry;
use std::path::Path;

/// Generates the full init.lua content for an instance.
/// `data_dir` is the absolute path to the instance's data/ directory (where lazy.nvim will be cloned).
/// `enabled_ids` are the workload IDs the user selected (optional workloads).
/// `disabled_features` are "WorkloadId/FeatureId" paths to exclude.
/// `extra_features` are "WorkloadId/FeatureId" paths to include even if their workload is off.
/// Base workloads from the registry are always included.
pub fn generate_init_lua(
    data_dir: &Path,
    registry: &WorkloadRegistry,
    enabled_ids: &[String],
    leader_key: &str,
) -> String {
    generate_init_lua_full(data_dir, registry, enabled_ids, &[], &[], leader_key)
}

/// Full version with feature-level overrides.
pub fn generate_init_lua_full(
    data_dir: &Path,
    registry: &WorkloadRegistry,
    enabled_ids: &[String],
    disabled_features: &[String],
    extra_features: &[String],
    leader_key: &str,
) -> String {
    let lazy_path = data_dir.join("lazy").join("lazy.nvim");
    let lazy_path_str = lazy_path.to_string_lossy().replace('\\', "/");

    // Escape the leader key for use in a Lua string literal
    let leader_lua = leader_key.replace('\\', "\\\\");

    let mut plugin_specs = Vec::new();
    let mut config_blocks = Vec::new();

    // Helper: collect plugins/config for a workload, respecting disabled_features
    let mut collect_workload = |workload: &crate::workload::Workload| {
        for feature in &workload.features {
            if !feature.default_enabled {
                continue;
            }
            let path = format!("{}/{}", workload.id, feature.id);
            if disabled_features.contains(&path) {
                continue;
            }
            plugin_specs.extend(feature.plugins.iter().cloned());
            if let Some(ref lua) = feature.config_lua {
                config_blocks.push(lua.clone());
            }
        }
    };

    // Always include base workloads (if not already in enabled_ids)
    for workload in registry.base() {
        if !enabled_ids.contains(&workload.id) {
            collect_workload(workload);
        }
    }

    // Add enabled workloads
    for id in enabled_ids {
        if let Some(workload) = registry.find_by_id(id) {
            collect_workload(workload);
        }
    }

    // Add extra features (from non-enabled workloads or non-default features)
    for path in extra_features {
        if let Some((_workload, feature)) = registry.find_feature_by_path(path) {
            plugin_specs.extend(feature.plugins.iter().cloned());
            if let Some(ref lua) = feature.config_lua {
                config_blocks.push(lua.clone());
            }
        }
    }

    let specs_lua = plugin_specs
        .iter()
        .map(|s| format!("  {s},"))
        .collect::<Vec<_>>()
        .join("\n");

    let feature_configs_lua = if config_blocks.is_empty() {
        String::new()
    } else {
        format!("\n{}\n", config_blocks.join("\n\n"))
    };

    format!(
        r#"-- Portable Neovim Manager — auto-generated init.lua
-- Modify with caution; this file is regenerated when features change.

-- Bootstrap lazy.nvim
local lazypath = "{lazy_path_str}"
if not (vim.uv or vim.loop).fs_stat(lazypath) then
  vim.fn.system({{
    "git",
    "clone",
    "--filter=blob:none",
    "https://github.com/folke/lazy.nvim.git",
    "--branch=stable",
    lazypath,
  }})
end
vim.opt.rtp:prepend(lazypath)

-- Leader key (before lazy setup)
vim.g.mapleader = "{leader_lua}"
vim.g.maplocalleader = "{leader_lua}"

-- Plugin specs
require("lazy").setup({{
{specs_lua}
}})
{feature_configs_lua}
-- Basic settings
vim.opt.number = true
vim.opt.relativenumber = true
vim.opt.termguicolors = true
vim.opt.signcolumn = "yes"
vim.opt.clipboard = "unnamedplus"
vim.opt.undofile = true
vim.opt.ignorecase = true
vim.opt.smartcase = true

-- Source user customizations if they exist
local user_config = vim.fn.stdpath("config") .. "/user.lua"
if (vim.uv or vim.loop).fs_stat(user_config) then
  dofile(user_config)
end
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workload::default_registry;
    use std::path::Path;

    #[test]
    fn test_generate_init_lua_contains_lazy_bootstrap() {
        let reg = default_registry();
        let data_dir = Path::new("/tmp/pnm_test_data");
        let output = generate_init_lua(data_dir, &reg, &[], " ");
        assert!(
            output.contains("lazy.nvim"),
            "output should mention lazy.nvim"
        );
        assert!(
            output.contains("clone"),
            "output should contain clone command"
        );
    }

    #[test]
    fn test_generate_init_lua_includes_base_features() {
        let reg = default_registry();
        let data_dir = Path::new("/tmp/pnm_test_data");
        let output = generate_init_lua(data_dir, &reg, &[], " ");
        assert!(
            output.contains("telescope"),
            "base features should include telescope"
        );
        assert!(
            output.contains("nvim-treesitter"),
            "base features should include treesitter"
        );
    }

    #[test]
    fn test_generate_init_lua_includes_selected_features() {
        let reg = default_registry();
        let data_dir = Path::new("/tmp/pnm_test_data");
        let enabled = vec!["Lsp".to_string()];
        let output = generate_init_lua(data_dir, &reg, &enabled, " ");
        assert!(
            output.contains("nvim-lspconfig"),
            "LSP feature should include nvim-lspconfig plugin"
        );
        assert!(
            output.contains("mason.nvim"),
            "LSP feature should include mason.nvim plugin"
        );
    }

    #[test]
    fn test_generate_init_lua_includes_base_feature_configs() {
        let reg = default_registry();
        let data_dir = Path::new("/tmp/pnm_test_data");
        let output = generate_init_lua(data_dir, &reg, &[], " ");
        assert!(
            output.contains("Telescope find_files"),
            "base Telescope config should include find_files keymap"
        );
        assert!(
            output.contains("Telescope buffers"),
            "base Telescope config should include buffers keymap"
        );
    }

    #[test]
    fn test_generate_init_lua_treeview_includes_neo_tree_config() {
        let reg = default_registry();
        let data_dir = Path::new("/tmp/pnm_test_data");
        let enabled = vec!["TreeView".to_string()];
        let output = generate_init_lua(data_dir, &reg, &enabled, " ");
        assert!(
            output.contains("neo-tree.nvim"),
            "TreeView should use neo-tree plugin"
        );
        assert!(
            output.contains(r#"require("neo-tree").setup"#),
            "TreeView should include neo-tree setup config"
        );
        assert!(
            output.contains("Neotree toggle"),
            "TreeView should include toggle keymap"
        );
        assert!(
            output.contains(r#""filesystem", "buffers", "git_status""#),
            "neo-tree should have filesystem, buffers, and git sources"
        );
    }

    #[test]
    fn test_generate_init_lua_tabs_includes_bufferline_config() {
        let reg = default_registry();
        let data_dir = Path::new("/tmp/pnm_test_data");
        let enabled = vec!["Tabs".to_string()];
        let output = generate_init_lua(data_dir, &reg, &enabled, " ");
        assert!(
            output.contains(r#"require("bufferline").setup"#),
            "Tabs should include bufferline setup config"
        );
        assert!(
            output.contains("neo-tree"),
            "bufferline should have neo-tree offset configured"
        );
        assert!(
            output.contains("BufferLineCycleNext"),
            "Tabs should include next buffer keymap"
        );
    }

    #[test]
    fn test_generate_init_lua_no_config_for_lsp() {
        let reg = default_registry();
        let data_dir = Path::new("/tmp/pnm_test_data");
        let enabled = vec!["Lsp".to_string()];
        let output = generate_init_lua(data_dir, &reg, &enabled, " ");
        assert!(
            !output.contains("-- Feature: Lsp"),
            "LSP should not have a feature config block"
        );
    }

    #[test]
    fn test_generate_init_lua_default_leader_key() {
        let reg = default_registry();
        let data_dir = Path::new("/tmp/pnm_test_data");
        let output = generate_init_lua(data_dir, &reg, &[], " ");
        assert!(
            output.contains(r#"vim.g.mapleader = " ""#),
            "default leader key should be space"
        );
    }

    #[test]
    fn test_generate_init_lua_custom_leader_key() {
        let reg = default_registry();
        let data_dir = Path::new("/tmp/pnm_test_data");
        let output = generate_init_lua(data_dir, &reg, &[], ",");
        assert!(
            output.contains(r#"vim.g.mapleader = ",""#),
            "leader key should be comma"
        );
        assert!(
            !output.contains(r#"vim.g.mapleader = " ""#),
            "should not contain space leader"
        );
    }

    #[test]
    fn test_generate_init_lua_backslash_leader_key() {
        let reg = default_registry();
        let data_dir = Path::new("/tmp/pnm_test_data");
        let output = generate_init_lua(data_dir, &reg, &[], "\\");
        assert!(
            output.contains(r#"vim.g.mapleader = "\\""#),
            "backslash leader should be escaped in Lua"
        );
    }
}
