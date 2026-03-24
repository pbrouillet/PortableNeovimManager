use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Feature — an individual toggleable unit within a workload
// ---------------------------------------------------------------------------

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Feature {
    pub id: String,
    pub name: String,
    pub description: String,
    pub plugins: Vec<String>,
    #[serde(default)]
    pub config_lua: Option<String>,
    /// Whether this feature is on by default when its parent workload is enabled.
    #[serde(default = "default_true")]
    pub default_enabled: bool,
}

// ---------------------------------------------------------------------------
// Preset — a named set of workloads for quick bulk-enable
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub workloads: Vec<String>,
}

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
    /// Features within this workload (new format).
    #[serde(default)]
    pub features: Vec<Feature>,
    /// DEPRECATED — kept for backward compat with old workloads.json.
    /// Use `features` instead. Auto-migrated on load via `normalize()`.
    #[serde(default, skip_serializing)]
    pub plugins: Vec<String>,
    /// DEPRECATED — kept for backward compat with old workloads.json.
    #[serde(default, skip_serializing)]
    pub config_lua: Option<String>,
    pub cli_aliases: Vec<String>,
    #[serde(default)]
    pub tutorial: Option<String>,
}

impl Workload {
    /// Migrate old-format workloads (plugins/config_lua on Workload) to the
    /// new feature-based format.  No-op if features are already populated.
    pub fn normalize(&mut self) {
        if self.features.is_empty() && !self.plugins.is_empty() {
            self.features = vec![Feature {
                id: self.id.to_lowercase(),
                name: self.name.clone(),
                description: self.description.clone(),
                plugins: std::mem::take(&mut self.plugins),
                config_lua: self.config_lua.take(),
                default_enabled: true,
            }];
        }
    }

    /// Collect all plugin specs from all features.
    pub fn all_plugins(&self) -> Vec<String> {
        self.features
            .iter()
            .flat_map(|f| f.plugins.iter().cloned())
            .collect()
    }

    /// Collect all config_lua blocks from all features.
    pub fn all_config_lua(&self) -> Vec<String> {
        self.features
            .iter()
            .filter_map(|f| f.config_lua.clone())
            .collect()
    }

    /// Find a feature by its id within this workload.
    pub fn find_feature(&self, feature_id: &str) -> Option<&Feature> {
        self.features.iter().find(|f| f.id == feature_id)
    }
}

// ---------------------------------------------------------------------------
// Tutorial
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tutorial {
    pub id: String,
    pub title: String,
    pub content: String,
}

// ---------------------------------------------------------------------------
// WorkloadRegistry
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorkloadRegistry {
    pub workloads: Vec<Workload>,
    #[serde(default)]
    pub presets: Vec<Preset>,
    #[serde(default)]
    pub tutorials: Vec<Tutorial>,
}

impl WorkloadRegistry {
    /// Normalize all workloads (migrate old format if needed).
    pub fn normalize(&mut self) {
        for w in &mut self.workloads {
            w.normalize();
        }
    }

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

    /// Find a general tutorial by its id.
    pub fn find_tutorial_by_id(&self, id: &str) -> Option<&Tutorial> {
        let lower = id.to_lowercase();
        self.tutorials.iter().find(|t| t.id.to_lowercase() == lower)
    }

    /// Returns a combined list of all tutorial topics: general tutorials first,
    /// then workloads that have a tutorial.  Each entry is (id, title).
    pub fn all_tutorial_topics(&self) -> Vec<(String, String)> {
        let mut topics: Vec<(String, String)> = self
            .tutorials
            .iter()
            .map(|t| (t.id.clone(), t.title.clone()))
            .collect();
        for w in &self.workloads {
            if w.tutorial.is_some() {
                topics.push((w.id.clone(), format!("{} — {}", w.name, w.description)));
            }
        }
        topics
    }

    /// Look up tutorial content by topic id.  Checks general tutorials first,
    /// then workload ids, then workload aliases.
    pub fn tutorial_content(&self, topic: &str) -> Option<(String, String)> {
        // General tutorials
        if let Some(t) = self.find_tutorial_by_id(topic) {
            return Some((t.title.clone(), t.content.clone()));
        }
        // Workload by id
        if let Some(w) = self.find_by_id(topic) {
            if let Some(ref content) = w.tutorial {
                return Some((w.name.clone(), content.clone()));
            }
        }
        // Workload by alias
        if let Some(w) = self.find_by_alias(topic) {
            if let Some(ref content) = w.tutorial {
                return Some((w.name.clone(), content.clone()));
            }
        }
        None
    }

