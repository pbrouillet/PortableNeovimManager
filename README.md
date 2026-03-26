# Portable Neovim Manager (pnm)

A single-binary tool for managing multiple self-contained Neovim installations on Windows, macOS, and Linux. Each instance gets its own binary, plugins, config, and data — nothing leaks between them.

## Why?

- **Try things without risk.** Spin up a fresh Neovim with LSP + DAP, break it, delete it — your other setups are untouched.
- **Per-project configs.** Keep a minimal instance for quick edits and a fully loaded one for development.
- **No system install required.** Everything lives in a directory you control. Carry it on a USB drive if you want.

## Quick Start

```bash
# Create an instance with LSP and tree explorer
pnm create work --features lsp,treeview

# Launch it
pnm launch work

# Open the interactive TUI
pnm tui
```

## Installation

Build from source (requires [Rust](https://rustup.rs/)):

```bash
cargo build --release
```

The binary is at `target/release/pnm` (or `pnm.exe` on Windows). Place it wherever you like — all supporting files (`workloads.json`, `settings.json`) live next to it.

## CLI Reference

### `pnm create <name>`

Create a new portable Neovim instance.

```bash
pnm create dev                           # Latest stable Neovim
pnm create nightly-test -v nightly       # Specific version
pnm create full -f lsp,dap,treeview,tabs # With features enabled
```

| Flag | Description |
|------|-------------|
| `-v, --version <tag>` | Neovim version tag (e.g. `v0.10.4`, `nightly`). Defaults to latest stable. |
| `-f, --features <list>` | Comma-separated features to enable. |
| `--js-runtime <value>` | JavaScript runtime override (`bun` or absolute path). See [JavaScript Runtime](#javascript-runtime). |

### `pnm list`

List all instances.

```
NAME                 VERSION         FEATURES                       UPDATED
--------------------------------------------------------------------------------
dev                  v0.10.4         Lsp, TreeView                  2025-03-20 14:30
nightly-test         nightly         Lsp, Dap                       2025-03-22 09:15
```

### `pnm info <name>`

Show detailed info about an instance.

### `pnm launch <name> [-- args...]`

Launch Neovim from an instance. Extra arguments are passed through to `nvim`.

```bash
pnm launch dev
pnm launch dev -- -u NONE          # Launch with no config
pnm launch dev -- myfile.txt       # Open a file
```

### `pnm update <name>`

Update an instance to the latest stable Neovim, or to a specific version with `--version`.

```bash
pnm update dev
pnm update dev --version v0.11.0
```

### `pnm delete <name>`

Delete an instance and all its data. Asks for confirmation unless `-y` is passed.

```bash
pnm delete old-test -y
```

### `pnm features <name>`

Toggle features on an existing instance.

```bash
pnm features dev --enable dap,tabs
pnm features dev --disable treeview
```

### `pnm marketplace`

Browse and install LSP servers, DAP adapters, formatters, and linters from the [mason-registry](https://github.com/mason-org/mason-registry).

```bash
pnm marketplace search python          # Search packages
pnm marketplace list --category lsp    # List LSP servers
pnm marketplace list --language Rust   # List Rust tools
pnm marketplace info pyright           # Show package details
pnm marketplace install dev pyright rust-analyzer debugpy  # Add to instance
pnm marketplace remove dev pyright     # Remove from instance
pnm marketplace refresh                # Update the cached registry
```

Installed packages are auto-installed by Mason on the next Neovim launch. The registry is cached locally and refreshed on demand.

### `pnm init`

Create a default `settings.json` next to the executable. Safe to run multiple times — won't overwrite an existing file.

### `pnm monitor <name>`

Show memory usage of a running Neovim instance.

```bash
pnm monitor dev                # Full memory snapshot
pnm monitor dev --no-lua       # Skip Lua heap query
```

Output includes:
- **Working set** (physical RAM) and **virtual memory** of the Neovim process
- **CPU usage** percentage
- **Lua heap** total (via RPC, unless `--no-lua`)
- **Child processes** (LSP servers, formatters, DAP adapters) with individual memory stats
- **Totals** across the process tree

```
Instance: dev (PID 12345)
──────────────────────────────────────────────────
Neovim process:
  Working Set:    128.4 MB
  Virtual Memory: 512.0 MB
  CPU:            2.3%

Lua Heap:         14.2 MB

Child Processes (3):
  PID      NAME                      WORKING SET     VIRTUAL
  23456    rust-analyzer              245.1 MB        1.2 GB
  23457    pyright                    89.3 MB         340.0 MB
  23458    lua-language-server        12.1 MB         80.0 MB

Total Working Set:  474.8 MB
Total Virtual:      2.1 GB
```

| Flag | Description |
|------|-------------|
| `--no-lua` | Skip the Lua heap memory query (useful if RPC is not available) |

### `pnm runtime <name>`

Get or set the JavaScript runtime for an instance. By default, plugins use the system Node.js. You can swap it for [Bun](https://bun.sh/) (or another runtime) to reduce memory usage.

```bash
pnm runtime dev                    # Show current runtime setting
pnm runtime dev --set bun          # Use Bun instead of Node
pnm runtime dev --set /path/to/bun # Custom path
pnm runtime dev --unset            # Revert to system Node
```

| Flag | Description |
|------|-------------|
| `--set <value>` | Set JS runtime (`bun` to find on PATH, or an absolute path) |
| `--unset` | Clear per-instance override, revert to global default |

### `pnm init-config <name>`

View, edit, or reset the init.lua configuration overrides for an instance.

```bash
pnm init-config dev                # Show current pre/post overrides
pnm init-config dev --edit-pre     # Edit pre-plugins Lua in $EDITOR
pnm init-config dev --edit-post    # Edit post-plugins Lua in $EDITOR
pnm init-config dev --reset        # Reset to smart defaults based on features
```

| Flag | Description |
|------|-------------|
| `--edit-pre` | Open `$EDITOR` to edit pre-plugins Lua (runs before `lazy.setup()`) |
| `--edit-post` | Open `$EDITOR` to edit post-plugins Lua (runs after plugin setup) |
| `--reset` | Regenerate smart defaults based on current enabled features |

### `pnm tui`

Open the interactive terminal UI for managing instances.

## TUI

The TUI provides a keyboard-driven interface for everything the CLI can do.

### Instance List (main screen)

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Enter` | View instance details |
| `l` | Launch instance |
| `u` | Update instance |
| `d` | Delete instance |
| `f` | Edit features |
| `m` | Change leader key |
| `o` | Open instance directory in file manager |
| `n` | Install Nerd Font (JetBrainsMono) |
| `s` | Initialize settings.json |
| `p` | Browse & install packages (marketplace) |
| `r` | Refresh list |
| `q` / `Esc` | Quit |

### Edit Features screen

Navigate with `j`/`k`, toggle with `Space`, apply with `Enter`. Dependencies are handled automatically — enabling OmniSharp also enables LSP, disabling LSP also disables OmniSharp/Node/Python.

### Edit Leader Key screen

Choose from Space, Comma, Backslash, or Semicolon. Applied immediately to `init.lua` on `Enter`.

### Marketplace screen

Browse the mason-registry catalog of 500+ packages. Navigate with `j`/`k`, switch categories with `Tab`, search with `/`, toggle packages with `Space`, and apply with `Enter`. Installed packages show a `●` marker.

### Monitor screen

Shows live memory usage of a running Neovim instance. Access from Instance Detail with `M`. Auto-refreshes every 2 seconds.

Displays working set (physical RAM), virtual memory, CPU %, Lua heap, and all child processes (LSP servers, formatters, DAP adapters) with individual memory stats. Press `r` to force refresh, `Esc` to go back.

### Init Config screen

View and manage init.lua configuration overrides. Access from Instance Detail with `I`.

Shows two panels: **Pre-plugins** (Lua that runs before `lazy.setup()`) and **Post-plugins** (Lua that runs after plugin setup). Both panels display Lua code with **syntax highlighting** (keywords, strings, comments, `vim.*` API, numbers). Press `Tab` to switch panels, `↑`/`↓` to scroll, `d` to reset to smart defaults, `Esc` to go back.

#### Inline editor

Press `e` to edit the focused panel in an inline syntax-highlighted editor. The editor supports:

- **Character input**, `Enter` (newline), `Backspace`, `Delete`
- **Arrow keys**, `Home`/`End` for cursor movement
- **Tab** to insert 2-space indent
- **Ctrl+Z** / **Ctrl+Y** for undo/redo (full-buffer snapshots, 100 levels)
- **Ctrl+S** to save changes and return to view mode (regenerates `init.lua` automatically)
- **Esc** to exit — prompts to discard if there are unsaved changes (`[modified]` indicator in header)

## Features (Workloads)

Features are modular plugin bundles. Base features are always included; optional ones are toggled per-instance.

### Always On

| Feature | Description |
|---------|-------------|
| **Telescope** | Fuzzy finder (`<leader>ff`, `<leader>fg`, `<leader>fb`, `<leader>fh`) |
| **Treesitter** | Syntax highlighting |

### Optional

| Feature | Aliases | Description | Depends On |
|---------|---------|-------------|------------|
| **Lsp** | `lsp` | Language Server Protocol (mason + lspconfig) | — |
| **Dap** | `dap` | Debug Adapter Protocol (nvim-dap + mason) | — |
| **TreeView** | `treeview`, `tree` | Left explorer panel (neo-tree) | — |
| **Tabs** | `tabs`, `tabline`, `bufferline` | Top tabbed editor bar (bufferline) | — |
| **OmniSharp** | `omnisharp`, `csharp`, `cs` | C# language server | Lsp |
| **Node** | `node`, `typescript`, `ts`, `js` | TypeScript/JavaScript language server | Lsp |
| **Python** | `python`, `py` | Python language server (pyright) | Lsp |

### Customizing Workloads

On first run, a `workloads.json` file is generated next to the executable with all built-in definitions. Edit it to add plugins, change keybindings, or define entirely new workloads.

Additionally, use `pnm marketplace` to browse and install individual LSP servers, DAP adapters, formatters, and linters from the mason-registry — these are auto-installed by Mason on next launch.

## Global Settings

Run `pnm init` (or press `s` in the TUI) to create a `settings.json` next to the executable:

```json
{
  "instances_dir": "C:\\Users\\you\\.portable-nvim\\instances"
}
```

| Field | Default | Description |
|-------|---------|-------------|
| `instances_dir` | `~/.portable-nvim/instances` | Where all instance directories are stored |

If `settings.json` doesn't exist, defaults are used. The file is designed to be forward-compatible — unknown fields are ignored, missing fields get defaults.

## User Customization

Each instance auto-loads a `user.lua` file if it exists:

```
<instance>/config/nvim/user.lua
```

Put your personal keymaps, options, or plugin configs there. It runs after the generated `init.lua`, so you can override anything.

## Gotchas

### First launch is slow

Lazy.nvim bootstraps itself and clones all plugins on the first launch of a new instance. This is normal — subsequent launches are fast.

### Nerd Font required for icons

TreeView and Tabs use Nerd Font glyphs. Press `n` in the TUI to install JetBrainsMono Nerd Font, then configure your terminal to use it. Without it, the UI falls back to ASCII characters but looks less polished.

### `workloads.json` is generated once

If you delete it, it's regenerated from built-in defaults on next run. If you've customized it, back it up. If parsing fails, the app falls back to built-in defaults with a warning.

### `init.lua` is regenerated on feature/leader changes

Don't hand-edit `init.lua` — your changes will be overwritten when you toggle features or change the leader key. Use `user.lua` instead.

### Windows: long path issues

Deeply nested plugin directories can hit the 260-character path limit on older Windows versions. Enable long paths in Windows settings or use a short `instances_dir` path in `settings.json`.

### Clipboard integration

The generated config sets `vim.opt.clipboard = "unnamedplus"`. This requires a clipboard provider — on Linux, install `xclip` or `wl-clipboard`.

### Version pinning

`pnm create` defaults to the latest stable release. If you need reproducibility, always pass `--version`:

```bash
pnm create ci-env --version v0.10.4
```

## JavaScript Runtime

Many Neovim plugins spawn Node.js processes — LSP servers, formatters, linters, Copilot, and DAP adapters all use `node`. These can consume significant memory (sometimes ~1GB+ combined).

pnm lets you transparently replace Node.js with [Bun](https://bun.sh/), a faster and more memory-efficient JavaScript runtime.

### How it works

When enabled, pnm creates a `shims/` directory in the instance with a fake `node` executable that delegates to Bun. This directory is prepended to PATH when launching Neovim, so every plugin that invokes `node`, `npm`, or `npx` transparently gets Bun instead.

Additionally, the generated `init.lua` includes a runtime detection block that sets `vim.g.copilot_node_command` and `vim.g.node_host_prog` to point at the shim. This ensures plugins like **copilot.vim** (which has its own Node discovery logic that bypasses PATH) and Neovim's built-in Node provider also use the alternative runtime.

### Enabling Bun

```bash
# Per-instance (CLI)
pnm runtime dev --set bun

# At creation time
pnm create dev --features lsp,treeview --js-runtime bun

# TUI: press B on the Instance Detail screen to toggle
```

### Global default

Set a global default in `settings.json` (or press Enter on "Default JS Runtime" in TUI settings):

```json
{
  "default_js_runtime": "bun"
}
```

Per-instance settings override the global default. Use `pnm runtime <name> --unset` to revert an instance to the global default.

### Compatibility

Bun is highly compatible with Node.js but not 100%. Most LSP servers, formatters, and linters work fine. Known caveats:

- **copilot.vim** — pnm automatically sets `vim.g.copilot_node_command` to redirect Copilot to the shim. If Copilot still misbehaves (e.g., Node version checks), revert with `pnm runtime <name> --unset`
- Some Mason packages that use native Node addons may not work with Bun
- Mason wrapper scripts created *before* enabling Bun may have hardcoded Node paths — reinstall affected Mason packages after enabling Bun (`MasonInstall <package>`)
- The shim is scoped to the instance — it doesn't affect other programs on your system

### Reverting

```bash
pnm runtime dev --unset    # CLI
# or press B again in TUI Instance Detail
```

## Init Config Overrides

pnm lets you inject custom Lua into the generated `init.lua` at two injection points:

- **Pre-plugins** — runs before `require("lazy").setup()`. Use for `vim.g` variables, `vim.opt` overrides, or anything that must be set before plugins load.
- **Post-plugins** — runs after plugin setup and feature config blocks. Use for autocmds, UI tweaks, or behaviors that depend on plugins being loaded.

### Smart defaults

When creating an instance, pnm auto-populates post-plugins with sensible defaults based on enabled features:

- **TreeView** → auto-opens the neo-tree explorer sidebar on startup (when no files are passed)

More defaults will be added for other features over time.

### Editing

```bash
pnm init-config dev --edit-post    # Edit post-plugins Lua in $EDITOR
pnm init-config dev --edit-pre     # Edit pre-plugins Lua in $EDITOR
pnm init-config dev --reset        # Reset to smart defaults

# TUI: press I on Instance Detail → view/edit/reset overrides
# In Init Config screen, press e to open inline syntax-highlighted editor
```

### Global defaults

Set global defaults in `settings.json`:

```json
{
  "default_init_lua_pre": "vim.opt.mouse = ''",
  "default_init_lua_post": "-- your custom post-plugins Lua"
}
```

Per-instance overrides take precedence. Instances with no override inherit the global default.

### How it works

The overrides are stored in the instance's `manifest.json` as `init_lua_pre` and `init_lua_post` strings. They're injected into the generated `init.lua` at their respective positions. The `user.lua` sourcing mechanism (at the very end of `init.lua`) remains available for additional customizations that shouldn't be managed by pnm.

## Monitoring

pnm can monitor the memory consumption of running Neovim instances — both the Neovim process itself and all child processes it spawns (LSP servers, formatters, DAP adapters, etc.).

### What gets measured

| Metric | Description |
|--------|-------------|
| **Working Set** | Physical RAM currently in use (RSS). This is the primary resource-pressure indicator — it's the memory your system actually has to provide. |
| **Virtual Memory** | Total address space reserved by the process. Typically much larger than working set because it includes memory-mapped files, shared libraries, and reserved-but-uncommitted pages. A high virtual memory number is normal and usually not a concern. |
| **CPU %** | CPU usage percentage at the time of the snapshot. |
| **Lua Heap** | Total memory used by Neovim's embedded Lua runtime (all plugins combined). Queried via Neovim's RPC interface using `collectgarbage("count")`. Per-plugin breakdown is not available because Neovim's Lua runtime doesn't track allocations per module. |
| **Child Processes** | Each process spawned by Neovim (LSP servers, formatters, DAP adapters) is reported individually with its own working set and virtual memory. Process names come from the OS process table. |

### How it works

When pnm launches Neovim, it:
1. Uses `.spawn()` to get the process ID (PID)
2. Writes `nvim.pid` to the instance directory
3. Adds `--listen <address>` to enable Neovim's RPC interface
4. Writes the RPC address to `nvim-rpc-addr.txt`
5. Cleans up both files when Neovim exits

The monitor reads the PID file, queries the OS for process memory stats using `sysinfo`, walks the process tree to find child processes, and optionally connects to Neovim's RPC to query the Lua heap.

### Usage

```bash
# CLI one-shot snapshot
pnm monitor my-env

# TUI live monitor (press M from Instance Detail)
pnm tui
```

### Limitations

- Only instances launched through pnm are monitorable (PID file is required)
- Lua heap is reported as a total — per-plugin memory breakdown is not supported by Neovim
- If Neovim crashes, the PID file may become stale; the monitor detects and cleans this up automatically

## Platform Support

| Platform | Archive Format | Binary |
|----------|---------------|--------|
| Windows x64 | `.zip` | `nvim.exe` |
| Linux x64 / ARM64 | `.tar.gz` | `nvim` |
| macOS x64 / ARM64 | `.tar.gz` | `nvim` |

## License

MIT
