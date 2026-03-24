use super::model::{Feature, Preset, Tutorial, Workload, WorkloadRegistry};

// ---------------------------------------------------------------------------
// Built-in defaults
// ---------------------------------------------------------------------------

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
            category: Some("Languages".into()),
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
            category: Some("Tasks & Launchers".into()),
        },
        Workload {
            id: "Overseer".into(),
            name: "Overseer".into(),
            description: "Task runner & VS Code tasks.json support (overseer.nvim)".into(),
            base: false,
            depends_on: vec![],
            features: vec![
                Feature {
                    id: "overseer".into(),
                    name: "Overseer".into(),
                    description: "Task runner with .vscode/tasks.json support and DAP integration".into(),
                    plugins: vec![
                        r#"{ "stevearc/overseer.nvim" }"#.into(),
                    ],
                    config_lua: Some(r#"-- Feature: Overseer (task runner)
require("overseer").setup()
vim.keymap.set("n", "<leader>or", "<cmd>OverseerRun<cr>", { desc = "Run task" })
vim.keymap.set("n", "<leader>ot", "<cmd>OverseerToggle<cr>", { desc = "Toggle task list" })
vim.keymap.set("n", "<leader>oa", "<cmd>OverseerTaskAction<cr>", { desc = "Task action" })
vim.keymap.set("n", "<leader>oi", "<cmd>OverseerInfo<cr>", { desc = "Overseer info" })"#.into()),
                    default_enabled: true,
                },
            ],
            plugins: vec![],
            config_lua: None,
            cli_aliases: vec!["overseer".into(), "tasks".into()],
            tutorial: Some(r#"Overseer (Task Runner)
======================

Overseer is a task runner and job management plugin that can read
VS Code's .vscode/tasks.json files natively.

Key commands
------------
  <leader>or    Run a task (shows picker with all available tasks)
  <leader>ot    Toggle the task list panel
  <leader>oa    Run an action on a task
  <leader>oi    Show overseer info (registered templates, components)

  :OverseerRun          Pick and run a task
  :OverseerToggle       Open/close task list

VS Code tasks.json
------------------
If your project has a .vscode/tasks.json file, Overseer picks it up
automatically.  Supported features include:

  - shell and process task types
  - Variable substitution (${workspaceFolder}, ${file}, etc.)
  - Input variables (promptString, pickString)
  - Problem matchers ($tsc, $eslint-stylish, and custom)
  - Compound tasks (dependsOn, dependsOrder)
  - Background/watch tasks
  - OS-specific properties (windows, linux, osx)

DAP integration
---------------
When both Overseer and DAP workloads are enabled, Overseer
automatically handles preLaunchTask and postDebugTask from
your .vscode/launch.json — no extra configuration needed.

Built-in task sources
---------------------
Overseer auto-detects tasks from many sources:
  - .vscode/tasks.json
  - Makefile
  - package.json (npm scripts)
  - Cargo.toml (cargo commands)
  - And more via template providers

Custom tasks
------------
Define project-local tasks in .overseer/ or global templates in
~/.config/nvim/lua/overseer/template/.

Tips
----
- Overseer integrates with neotest (Testing workload) to run tests
- Task output is parsed into Neovim diagnostics via problem matchers
- Use :OverseerRun to see all available tasks from every source"#.into()),
            category: Some("Tasks & Launchers".into()),
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
            category: None,
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
            category: None,
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
            category: None,
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
            category: None,
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
            category: Some("Languages".into()),
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
            category: Some("Languages".into()),
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
            category: Some("Languages".into()),
        },
        // -----------------------------------------------------------------
        // New IDE workloads
        // -----------------------------------------------------------------
        Workload {
            id: "Git".into(),
            name: "Git".into(),
            description: "Git integration (gitsigns)".into(),
            base: false,
            depends_on: vec![],
            features: vec![
                Feature {
                    id: "gitsigns".into(),
                    name: "Gitsigns".into(),
                    description: "Git signs in the gutter, hunk actions, blame".into(),
                    plugins: vec![
                        r#"{ "lewis6991/gitsigns.nvim" }"#.into(),
                    ],
                    config_lua: Some(r#"-- Feature: Git (gitsigns)
require("gitsigns").setup({
  on_attach = function(bufnr)
    local gs = require("gitsigns")
    local function map(mode, l, r, opts)
      opts = opts or {}
      opts.buffer = bufnr
      vim.keymap.set(mode, l, r, opts)
    end
    -- Hunk navigation
    map("n", "]c", function()
      if vim.wo.diff then vim.cmd.normal({ "]c", bang = true }) else gs.nav_hunk("next") end
    end, { desc = "Next hunk" })
    map("n", "[c", function()
      if vim.wo.diff then vim.cmd.normal({ "[c", bang = true }) else gs.nav_hunk("prev") end
    end, { desc = "Prev hunk" })
    -- Actions
    map("n", "<leader>hs", gs.stage_hunk, { desc = "Stage hunk" })
    map("n", "<leader>hr", gs.reset_hunk, { desc = "Reset hunk" })
    map("v", "<leader>hs", function() gs.stage_hunk({ vim.fn.line("."), vim.fn.line("v") }) end, { desc = "Stage hunk" })
    map("v", "<leader>hr", function() gs.reset_hunk({ vim.fn.line("."), vim.fn.line("v") }) end, { desc = "Reset hunk" })
    map("n", "<leader>hS", gs.stage_buffer, { desc = "Stage buffer" })
    map("n", "<leader>hR", gs.reset_buffer, { desc = "Reset buffer" })
    map("n", "<leader>hp", gs.preview_hunk, { desc = "Preview hunk" })
    map("n", "<leader>hb", function() gs.blame_line({ full = true }) end, { desc = "Blame line" })
    map("n", "<leader>hd", gs.diffthis, { desc = "Diff this" })
    map("n", "<leader>tb", gs.toggle_current_line_blame, { desc = "Toggle line blame" })
  end,
})"#.into()),
                    default_enabled: true,
                },
            ],
            plugins: vec![],
            config_lua: None,
            cli_aliases: vec!["git".into(), "gitsigns".into()],
            tutorial: Some(r#"Git (Gitsigns)
==============

Gitsigns adds Git integration directly into your editor: signs in the
gutter showing added/changed/deleted lines, hunk actions, and blame.

Key commands
------------
  ]c / [c              Navigate between hunks
  <leader>hs           Stage hunk (normal or visual)
  <leader>hr           Reset hunk (normal or visual)
  <leader>hS           Stage entire buffer
  <leader>hR           Reset entire buffer
  <leader>hp           Preview hunk in popup
  <leader>hb           Blame current line (full commit info)
  <leader>hd           Diff this file
  <leader>tb           Toggle inline line blame

Workflow
--------
1. Edit files in a git repository
2. Signs appear automatically in the gutter:
   ┃  Added line
   ┃  Changed line
   _  Deleted line
3. Navigate hunks with ]c / [c
4. Stage hunks with <leader>hs or stage all with <leader>hS
5. Preview changes with <leader>hp before deciding

Tips
----
- Use :Gitsigns diffthis to see a full diff split
- Use :Gitsigns blame to see the full buffer blame view
- Gitsigns integrates with lualine to show git branch and diff stats"#.into()),
            category: None,
        },
        Workload {
            id: "Completion".into(),
            name: "Completion".into(),
            description: "Auto-completion (blink.cmp)".into(),
            base: false,
            depends_on: vec!["Lsp".into()],
            features: vec![
                Feature {
                    id: "blink-cmp".into(),
                    name: "Blink CMP".into(),
                    description: "Fast, batteries-included completion with fuzzy matching".into(),
                    plugins: vec![
                        r#"{ "saghen/blink.cmp", version = "1.*", dependencies = { "rafamadriz/friendly-snippets" } }"#.into(),
                    ],
                    config_lua: Some(r#"-- Feature: Completion (blink.cmp)
require("blink.cmp").setup({
  keymap = { preset = "default" },
  appearance = { nerd_font_variant = "mono" },
  sources = { default = { "lsp", "path", "snippets", "buffer" } },
  signature = { enabled = true },
})"#.into()),
                    default_enabled: true,
                },
            ],
            plugins: vec![],
            config_lua: None,
            cli_aliases: vec!["completion".into(), "cmp".into(), "autocomplete".into()],
            tutorial: Some(r#"Completion (blink.cmp)
======================

Blink.cmp provides IDE-style auto-completion: LSP suggestions, snippets,
file paths, and buffer words — all with typo-resistant fuzzy matching.

How it works
------------
Completion appears automatically as you type. Sources include:
  - LSP     Language server completions (functions, variables, types)
  - Snippets  Code templates from friendly-snippets
  - Path    File system paths
  - Buffer  Words from open buffers

Key bindings (default keymap)
-----------------------------
  <C-space>       Trigger completion manually
  <C-e>           Cancel/dismiss completion
  <C-y> / <CR>    Accept selected item
  <C-n> / <Down>  Next item
  <C-p> / <Up>    Previous item
  <Tab>           Snippet jump forward
  <S-Tab>         Snippet jump backward

Signature help appears automatically when typing function arguments.

Prerequisites
-------------
  - The Lsp workload must be enabled (auto-enabled as dependency)
  - A language server must be installed via :Mason

Tips
----
- Completion is async and updates on every keystroke
- Fuzzy matching handles typos (e.g. typing "fnctin" matches "function")
- Use :checkhealth blink.cmp to diagnose issues"#.into()),
            category: None,
        },
        Workload {
            id: "Formatting".into(),
            name: "Formatting".into(),
            description: "Code formatting and linting (conform + nvim-lint)".into(),
            base: false,
            depends_on: vec!["Lsp".into()],
            features: vec![
                Feature {
                    id: "conform".into(),
                    name: "Conform".into(),
                    description: "Lightweight code formatter with format-on-save".into(),
                    plugins: vec![
                        r#"{ "stevearc/conform.nvim" }"#.into(),
                    ],
                    config_lua: Some(r#"-- Feature: Formatting (conform.nvim)
require("conform").setup({
  format_on_save = {
    timeout_ms = 500,
    lsp_format = "fallback",
  },
})
vim.keymap.set({ "n", "v" }, "<leader>cf", function()
  require("conform").format({ async = true, lsp_format = "fallback" })
end, { desc = "Format buffer/selection" })"#.into()),
                    default_enabled: true,
                },
                Feature {
                    id: "nvim-lint".into(),
                    name: "Nvim Lint".into(),
                    description: "Async linter complementing LSP diagnostics".into(),
                    plugins: vec![
                        r#"{ "mfussenegger/nvim-lint" }"#.into(),
                    ],
                    config_lua: Some(r#"-- Feature: Linting (nvim-lint)
vim.api.nvim_create_autocmd({ "BufWritePost", "InsertLeave" }, {
  callback = function()
    require("lint").try_lint()
  end,
})"#.into()),
                    default_enabled: true,
                },
            ],
            plugins: vec![],
            config_lua: None,
            cli_aliases: vec!["formatting".into(), "format".into(), "lint".into(), "linting".into()],
            tutorial: Some(r#"Formatting & Linting
====================

This workload provides two complementary tools:

  - conform.nvim   Format code on save (or manually)
  - nvim-lint       Async linting beyond what LSP provides

Format on Save
--------------
Files are automatically formatted when you save (:w). The formatter
tries your configured formatter first, falling back to LSP formatting.

Manual formatting: <leader>cf (works in normal and visual mode)

Setting up formatters
---------------------
Add formatter configuration to your user.lua:

  require("conform").setup({
    formatters_by_ft = {
      lua = { "stylua" },
      python = { "isort", "black" },
      javascript = { "prettierd", "prettier", stop_after_first = true },
      rust = { "rustfmt", lsp_format = "fallback" },
    },
  })

Install formatters via :Mason or your system package manager.

Setting up linters
------------------
Add linter configuration to your user.lua:

  require("lint").linters_by_ft = {
    python = { "ruff" },
    javascript = { "eslint" },
    markdown = { "vale" },
  }

Install linters via :Mason or your system package manager.

Commands
--------
  :ConformInfo     Show configured formatters and log
  <leader>cf       Format buffer or selection

Tips
----
- Format-on-save has a 500ms timeout to keep saves fast
- Range formatting works: select lines in visual mode, then <leader>cf
- Linting runs on save and when leaving insert mode"#.into()),
            category: None,
        },
        Workload {
            id: "Testing".into(),
            name: "Testing".into(),
            description: "Test runner framework (neotest)".into(),
            base: false,
            depends_on: vec!["Treesitter".into()],
            features: vec![
                Feature {
                    id: "neotest".into(),
                    name: "Neotest".into(),
                    description: "Run and debug tests with 35+ language adapters".into(),
                    plugins: vec![
                        r#"{ "nvim-neotest/neotest", dependencies = { "nvim-neotest/nvim-nio", "nvim-lua/plenary.nvim", "antoinemadec/FixCursorHold.nvim", "nvim-treesitter/nvim-treesitter" } }"#.into(),
                    ],
                    config_lua: Some(r#"-- Feature: Testing (neotest)
require("neotest").setup({
  -- Add adapters in your user.lua, e.g.:
  -- adapters = { require("neotest-python"), require("neotest-jest") }
})
vim.keymap.set("n", "<leader>tt", function() require("neotest").run.run() end, { desc = "Run nearest test" })
vim.keymap.set("n", "<leader>tf", function() require("neotest").run.run(vim.fn.expand("%")) end, { desc = "Run file tests" })
vim.keymap.set("n", "<leader>ts", function() require("neotest").summary.toggle() end, { desc = "Toggle test summary" })
vim.keymap.set("n", "<leader>to", function() require("neotest").output.open({ enter_on_open = true }) end, { desc = "Show test output" })"#.into()),
                    default_enabled: true,
                },
            ],
            plugins: vec![],
            config_lua: None,
            cli_aliases: vec!["testing".into(), "test".into(), "neotest".into()],
            tutorial: Some(r#"Testing (Neotest)
=================

Neotest is a test runner framework — like VSCode's Test Explorer but
inside Neovim. It supports 35+ language adapters.

Key commands
------------
  <leader>tt    Run the nearest test
  <leader>tf    Run all tests in the current file
  <leader>ts    Toggle the test summary panel
  <leader>to    Show output of the last test run

Setting up test adapters
------------------------
Install the adapter for your language, then configure in user.lua:

  -- Python (pytest/unittest)
  require("neotest").setup({
    adapters = {
      require("neotest-python"),
    },
  })

Popular adapters:
  neotest-python     pytest, unittest
  neotest-jest       Jest (JavaScript)
  neotest-vitest     Vitest (JavaScript)
  neotest-go         Go testing
  neotest-rust       Rust (cargo test)
  neotest-dotnet     .NET (xUnit, NUnit, MSTest)

Install adapters as lazy.nvim plugins in your user.lua:
  { "nvim-neotest/neotest-python" }
  { "nvim-neotest/neotest-jest" }

Workflow
--------
1. Open a test file
2. Press <leader>tt to run the nearest test
3. Press <leader>ts to see the summary panel
4. Green ✓ for passing, red ✗ for failing
5. Press <leader>to on a failed test to see the output

Tips
----
- Neotest integrates with DAP for debugging tests
- Use :Neotest run {strategy = "dap"} to debug a test
- The summary panel shows the full test tree"#.into()),
            category: None,
        },
        Workload {
            id: "Editing".into(),
            name: "Editing".into(),
            description: "Editing conveniences (autopairs, surround, flash, comments)".into(),
            base: false,
            depends_on: vec![],
            features: vec![
                Feature {
                    id: "autopairs".into(),
                    name: "Auto Pairs".into(),
                    description: "Automatically close brackets, quotes, etc.".into(),
                    plugins: vec![
                        r#"{ "windwp/nvim-autopairs", event = "InsertEnter", config = true }"#.into(),
                    ],
                    config_lua: None,
                    default_enabled: true,
                },
                Feature {
                    id: "surround".into(),
                    name: "Surround".into(),
                    description: "Add/change/delete surrounding pairs".into(),
                    plugins: vec![
                        r#"{ "kylechui/nvim-surround", version = "^4.0.0", event = "VeryLazy", config = true }"#.into(),
                    ],
                    config_lua: None,
                    default_enabled: true,
                },
                Feature {
                    id: "flash".into(),
                    name: "Flash".into(),
                    description: "Lightning-fast navigation with search labels".into(),
                    plugins: vec![
                        r#"{ "folke/flash.nvim", event = "VeryLazy" }"#.into(),
                    ],
                    config_lua: Some(r#"-- Feature: Editing (flash.nvim)
vim.keymap.set({ "n", "x", "o" }, "s", function() require("flash").jump() end, { desc = "Flash jump" })
vim.keymap.set({ "n", "x", "o" }, "S", function() require("flash").treesitter() end, { desc = "Flash Treesitter" })
vim.keymap.set("o", "r", function() require("flash").remote() end, { desc = "Remote Flash" })"#.into()),
                    default_enabled: true,
                },
                Feature {
                    id: "comment".into(),
                    name: "Comment".into(),
                    description: "Toggle comments with gc/gcc".into(),
                    plugins: vec![
                        r#"{ "numToStr/Comment.nvim", event = "VeryLazy", config = true }"#.into(),
                    ],
                    config_lua: None,
                    default_enabled: true,
                },
            ],
            plugins: vec![],
            config_lua: None,
            cli_aliases: vec!["editing".into(), "edit".into(), "convenience".into()],
            tutorial: Some(r#"Editing Conveniences
====================

This workload bundles four editing quality-of-life plugins:

Auto Pairs
----------
Automatically closes brackets, quotes, and other pairs as you type.
  Type (  →  get ()  with cursor between
  Type {  →  get {}
  Press Enter between {} →  properly indented block

Surround (nvim-surround)
------------------------
Add, change, or delete surrounding characters:
  ys{motion}{char}    Add surround     ysiw) → surround word with ()
  ds{char}            Delete surround  ds]   → remove [ ]
  cs{old}{new}        Change surround  cs'"  → change ' to "

Examples:
  Old text              Command    New text
  surr*ound_words       ysiw)      (surround_words)
  [delete ar*ound me!]  ds]        delete around me!
  'change quot*es'      cs'"       "change quotes"

Flash (flash.nvim)
------------------
Lightning-fast navigation using search labels:
  s        Flash jump — type characters, then a label to jump
  S        Flash Treesitter — select Treesitter nodes
  r        Remote Flash (in operator-pending mode)

Comment (Comment.nvim)
----------------------
Toggle comments easily:
  gcc      Toggle comment on current line
  gc{motion}  Toggle comment on motion (e.g. gcap for paragraph)
  gc       Toggle comment on selection (visual mode)"#.into()),
            category: None,
        },
        Workload {
            id: "Statusline".into(),
            name: "Statusline".into(),
            description: "Status line (lualine)".into(),
            base: false,
            depends_on: vec![],
            features: vec![
                Feature {
                    id: "lualine".into(),
                    name: "Lualine".into(),
                    description: "Blazing fast statusline with mode, branch, diagnostics".into(),
                    plugins: vec![
                        r#"{ "nvim-lualine/lualine.nvim", dependencies = { "nvim-tree/nvim-web-devicons" } }"#.into(),
                    ],
                    config_lua: Some(r#"-- Feature: Statusline (lualine)
require("lualine").setup({
  options = {
    theme = "auto",
    component_separators = { left = "", right = "" },
    section_separators = { left = "", right = "" },
  },
})"#.into()),
                    default_enabled: true,
                },
            ],
            plugins: vec![],
            config_lua: None,
            cli_aliases: vec!["statusline".into(), "lualine".into(), "status".into()],
            tutorial: Some(r#"Statusline (Lualine)
====================

Lualine adds a beautiful, informative status bar at the bottom of your
editor — showing mode, git branch, diagnostics, file type, and position.

Layout
------
  +-------------------------------------------------+
  | A | B | C                             X | Y | Z |
  +-------------------------------------------------+

Default sections:
  A  Current mode (NORMAL, INSERT, VISUAL, etc.)
  B  Git branch + diff stats (if gitsigns is enabled)
  C  File path
  X  File encoding
  Y  File type
  Z  Cursor position (line:column)

Customization
-------------
Add to your user.lua to customize sections:

  require("lualine").setup({
    sections = {
      lualine_a = { "mode" },
      lualine_b = { "branch", "diff", "diagnostics" },
      lualine_c = { "filename" },
      lualine_x = { "encoding", "fileformat", "filetype" },
      lualine_y = { "progress" },
      lualine_z = { "location" },
    },
  })

Tips
----
- Lualine auto-detects your colorscheme
- It shows LSP diagnostics count (errors, warnings)
- Git integration requires the Git workload (gitsigns)
- Install a Nerd Font for proper icon display (pnm install-font)"#.into()),
            category: None,
        },
        Workload {
            id: "AI".into(),
            name: "AI".into(),
            description: "AI-assisted coding (GitHub Copilot Chat)".into(),
            base: false,
            depends_on: vec!["Lsp".into()],
            features: vec![
                Feature {
                    id: "copilot-chat".into(),
                    name: "Copilot Chat".into(),
                    description: "GitHub Copilot Chat with tool calling and multi-model support".into(),
                    plugins: vec![
                        r#"{ "github/copilot.vim" }"#.into(),
                        r#"{ "CopilotC-Nvim/CopilotChat.nvim", dependencies = { "github/copilot.vim", "nvim-lua/plenary.nvim" }, config = true }"#.into(),
                    ],
                    config_lua: Some(r#"-- Feature: AI (CopilotChat)
vim.keymap.set({ "n", "v" }, "<leader>ac", "<cmd>CopilotChatToggle<cr>", { desc = "Toggle Copilot Chat" })
vim.keymap.set({ "n", "v" }, "<leader>ae", "<cmd>CopilotChatExplain<cr>", { desc = "Explain code" })
vim.keymap.set({ "n", "v" }, "<leader>ar", "<cmd>CopilotChatReview<cr>", { desc = "Review code" })
vim.keymap.set({ "n", "v" }, "<leader>af", "<cmd>CopilotChatFix<cr>", { desc = "Fix code" })
vim.keymap.set({ "n", "v" }, "<leader>ao", "<cmd>CopilotChatOptimize<cr>", { desc = "Optimize code" })
vim.keymap.set({ "n", "v" }, "<leader>at", "<cmd>CopilotChatTests<cr>", { desc = "Generate tests" })"#.into()),
                    default_enabled: true,
                },
            ],
            plugins: vec![],
            config_lua: None,
            cli_aliases: vec!["ai".into(), "copilot".into(), "copilot-chat".into()],
            tutorial: Some(r#"AI (GitHub Copilot Chat)
========================

This workload adds GitHub Copilot for inline completions and an
interactive chat interface for code explanation, review, and generation.

Prerequisites
-------------
  - A GitHub Copilot subscription (Individual, Business, or Enterprise)
  - Copilot Chat enabled in your GitHub settings
  - Run :Copilot auth on first launch to authenticate

Inline completions (copilot.vim)
--------------------------------
Copilot suggests code as you type, shown as ghost text:
  Tab            Accept the suggestion
  <M-]>          Next suggestion
  <M-[>          Previous suggestion
  <C-]>          Dismiss suggestion

Chat commands
-------------
  <leader>ac     Toggle Copilot Chat window
  <leader>ae     Explain selected code
  <leader>ar     Review selected code
  <leader>af     Fix issues in selected code
  <leader>ao     Optimize selected code
  <leader>at     Generate tests for selected code

Workflow
--------
1. Select code in visual mode
2. Press <leader>ae to get an explanation
3. Or press <leader>af to ask Copilot to fix issues
4. Chat responses appear in a split window

Supported models
----------------
Copilot Chat supports multiple AI models including GPT-4o, Claude,
and Gemini. The available models depend on your GitHub Copilot settings.

Tips
----
- Use :CopilotChat to start a free-form conversation
- Chat has tool calling: it can read files and search your workspace
- Select specific code before chatting for better context
- Use :CopilotChatCommit to generate commit messages"#.into()),
            category: None,
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
            description: "LSP, completion, git, tree view, tabs, editing, statusline".into(),
            workloads: vec![
                "Lsp".into(), "Completion".into(), "Git".into(),
                "TreeView".into(), "Tabs".into(), "Editing".into(),
                "Statusline".into(),
            ],
        },
        Preset {
            id: "ide-full".into(),
            name: "IDE Full".into(),
            description: "Full IDE: all features including formatting, testing, and AI".into(),
            workloads: vec![
                "Lsp".into(), "Dap".into(), "Overseer".into(), "Completion".into(),
                "Git".into(), "Formatting".into(), "Testing".into(),
                "TreeView".into(), "Tabs".into(), "Editing".into(),
                "Statusline".into(), "AI".into(),
            ],
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