    /// Find a preset by its id.
    pub fn find_preset(&self, id: &str) -> Option<&Preset> {
        let lower = id.to_lowercase();
        self.presets.iter().find(|p| p.id.to_lowercase() == lower)
    }

    /// Resolve a "WorkloadId/FeatureId" path to the workload and feature.
    pub fn find_feature_by_path(&self, path: &str) -> Option<(&Workload, &Feature)> {
        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.len() != 2 {
            return None;
        }
        let workload = self.find_by_id(parts[0])?;
        let feature = workload.find_feature(parts[1])?;
        Some((workload, feature))
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
    let mut registry: WorkloadRegistry = serde_json::from_str(&data)?;
    registry.normalize();
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
        presets: default_presets(),
        tutorials: default_tutorials(),
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
            features: vec![
                Feature {
                    id: "lspconfig".into(),
                    name: "LSP Core".into(),
                    description: "Language server configs + Mason installer".into(),
                    plugins: vec![
                        r#"{ "neovim/nvim-lspconfig" }"#.into(),
                        r#"{ "williamboman/mason.nvim", config = true }"#.into(),
                        r#"{ "williamboman/mason-lspconfig.nvim", config = true }"#.into(),
                    ],
                    config_lua: None,
                    default_enabled: true,
                },
            ],
            plugins: vec![],
            config_lua: None,
            cli_aliases: vec!["lsp".into()],
            tutorial: Some(r#"LSP (Language Server Protocol)
==============================

LSP gives you IDE-like features: go-to-definition, hover docs, rename,
diagnostics, and auto-completion — all powered by language servers.

How it works in pnm
--------------------
Enabling the Lsp workload installs three plugins:

  - nvim-lspconfig   Configs for dozens of language servers
  - mason.nvim       Download and manage language servers/tools
  - mason-lspconfig  Bridges mason with lspconfig for auto-setup

First launch
------------
1. Open Neovim:  pnm launch <instance>
2. Run :Mason to open the Mason UI
3. Search for a language server (e.g. lua_ls, pyright, ts_ls)
4. Press i to install it
5. Restart Neovim — LSP attaches automatically to matching files

Key commands
------------
  gd          Go to definition
  gr          Go to references
  K           Hover documentation
  <leader>rn  Rename symbol (if mapped)
  :LspInfo    Show attached language servers

Adding a new language server
----------------------------
1. Run :Mason and install the server
2. It will auto-attach on next file open
3. For custom configuration, add to your user.lua:

   require("lspconfig").lua_ls.setup({
     settings = { Lua = { diagnostics = { globals = { "vim" } } } }
   })

Troubleshooting
---------------
- :LspLog shows the LSP log file
- :LspInfo shows which servers are attached
- If a server doesn't attach, check :Mason to confirm it's installed"#.into()),
        },
        Workload {
            id: "Dap".into(),
            name: "DAP".into(),
            description: "Debug Adapter Protocol (nvim-dap + mason)".into(),
            base: false,
            depends_on: vec![],
            features: vec![
                Feature {
                    id: "dap-core".into(),
                    name: "DAP Core".into(),
                    description: "Debug adapter client + UI + Mason integration".into(),
                    plugins: vec![
                        r#"{ "mfussenegger/nvim-dap" }"#.into(),
                        r#"{ "rcarriga/nvim-dap-ui", dependencies = { "mfussenegger/nvim-dap", "nvim-neotest/nvim-nio" }, config = true }"#.into(),
                        r#"{ "jay-babu/mason-nvim-dap.nvim", config = true }"#.into(),
                    ],
                    config_lua: None,
                    default_enabled: true,
                },
            ],
            plugins: vec![],
            config_lua: None,
            cli_aliases: vec!["dap".into()],
            tutorial: Some(r#"DAP (Debug Adapter Protocol)
============================

DAP lets you debug programs directly inside Neovim — set breakpoints,
step through code, inspect variables, and evaluate expressions.

How it works in pnm
--------------------
The Dap workload installs:

  - nvim-dap           Core debug adapter client
  - nvim-dap-ui        Split-pane UI for debugging (variables, stacks, etc.)
  - mason-nvim-dap     Install debug adapters through Mason

Setup
-----
1. Enable the Dap feature:  pnm features <instance> --enable dap
2. Launch your instance and run :Mason
3. Install a debug adapter for your language:
   - Python:  debugpy
   - Node:    js-debug-adapter
   - C#:      netcoredbg
   - C/C++:   codelldb

Configuring a debug adapter
----------------------------
Add a DAP configuration in your user.lua. Example for Python:

  local dap = require("dap")
  dap.adapters.python = {
    type = "executable",
    command = "python",
    args = { "-m", "debugpy.adapter" },
  }
  dap.configurations.python = {
    {
      type = "python",
      request = "launch",
      name = "Launch file",
      program = "${file}",
    },
  }

Key commands
------------
  :DapToggleBreakpoint   Toggle breakpoint on current line
  :DapContinue           Start/continue debugging
  :DapStepOver           Step over
  :DapStepInto           Step into
  :DapStepOut            Step out
  :DapTerminate          Stop the debugger

  :DapToggleRepl         Open the debug REPL
  :lua require("dapui").toggle()   Toggle the debug UI

Workflow
--------
1. Open your source file
2. Set breakpoints with :DapToggleBreakpoint
3. Start debugging with :DapContinue
4. The DAP UI opens automatically showing variables, call stack, etc.
5. Step through code, inspect values, evaluate expressions in the REPL"#.into()),
        },
        Workload {
            id: "TreeView".into(),
            name: "TreeView".into(),
            description: "Left explorer panel (neo-tree)".into(),
            base: false,
            depends_on: vec![],
            features: vec![
                Feature {
                    id: "neo-tree".into(),
                    name: "Neo-tree".into(),
                    description: "VS Code-style file explorer sidebar".into(),
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
                    default_enabled: true,
                },
            ],
            plugins: vec![],
            config_lua: None,
            cli_aliases: vec!["treeview".into(), "tree-view".into(), "tree".into()],
            tutorial: Some(r#"TreeView (Neo-tree File Explorer)
=================================

Neo-tree provides a VS Code-style file explorer panel on the left side
of your editor, with file browsing, buffer listing, and git status views.

Keybindings
-----------
  <leader>e    Toggle the file explorer
  <leader>be   Toggle the buffer explorer
  <leader>ge   Toggle the git status explorer

(The leader key is Space by default — so press Space then e.)

Navigating the tree
-------------------
  j / k       Move up/down
  Enter       Open file or expand/collapse folder
  o           Open file
  s           Open in vertical split
  S           Open in horizontal split
  a           Create a new file (type the name, end with / for a folder)
  d           Delete file/folder
  r           Rename
  c           Copy
  m           Move
  q           Close the tree

Source tabs
-----------
Neo-tree has three views accessible from the top bar:
  [Files]     Browse the filesystem
  [Buffers]   See open buffers
  [Git]       See changed files (staged/unstaged)

Tips
----
- The tree follows your current file automatically
- Git-ignored files are hidden by default
- Install a Nerd Font (press n in pnm TUI) for file type icons"#.into()),
        },
        Workload {
            id: "Tabs".into(),
            name: "Tabs".into(),
            description: "Top tabbed editor bar (bufferline)".into(),
            base: false,
            depends_on: vec![],
            features: vec![
                Feature {
                    id: "bufferline".into(),
                    name: "Bufferline".into(),
                    description: "IDE-style tab bar for open buffers".into(),
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
                    default_enabled: true,
                },
            ],
            plugins: vec![],
            config_lua: None,
            cli_aliases: vec!["tabs".into(), "tabline".into(), "bufferline".into()],
            tutorial: Some(r#"Tabs (Bufferline)
=================

Bufferline adds a tabbed bar at the top of your editor, showing all
open buffers as clickable tabs — similar to browser or IDE tabs.

Keybindings
-----------
  <leader>bn   Next buffer (tab)
  <leader>bp   Previous buffer (tab)
  <leader>bd   Close current buffer
  <leader>bo   Close all other buffers

Understanding buffers vs tabs
-----------------------------
In Neovim, a "buffer" is a loaded file. The bufferline shows each
buffer as a tab. This is different from Vim's native tab pages.

When you open a file (e.g. :e myfile.txt), it becomes a buffer and
appears in the tab bar. Closing a buffer (:bd) removes it from the bar.

LSP diagnostics
---------------
If you have the Lsp workload enabled, the tab bar shows diagnostic
indicators (errors, warnings) next to file names.

Integration with TreeView
-------------------------
When TreeView is also enabled, the tab bar offsets itself to align
with the explorer panel, showing "Explorer" above the tree."#.into()),
        },
        Workload {
            id: "Telescope".into(),
            name: "Telescope".into(),
            description: "Fuzzy finder (always on)".into(),
            base: true,
            depends_on: vec![],
            features: vec![
                Feature {
                    id: "telescope".into(),
                    name: "Telescope".into(),
                    description: "Fuzzy finder for files, grep, buffers, and more".into(),
                    plugins: vec![
                        r#"{ "nvim-telescope/telescope.nvim", tag = "0.1.8", dependencies = { "nvim-lua/plenary.nvim" } }"#.into(),
                    ],
                    config_lua: Some(r#"-- Feature: Telescope
require("telescope").setup()
vim.keymap.set("n", "<leader>ff", "<cmd>Telescope find_files<cr>", { desc = "Find files" })
vim.keymap.set("n", "<leader>fg", "<cmd>Telescope live_grep<cr>", { desc = "Live grep" })
vim.keymap.set("n", "<leader>fb", "<cmd>Telescope buffers<cr>", { desc = "Find buffers" })
vim.keymap.set("n", "<leader>fh", "<cmd>Telescope help_tags<cr>", { desc = "Help tags" })"#.into()),
                    default_enabled: true,
                },
            ],
            plugins: vec![],
            config_lua: None,
            cli_aliases: vec!["telescope".into()],
            tutorial: Some(r#"Telescope (Fuzzy Finder)
========================

Telescope is a powerful fuzzy finder — think Ctrl+P in VS Code but
more versatile. It's always enabled in every pnm instance.

Keybindings
-----------
  <leader>ff   Find files by name
  <leader>fg   Live grep (search file contents)
  <leader>fb   Find open buffers
  <leader>fh   Search help tags

Inside the Telescope window
----------------------------
  Ctrl+j / Ctrl+k   Move up/down in results
  Enter              Open selected file
  Ctrl+x             Open in horizontal split
  Ctrl+v             Open in vertical split
  Esc                Close Telescope

Tips
----
- Live grep (<leader>fg) is the fastest way to find anything in a
  project.  Type a few characters and results filter in real time.
- File finder (<leader>ff) respects .gitignore by default.
- For best performance, install ripgrep (rg) and fd on your system."#.into()),
        },
        Workload {
            id: "Treesitter".into(),
            name: "Treesitter".into(),
            description: "Syntax highlighting (always on)".into(),
            base: true,
            depends_on: vec![],
            features: vec![
                Feature {
                    id: "treesitter".into(),
                    name: "Treesitter".into(),
                    description: "Tree-based syntax highlighting and code parsing".into(),
                    plugins: vec![
                        r#"{ "nvim-treesitter/nvim-treesitter", build = ":TSUpdate", config = function() require("nvim-treesitter.install").prefer_git = false end }"#.into(),
                    ],
                    config_lua: None,
                    default_enabled: true,
                },
            ],
            plugins: vec![],
            config_lua: None,
            cli_aliases: vec!["treesitter".into()],
            tutorial: Some(r#"Treesitter (Syntax Highlighting)
=================================

Treesitter provides accurate, tree-based syntax highlighting.  Unlike
regex-based highlighting, it understands the actual structure of your
code.  It's always enabled in every pnm instance.

How it works
------------
On first opening a file type, Treesitter downloads a parser for that
language.  Parsers are small and cached in the instance's data
directory.

Commands
--------
  :TSInstall <lang>     Manually install a parser (e.g. :TSInstall rust)
  :TSUpdate             Update all installed parsers
  :TSInstallInfo        List available parsers and their status

Treesitter also powers
----------------------
- Smarter code folding (zc / zo)
- Better indentation
- Incremental selection (if configured)
- Syntax-aware text objects (e.g. select a function)

Note
----
Parsers are compiled natively, so a C compiler is required on your
system (gcc, clang, or MSVC on Windows).  Most systems have one
already."#.into()),
        },
        Workload {
            id: "OmniSharp".into(),
            name: "OmniSharp".into(),
            description: "C# language server (OmniSharp)".into(),
            base: false,
            depends_on: vec!["Lsp".into()],
            features: vec![
                Feature {
                    id: "omnisharp".into(),
                    name: "OmniSharp".into(),
                    description: "Extended LSP support for C# via OmniSharp".into(),
                    plugins: vec![
                        r#"{ "Hoffs/omnisharp-extended-lsp.nvim" }"#.into(),
                    ],
                    config_lua: None,
                    default_enabled: true,
                },
            ],
            plugins: vec![],
            config_lua: None,
            cli_aliases: vec!["omnisharp".into(), "csharp".into(), "cs".into()],
            tutorial: Some(r#"OmniSharp (C# / .NET Development)
==================================

OmniSharp provides C# language support: IntelliSense, go-to-definition,
refactoring, and diagnostics for .NET projects.

Prerequisites
-------------
  - .NET SDK installed (dotnet command available)
  - The Lsp workload (auto-enabled as a dependency)

Setup
-----
1. Enable the feature:
   pnm features <instance> --enable omnisharp

2. Launch Neovim and run :Mason
3. Install "omnisharp" from the Mason list
4. Open a .cs file — OmniSharp starts automatically

Running a .NET project
----------------------
Use the built-in terminal in Neovim:

  :terminal dotnet run
  :terminal dotnet test

Or use a split terminal:
  :split | terminal dotnet run

Debugging C# projects
----------------------
1. Enable the Dap workload:
   pnm features <instance> --enable dap

2. Install netcoredbg via :Mason
3. Add to your user.lua:

   local dap = require("dap")
   dap.adapters.coreclr = {
     type = "executable",
     command = "netcoredbg",
     args = { "--interpreter=vscode" },
   }
   dap.configurations.cs = {
     {
       type = "coreclr",
       name = "Launch",
       request = "launch",
       program = function()
         return vim.fn.input("Path to dll: ", vim.fn.getcwd() .. "/bin/Debug/", "file")
       end,
     },
   }

4. Build your project: dotnet build
5. Set breakpoints: :DapToggleBreakpoint
6. Start debugging: :DapContinue

Key LSP commands for C#
-----------------------
  gd          Go to definition (decompiles into metadata if needed)
  gr          Find references
  K           Hover documentation
  :LspInfo    Verify OmniSharp is attached"#.into()),
        },
        Workload {
            id: "Node".into(),
            name: "Node".into(),
            description: "TypeScript/JavaScript language server (ts_ls)".into(),
            base: false,
            depends_on: vec!["Lsp".into()],
            features: vec![
                Feature {
                    id: "typescript-tools".into(),
                    name: "TypeScript Tools".into(),
                    description: "Fast TypeScript/JavaScript language support".into(),
                    plugins: vec![
                        r#"{ "pmizio/typescript-tools.nvim", dependencies = { "nvim-lua/plenary.nvim", "neovim/nvim-lspconfig" }, config = true }"#.into(),
                    ],
                    config_lua: None,
                    default_enabled: true,
                },
            ],
            plugins: vec![],
            config_lua: None,
            cli_aliases: vec!["node".into(), "typescript".into(), "ts".into(), "javascript".into(), "js".into()],
            tutorial: Some(r#"Node (TypeScript / JavaScript Development)
==========================================

The Node workload sets up TypeScript and JavaScript support using
typescript-tools.nvim for fast, native TS/JS language features.

Prerequisites
-------------
  - Node.js installed (node and npm available)
  - The Lsp workload (auto-enabled as a dependency)

Setup
-----
1. Enable the feature:
   pnm features <instance> --enable node

2. Launch Neovim and open a .ts, .tsx, .js, or .jsx file
3. typescript-tools starts automatically (no Mason install needed)

Running a Node project
----------------------
Use the built-in terminal:

  :terminal npm run dev
  :terminal npx ts-node myfile.ts
  :terminal node myfile.js

Or use a split terminal:
  :split | terminal npm test

Debugging Node/TypeScript projects
-----------------------------------
1. Enable the Dap workload:
   pnm features <instance> --enable dap

2. Install js-debug-adapter via :Mason
3. Add to your user.lua:

   local dap = require("dap")
   dap.adapters["pwa-node"] = {
     type = "server",
     host = "localhost",
     port = "${port}",
     executable = {
       command = "js-debug-adapter",
       args = { "${port}" },
     },
   }
   dap.configurations.typescript = {
     {
       type = "pwa-node",
       request = "launch",
       name = "Launch file",
       program = "${file}",
       cwd = "${workspaceFolder}",
       runtimeExecutable = "npx",
       runtimeArgs = { "ts-node" },
     },
   }
   dap.configurations.javascript = {
     {
       type = "pwa-node",
       request = "launch",
       name = "Launch file",
       program = "${file}",
       cwd = "${workspaceFolder}",
     },
   }

4. Set breakpoints: :DapToggleBreakpoint
5. Start debugging: :DapContinue

Key LSP commands
----------------
  gd          Go to definition
  gr          Find references
  K           Hover documentation
  :TSToolsOrganizeImports    Organize imports
  :TSToolsRenameFile         Rename file and update imports"#.into()),
        },
        Workload {
            id: "Python".into(),
            name: "Python".into(),
            description: "Python language server (pyright)".into(),
            base: false,
            depends_on: vec!["Lsp".into()],
            features: vec![
                Feature {
                    id: "pyright".into(),
                    name: "Pyright".into(),
                    description: "Python language server with type checking".into(),
                    plugins: vec![
                        r#"{ "neovim/nvim-lspconfig" }"#.into(),
                    ],
                    config_lua: None,
                    default_enabled: true,
                },
            ],
            plugins: vec![],
            config_lua: None,
            cli_aliases: vec!["python".into(), "py".into()],
            tutorial: Some(r#"Python Development
==================

The Python workload sets up Pyright, a fast and feature-rich Python
language server with type checking and IntelliSense.

Prerequisites
-------------
  - Python 3 installed
  - The Lsp workload (auto-enabled as a dependency)

Setup
-----
1. Enable the feature:
   pnm features <instance> --enable python

2. Launch Neovim and run :Mason
3. Install "pyright" from the Mason list
4. Open a .py file — Pyright starts automatically

Virtual environments
--------------------
Pyright respects virtual environments.  Activate your venv before
launching Neovim:

  # Linux/macOS
  source .venv/bin/activate && pnm launch <instance>

  # Windows
  .venv\Scripts\activate && pnm launch <instance>

Or set the Python path in a pyrightconfig.json at your project root:

  {
    "venvPath": ".",
    "venv": ".venv"
  }

Running Python projects
-----------------------
Use the built-in terminal:

  :terminal python main.py
  :terminal python -m pytest

Or use a split terminal:
  :split | terminal python -m flask run

Debugging Python projects
--------------------------
1. Enable the Dap workload:
   pnm features <instance> --enable dap

2. Install debugpy via :Mason (or pip install debugpy)
3. Add to your user.lua:

   local dap = require("dap")
   dap.adapters.python = {
     type = "executable",
     command = "python",
     args = { "-m", "debugpy.adapter" },
   }
   dap.configurations.python = {
     {
       type = "python",
       request = "launch",
       name = "Launch file",
       program = "${file}",
       pythonPath = function()
         local venv = os.getenv("VIRTUAL_ENV")
         if venv then
           return venv .. "/bin/python"
         end
         return "python"
       end,
     },
   }

4. Set breakpoints: :DapToggleBreakpoint
5. Start debugging: :DapContinue

Key LSP commands
----------------
  gd          Go to definition
  gr          Find references
  K           Hover type information
  :LspInfo    Verify pyright is attached"#.into()),
        },
    ]
}

fn default_presets() -> Vec<Preset> {
    vec![
        Preset {
            id: "minimal".into(),
            name: "Minimal".into(),
            description: "Just syntax highlighting and fuzzy finding (base workloads only)".into(),
            workloads: vec![],
        },
        Preset {
            id: "ide-core".into(),
            name: "IDE Core".into(),
            description: "LSP, tree view, tabs — essential IDE features".into(),
            workloads: vec!["Lsp".into(), "TreeView".into(), "Tabs".into()],
        },
        Preset {
            id: "ide-full".into(),
            name: "IDE Full".into(),
            description: "Full IDE: LSP, DAP, tree view, tabs".into(),
            workloads: vec!["Lsp".into(), "Dap".into(), "TreeView".into(), "Tabs".into()],
        },
    ]
}

fn default_tutorials() -> Vec<Tutorial> {
    vec![
        Tutorial {
            id: "leader-key".into(),
            title: "Understanding the Leader Key".into(),
            content: r#"Understanding the Leader Key
============================

The "leader key" is a prefix key in Neovim that starts multi-key
shortcuts.  Think of it like a modifier key (Ctrl, Alt) but for
custom keybindings.

How it works
------------
When you press the leader key, Neovim waits briefly for a follow-up
key.  For example, with leader set to Space:

  Space → f → f    Opens Telescope file finder
  Space → e        Toggles the file explorer
  Space → b → n    Next buffer tab

The leader key itself doesn't do anything — it just starts a sequence.

Available leader keys in pnm
-----------------------------
  Space       (default) The most popular choice — easy to reach
  Comma       Classic Vim choice, stays on home row
  Backslash   Vim's original default leader
  Semicolon   Alternative home-row option

Changing the leader key
-----------------------
CLI:
  Not directly via CLI — use the TUI or edit the manifest.

TUI:
  1. Open the TUI: pnm tui
  2. Select your instance and press Enter
  3. Press m to change the leader key
  4. Pick your preferred key and press Enter

The leader key is set BEFORE plugins load, so all keybindings
automatically use your chosen key.

Common keybinding patterns
--------------------------
The leader key organizes shortcuts into logical groups:

  <leader>f   Find/search (Telescope)
    ff        Find files
    fg        Live grep
    fb        Find buffers
    fh        Help tags

  <leader>e   Explorer (TreeView)

  <leader>b   Buffer management (Tabs)
    bn        Next buffer
    bp        Previous buffer
    bd        Delete buffer
    bo        Close others

  <leader>g   Git (TreeView git status)
    ge        Git explorer

Tip: press the leader key and wait — if you have which-key installed,
it will show all available continuations."#.into(),
        },
        Tutorial {
            id: "getting-started".into(),
            title: "Getting Started with pnm".into(),
            content: r#"Getting Started with pnm
========================

This guide walks you through creating your first Neovim instance
and understanding the basics.

Step 1: Create an instance
--------------------------
  pnm create myenv --features lsp,treeview,tabs

This creates a fully isolated Neovim installation with:
  - LSP support (language servers via Mason)
  - A file explorer panel
  - A tabbed buffer bar

Step 2: First launch
--------------------
  pnm launch myenv

On first launch, Lazy.nvim (the plugin manager) bootstraps itself
and downloads all configured plugins.  This takes 30-60 seconds
depending on your connection.  Subsequent launches are instant.

Step 3: Install a language server
---------------------------------
Once inside Neovim:
  1. Run :Mason
  2. Find your language (e.g. pyright for Python, ts_ls for TypeScript)
  3. Press i to install

Step 4: Explore the keybindings
-------------------------------
Your instance comes with these keybindings (Space is the leader):

  Space ff    Find files
  Space fg    Search in files (grep)
  Space e     Toggle file explorer
  Space bn    Next tab
  Space bp    Previous tab
  Space bd    Close current tab

Step 5: Customize
-----------------
Each instance has a user.lua file for personal customizations:

  <instance>/config/nvim/user.lua

Add your own keybindings, plugin configs, or settings there.
This file is never overwritten by pnm.

Instance isolation
------------------
Each instance is completely self-contained:
  - Its own Neovim binary
  - Its own plugins and plugin data
  - Its own config, cache, and state
  - Nothing is shared between instances

This means you can have a minimal instance for quick edits and a
fully-loaded one for development, without conflicts.

Managing instances
------------------
  pnm list                  See all instances
  pnm info <name>           Detailed info
  pnm update <name>         Update Neovim version
  pnm features <name> ...   Toggle features
  pnm delete <name>         Remove an instance
  pnm tui                   Interactive management UI"#.into(),
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
            assert_eq!(a.features.len(), b.features.len());
        }
        assert_eq!(loaded.tutorials.len(), reg.tutorials.len());
    }

    #[test]
    fn test_backward_compat_no_tutorial_fields() {
        let json = r#"{
            "workloads": [{
                "id": "Lsp",
                "name": "LSP",
                "description": "test",
                "base": false,
                "depends_on": [],
                "plugins": [],
                "config_lua": null,
                "cli_aliases": ["lsp"]
            }]
        }"#;
        let reg: WorkloadRegistry = serde_json::from_str(json).unwrap();
        assert_eq!(reg.workloads.len(), 1);
        assert!(reg.workloads[0].tutorial.is_none());
        assert!(reg.tutorials.is_empty());
    }

    #[test]
    fn test_all_workloads_have_tutorials() {
        let reg = default_registry();
        for w in reg.all() {
            assert!(
                w.tutorial.is_some(),
                "Workload '{}' is missing a tutorial",
                w.id
            );
        }
    }

    #[test]
    fn test_default_general_tutorials() {
        let reg = default_registry();
        assert!(reg.tutorials.len() >= 2);
        assert!(reg.find_tutorial_by_id("leader-key").is_some());
        assert!(reg.find_tutorial_by_id("getting-started").is_some());
    }

    #[test]
    fn test_all_tutorial_topics() {
        let reg = default_registry();
        let topics = reg.all_tutorial_topics();
        // 2 general + 9 workloads = 11
        assert_eq!(topics.len(), 11);
    }

    #[test]
    fn test_tutorial_content_by_id() {
        let reg = default_registry();
        let (title, content) = reg.tutorial_content("leader-key").unwrap();
        assert_eq!(title, "Understanding the Leader Key");
        assert!(!content.is_empty());
    }

    #[test]
    fn test_tutorial_content_by_workload_id() {
        let reg = default_registry();
        let (title, content) = reg.tutorial_content("Python").unwrap();
        assert_eq!(title, "Python");
        assert!(content.contains("pyright"));
    }

    #[test]
    fn test_tutorial_content_by_alias() {
        let reg = default_registry();
        let (title, _) = reg.tutorial_content("py").unwrap();
        assert_eq!(title, "Python");
        let (title, _) = reg.tutorial_content("cs").unwrap();
        assert_eq!(title, "OmniSharp");
        let (title, _) = reg.tutorial_content("ts").unwrap();
        assert_eq!(title, "Node");
    }

    #[test]
    fn test_tutorial_content_unknown_returns_none() {
        let reg = default_registry();
        assert!(reg.tutorial_content("nonexistent").is_none());
    }

    #[test]
    fn test_all_workloads_have_features() {
        let reg = default_registry();
        for w in reg.all() {
            assert!(
                !w.features.is_empty(),
                "Workload '{}' has no features",
                w.id
            );
        }
    }

    #[test]
    fn test_workload_all_plugins_via_features() {
        let reg = default_registry();
        let lsp = reg.find_by_id("Lsp").unwrap();
        let plugins = lsp.all_plugins();
        assert!(!plugins.is_empty());
        assert!(plugins.iter().any(|p| p.contains("nvim-lspconfig")));
    }

    #[test]
    fn test_find_feature_by_path() {
        let reg = default_registry();
        let (w, f) = reg.find_feature_by_path("Lsp/lspconfig").unwrap();
        assert_eq!(w.id, "Lsp");
        assert_eq!(f.id, "lspconfig");
        assert!(reg.find_feature_by_path("Nonexistent/foo").is_none());
    }

    #[test]
    fn test_normalize_migrates_old_format() {
        let json = r#"{
            "workloads": [{
                "id": "Old",
                "name": "Old",
                "description": "test",
                "base": false,
                "depends_on": [],
                "plugins": ["{ \"some/plugin\" }"],
                "config_lua": "vim.g.test = true",
                "cli_aliases": ["old"]
            }]
        }"#;
        let mut reg: WorkloadRegistry = serde_json::from_str(json).unwrap();
        reg.normalize();
        let w = &reg.workloads[0];
        assert_eq!(w.features.len(), 1);
        assert_eq!(w.features[0].id, "old");
        assert_eq!(w.features[0].plugins.len(), 1);
        assert!(w.features[0].config_lua.is_some());
        // Old fields should be emptied after migration
        assert!(w.plugins.is_empty());
        assert!(w.config_lua.is_none());
    }

    #[test]
    fn test_presets_exist() {
        let reg = default_registry();
        assert!(reg.find_preset("minimal").is_some());
        assert!(reg.find_preset("ide-core").is_some());
        assert!(reg.find_preset("ide-full").is_some());
    }
}
