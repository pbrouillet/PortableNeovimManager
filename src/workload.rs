use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Workload
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Workload {
    pub id: String,
    pub name: String,
    pub description: String,
    pub base: bool,
    pub depends_on: Vec<String>,
    pub plugins: Vec<String>,
    pub config_lua: Option<String>,
    pub cli_aliases: Vec<String>,
}

// ---------------------------------------------------------------------------
// WorkloadRegistry
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorkloadRegistry {
    pub workloads: Vec<Workload>,
}

impl WorkloadRegistry {
    pub fn all(&self) -> &[Workload] {
        &self.workloads
    }

    pub fn base(&self) -> Vec<&Workload> {
        self.workloads.iter().filter(|w| w.base).collect()
    }

    pub fn optional(&self) -> Vec<&Workload> {
        self.workloads.iter().filter(|w| !w.base).collect()
    }

    pub fn find_by_id(&self, id: &str) -> Option<&Workload> {
        self.workloads.iter().find(|w| w.id == id)
    }

    pub fn find_by_alias(&self, alias: &str) -> Option<&Workload> {
        let lower = alias.to_lowercase();
        self.workloads
            .iter()
            .find(|w| w.cli_aliases.iter().any(|a| a.to_lowercase() == lower))
    }

    /// Returns IDs of workloads that depend on the given workload ID.
    pub fn dependents_of(&self, id: &str) -> Vec<String> {
        self.workloads
            .iter()
            .filter(|w| w.depends_on.iter().any(|dep| dep == id))
            .map(|w| w.id.clone())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

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

fn load_workloads_from(path: &Path) -> Result<WorkloadRegistry, Box<dyn std::error::Error>> {
    let data = std::fs::read_to_string(path)?;
    let registry: WorkloadRegistry = serde_json::from_str(&data)?;
    Ok(registry)
}

fn write_workloads_file(
    path: &Path,
    registry: &WorkloadRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(registry)?;
    std::fs::write(path, json)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Built-in defaults
// ---------------------------------------------------------------------------

pub fn default_registry() -> WorkloadRegistry {
    WorkloadRegistry {
        workloads: default_workloads(),
    }
}

fn default_workloads() -> Vec<Workload> {
    vec![
        Workload {
            id: "Lsp".into(),
            name: "LSP".into(),
            description: "Language Server Protocol (mason + lspconfig)".into(),
            base: false,
            depends_on: vec![],
            plugins: vec![
                r#"{ "neovim/nvim-lspconfig" }"#.into(),
                r#"{ "williamboman/mason.nvim", config = true }"#.into(),
                r#"{ "williamboman/mason-lspconfig.nvim", config = true }"#.into(),
            ],
            config_lua: None,
            cli_aliases: vec!["lsp".into()],
        },
        Workload {
            id: "Dap".into(),
            name: "DAP".into(),
            description: "Debug Adapter Protocol (nvim-dap + mason)".into(),
            base: false,
            depends_on: vec![],
            plugins: vec![
                r#"{ "mfussenegger/nvim-dap" }"#.into(),
                r#"{ "rcarriga/nvim-dap-ui", dependencies = { "mfussenegger/nvim-dap", "nvim-neotest/nvim-nio" }, config = true }"#.into(),
                r#"{ "jay-babu/mason-nvim-dap.nvim", config = true }"#.into(),
            ],
            config_lua: None,
            cli_aliases: vec!["dap".into()],
        },
        Workload {
            id: "TreeView".into(),
            name: "TreeView".into(),
            description: "Left explorer panel (neo-tree)".into(),
            base: false,
            depends_on: vec![],
            plugins: vec![
                r#"{ "nvim-neo-tree/neo-tree.nvim", branch = "v3.x", dependencies = { "nvim-lua/plenary.nvim", "nvim-tree/nvim-web-devicons", "MunifTanjim/nui.nvim" } }"#.into(),
            ],
            config_lua: Some(r#"-- Feature: TreeView (neo-tree)
-- Detect Nerd Font availability
vim.g.have_nerd_font = (function()
  -- Check if a known Nerd Font glyph renders at double width
  local ok, _ = pcall(function()
    return vim.fn.strdisplaywidth("󰉓") > 1
  end)
  return ok and vim.fn.strdisplaywidth("󰉓") > 1
end)()

local nf = vim.g.have_nerd_font
require("neo-tree").setup({
  close_if_last_window = true,
  popup_border_style = "rounded",
  enable_git_status = true,
  enable_diagnostics = true,
  sources = { "filesystem", "buffers", "git_status" },
  source_selector = {
    winbar = true,
    content_layout = "center",
    sources = {
      { source = "filesystem", display_name = nf and " 󰉓 Files " or " [Files] " },
      { source = "buffers", display_name = nf and " 󰈚 Buffers " or " [Buffers] " },
      { source = "git_status", display_name = nf and " 󰊢 Git " or " [Git] " },
    },
  },
  default_component_configs = {
    icon = {
      folder_closed = nf and "󰉋" or "+",
      folder_open = nf and "󰝰" or "-",
      folder_empty = nf and "󰉖" or "~",
      default = nf and "󰈙" or "*",
    },
    indent = {
      with_expanders = true,
    },
  },
  window = {
    position = "left",
    width = 35,
    mappings = {
      ["<space>"] = "none",
    },
  },
  filesystem = {
    follow_current_file = { enabled = true },
    use_libuv_file_watcher = true,
    filtered_items = {
      visible = false,
      hide_dotfiles = false,
      hide_gitignored = true,
    },
  },
  buffers = {
    follow_current_file = { enabled = true },
    group_empty_dirs = true,
  },
})
vim.keymap.set("n", "<leader>e", "<cmd>Neotree toggle<cr>", { desc = "Toggle explorer" })
vim.keymap.set("n", "<leader>be", "<cmd>Neotree buffers toggle<cr>", { desc = "Toggle buffer explorer" })
vim.keymap.set("n", "<leader>ge", "<cmd>Neotree git_status toggle<cr>", { desc = "Toggle git explorer" })"#.into()),
            cli_aliases: vec!["treeview".into(), "tree-view".into(), "tree".into()],
        },
        Workload {
            id: "Tabs".into(),
            name: "Tabs".into(),
            description: "Top tabbed editor bar (bufferline)".into(),
            base: false,
            depends_on: vec![],
            plugins: vec![
                r#"{ "akinsho/bufferline.nvim", version = "*", dependencies = { "nvim-tree/nvim-web-devicons" } }"#.into(),
            ],
            config_lua: Some(r#"-- Feature: Tabs (bufferline)
require("bufferline").setup({
  options = {
    mode = "buffers",
    diagnostics = "nvim_lsp",
    show_buffer_close_icons = true,
    show_close_icon = false,
    separator_style = "slant",
    always_show_bufferline = true,
    offsets = {
      {
        filetype = "neo-tree",
        text = "Explorer",
        highlight = "Directory",
        separator = true,
      },
    },
  },
})
vim.keymap.set("n", "<leader>bn", "<cmd>BufferLineCycleNext<cr>", { desc = "Next buffer" })
vim.keymap.set("n", "<leader>bp", "<cmd>BufferLineCyclePrev<cr>", { desc = "Previous buffer" })
vim.keymap.set("n", "<leader>bd", "<cmd>bdelete<cr>", { desc = "Delete buffer" })
vim.keymap.set("n", "<leader>bo", "<cmd>BufferLineCloseOthers<cr>", { desc = "Close other buffers" })"#.into()),
            cli_aliases: vec!["tabs".into(), "tabline".into(), "bufferline".into()],
        },
        Workload {
            id: "Telescope".into(),
            name: "Telescope".into(),
            description: "Fuzzy finder (always on)".into(),
            base: true,
            depends_on: vec![],
            plugins: vec![
                r#"{ "nvim-telescope/telescope.nvim", tag = "0.1.8", dependencies = { "nvim-lua/plenary.nvim" } }"#.into(),
            ],
            config_lua: Some(r#"-- Feature: Telescope
require("telescope").setup()
vim.keymap.set("n", "<leader>ff", "<cmd>Telescope find_files<cr>", { desc = "Find files" })
vim.keymap.set("n", "<leader>fg", "<cmd>Telescope live_grep<cr>", { desc = "Live grep" })
vim.keymap.set("n", "<leader>fb", "<cmd>Telescope buffers<cr>", { desc = "Find buffers" })
vim.keymap.set("n", "<leader>fh", "<cmd>Telescope help_tags<cr>", { desc = "Help tags" })"#.into()),
            cli_aliases: vec!["telescope".into()],
        },
        Workload {
            id: "Treesitter".into(),
            name: "Treesitter".into(),
            description: "Syntax highlighting (always on)".into(),
            base: true,
            depends_on: vec![],
            plugins: vec![
                r#"{ "nvim-treesitter/nvim-treesitter", build = ":TSUpdate", config = function() require("nvim-treesitter.install").prefer_git = false end }"#.into(),
            ],
            config_lua: None,
            cli_aliases: vec!["treesitter".into()],
        },
        Workload {
            id: "OmniSharp".into(),
            name: "OmniSharp".into(),
            description: "C# language server (OmniSharp)".into(),
            base: false,
            depends_on: vec!["Lsp".into()],
            plugins: vec![
                r#"{ "Hoffs/omnisharp-extended-lsp.nvim" }"#.into(),
            ],
            config_lua: None,
            cli_aliases: vec!["omnisharp".into(), "csharp".into(), "cs".into()],
        },
        Workload {
            id: "Node".into(),
            name: "Node".into(),
            description: "TypeScript/JavaScript language server (ts_ls)".into(),
            base: false,
            depends_on: vec!["Lsp".into()],
            plugins: vec![
                r#"{ "pmizio/typescript-tools.nvim", dependencies = { "nvim-lua/plenary.nvim", "neovim/nvim-lspconfig" }, config = true }"#.into(),
            ],
            config_lua: None,
            cli_aliases: vec!["node".into(), "typescript".into(), "ts".into(), "javascript".into(), "js".into()],
        },
        Workload {
            id: "Python".into(),
            name: "Python".into(),
            description: "Python language server (pyright)".into(),
            base: false,
            depends_on: vec!["Lsp".into()],
            plugins: vec![
                r#"{ "neovim/nvim-lspconfig" }"#.into(),
            ],
            config_lua: None,
            cli_aliases: vec!["python".into(), "py".into()],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registry_has_all_workloads() {
        let reg = default_registry();
        assert_eq!(reg.all().len(), 9);
    }

    #[test]
    fn test_base_workloads() {
        let reg = default_registry();
        let base: Vec<&str> = reg.base().iter().map(|w| w.id.as_str()).collect();
        assert!(base.contains(&"Telescope"));
        assert!(base.contains(&"Treesitter"));
        assert_eq!(base.len(), 2);
    }

    #[test]
    fn test_optional_workloads() {
        let reg = default_registry();
        let optional: Vec<&str> = reg.optional().iter().map(|w| w.id.as_str()).collect();
        assert_eq!(optional.len(), 7);
        assert!(!optional.contains(&"Telescope"));
        assert!(!optional.contains(&"Treesitter"));
    }

    #[test]
    fn test_find_by_id() {
        let reg = default_registry();
        let lsp = reg.find_by_id("Lsp").unwrap();
        assert_eq!(lsp.name, "LSP");
    }

    #[test]
    fn test_find_by_alias() {
        let reg = default_registry();
        assert_eq!(reg.find_by_alias("ts").unwrap().id, "Node");
        assert_eq!(reg.find_by_alias("py").unwrap().id, "Python");
        assert_eq!(reg.find_by_alias("tree").unwrap().id, "TreeView");
        assert_eq!(reg.find_by_alias("csharp").unwrap().id, "OmniSharp");
    }

    #[test]
    fn test_dependents_of() {
        let reg = default_registry();
        let dependents = reg.dependents_of("Lsp");
        assert!(dependents.contains(&"OmniSharp".to_string()));
        assert!(dependents.contains(&"Node".to_string()));
        assert!(dependents.contains(&"Python".to_string()));
    }

    #[test]
    fn test_json_round_trip() {
        let reg = default_registry();
        let json = serde_json::to_string_pretty(&reg).unwrap();
        let loaded: WorkloadRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.workloads.len(), reg.workloads.len());
        for (a, b) in reg.workloads.iter().zip(loaded.workloads.iter()) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.plugins.len(), b.plugins.len());
        }
    }
}
