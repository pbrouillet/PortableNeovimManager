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
    mason_packages: &[String],
    init_lua_pre: Option<&str>,
    init_lua_post: Option<&str>,
) -> String {
    generate_init_lua_full(data_dir, registry, enabled_ids, &[], &[], leader_key, mason_packages, init_lua_pre, init_lua_post)
}

/// Full version with feature-level overrides.
pub fn generate_init_lua_full(
    data_dir: &Path,
    registry: &WorkloadRegistry,
    enabled_ids: &[String],
    disabled_features: &[String],
    extra_features: &[String],
    leader_key: &str,
    mason_packages: &[String],
    init_lua_pre: Option<&str>,
    init_lua_post: Option<&str>,
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

    let mason_ensure_lua = if mason_packages.is_empty() {
        String::new()
    } else {
        let names = mason_packages
            .iter()
            .map(|n| format!(r#"  "{}""#, n))
            .collect::<Vec<_>>()
            .join(",\n");
        format!(
            r#"
-- Mason ensure_installed (managed by pnm marketplace)
local mason_ensure = {{
{names}
}}
vim.defer_fn(function()
  local ok, mr = pcall(require, "mason-registry")
  if not ok then return end
  mr.refresh(function()
    for _, name in ipairs(mason_ensure) do
      local p_ok, pkg = pcall(mr.get_package, name)
      if p_ok and pkg and not pkg:is_installed() then
        pkg:install()
      end
    end
  end)
end, 1000)
"#
        )
    };

    let pre_plugins_lua = match init_lua_pre {
        Some(s) if !s.trim().is_empty() => format!(
            "\n-- Init overrides: pre-plugins (managed by pnm)\n{}\n",
            s.trim()
        ),
        _ => String::new(),
    };

    let post_plugins_lua = match init_lua_post {
        Some(s) if !s.trim().is_empty() => format!(
            "\n-- Init overrides: post-plugins (managed by pnm)\n{}\n",
            s.trim()
        ),
        _ => String::new(),
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

-- JavaScript runtime override (managed by pnm)
-- When pnm launches Neovim with a JS runtime shim (e.g. Bun), the shims/
-- directory contains a node[.exe] that delegates to the alternative runtime.
-- This block tells plugins like copilot.vim and Neovim's node provider to use
-- our shim instead of their own Node discovery logic.
do
  local data_dir = vim.fn.stdpath("data")
  local instance_dir = data_dir:match("(.+)[/\\]data$") or (data_dir .. "/..")
  local node_shim = instance_dir .. "/shims/node"
  if vim.fn.has("win32") == 1 then node_shim = node_shim .. ".exe" end
  if vim.fn.executable(node_shim) == 1 then
    vim.g.copilot_node_command = node_shim
    vim.g.node_host_prog = node_shim
  end
end
{pre_plugins_lua}
-- Plugin specs
require("lazy").setup({{
{specs_lua}
}})
{feature_configs_lua}{mason_ensure_lua}{post_plugins_lua}
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
        let output = generate_init_lua(data_dir, &reg, &[], " ", &[], None, None);
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
        let output = generate_init_lua(data_dir, &reg, &[], " ", &[], None, None);
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
        let output = generate_init_lua(data_dir, &reg, &enabled, " ", &[], None, None);
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
        let output = generate_init_lua(data_dir, &reg, &[], " ", &[], None, None);
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
        let output = generate_init_lua(data_dir, &reg, &enabled, " ", &[], None, None);
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
        let output = generate_init_lua(data_dir, &reg, &enabled, " ", &[], None, None);
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
        let output = generate_init_lua(data_dir, &reg, &enabled, " ", &[], None, None);
        assert!(
            !output.contains("-- Feature: Lsp"),
            "LSP should not have a feature config block"
        );
    }

    #[test]
    fn test_generate_init_lua_default_leader_key() {
        let reg = default_registry();
        let data_dir = Path::new("/tmp/pnm_test_data");
        let output = generate_init_lua(data_dir, &reg, &[], " ", &[], None, None);
        assert!(
            output.contains(r#"vim.g.mapleader = " ""#),
            "default leader key should be space"
        );
    }

    #[test]
    fn test_generate_init_lua_custom_leader_key() {
        let reg = default_registry();
        let data_dir = Path::new("/tmp/pnm_test_data");
        let output = generate_init_lua(data_dir, &reg, &[], ",", &[], None, None);
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
        let output = generate_init_lua(data_dir, &reg, &[], "\\", &[], None, None);
        assert!(
            output.contains(r#"vim.g.mapleader = "\\""#),
            "backslash leader should be escaped in Lua"
        );
    }

    #[test]
    fn test_generate_init_lua_contains_runtime_override_block() {
        let reg = default_registry();
        let data_dir = Path::new("/tmp/pnm_test_data");
        let output = generate_init_lua(data_dir, &reg, &[], " ", &[], None, None);
        assert!(
            output.contains("vim.g.copilot_node_command"),
            "should contain copilot_node_command override"
        );
        assert!(
            output.contains("vim.g.node_host_prog"),
            "should contain node_host_prog override"
        );
        assert!(
            output.contains("shims/node"),
            "should reference the shims/node path"
        );
    }

    #[test]
    fn test_generate_init_lua_injects_pre_post_overrides() {
        let reg = default_registry();
        let data_dir = Path::new("/tmp/pnm_test_data");
        let output = generate_init_lua(
            data_dir,
            &reg,
            &[],
            " ",
            &[],
            Some("vim.opt.mouse = ''"),
            Some("vim.cmd('Neotree show')"),
        );
        assert!(
            output.contains("vim.opt.mouse = ''"),
            "pre-plugins override should be injected"
        );
        assert!(
            output.contains("vim.cmd('Neotree show')"),
            "post-plugins override should be injected"
        );
        // Pre should come before lazy.setup, post should come after
        let pre_pos = output.find("vim.opt.mouse").unwrap();
        let lazy_pos = output.find("require(\"lazy\").setup").unwrap();
        let post_pos = output.find("Neotree show").unwrap();
        assert!(
            pre_pos < lazy_pos,
            "pre-plugins should come before lazy.setup"
        );
        assert!(
            post_pos > lazy_pos,
            "post-plugins should come after lazy.setup"
        );
    }

    #[test]
    fn test_generate_init_lua_no_overrides_when_none() {
        let reg = default_registry();
        let data_dir = Path::new("/tmp/pnm_test_data");
        let output = generate_init_lua(data_dir, &reg, &[], " ", &[], None, None);
        assert!(
            !output.contains("Init overrides"),
            "should not contain override markers when None"
        );
    }
}
