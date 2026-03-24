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

## Platform Support

| Platform | Archive Format | Binary |
|----------|---------------|--------|
| Windows x64 | `.zip` | `nvim.exe` |
| Linux x64 / ARM64 | `.tar.gz` | `nvim` |
| macOS x64 / ARM64 | `.tar.gz` | `nvim` |

## License

MIT
