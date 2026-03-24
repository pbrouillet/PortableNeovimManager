# Architecture

This document describes the internal structure of Portable Neovim Manager (pnm) for contributors and anyone curious about how it works.

## High-Level Overview

```
┌─────────┐     ┌─────────────┐     ┌──────────────┐
│  CLI    │────▶│   main.rs   │────▶│  instance.rs  │──▶ create/update/delete/list
│ (clap)  │     │  (dispatch)  │     └──────┬───────┘
└─────────┘     └──────┬──────┘            │
                       │              ┌────▼────┐
┌─────────┐            │              │config.rs│──▶ GlobalSettings + InstanceManifest
│  TUI    │◀───────────┘              └────┬────┘
│(ratatui)│                                │
└─────────┘                           ┌────▼─────┐
                                      │neovim.rs │──▶ binary discovery + XDG launch
                                      └──────────┘

Supporting modules:
  github.rs   ─ GitHub Releases API client
  archive.rs  ─ ZIP/tar.gz extraction + binary installation
  workload.rs ─ Feature/plugin registry
  plugins/    ─ init.lua code generation
  font.rs     ─ Nerd Font installer
```

## Module Breakdown

### `main.rs` — Entry Point

Parses CLI arguments with clap, loads `GlobalSettings` and the `WorkloadRegistry`, then dispatches to the appropriate handler. Every command receives the settings so directory paths are consistent.

```
main()
  ├── config::load_global_settings()
  ├── workload::load_workloads()
  └── match command
        ├── Create  → instance::create()
        ├── List    → instance::list()
        ├── Info    → config::InstanceManifest::load()
        ├── Launch  → neovim::launch()
        ├── Update  → instance::update()
        ├── Delete  → instance::delete()
        ├── Features→ instance::update_features()
        ├── Init    → config::init_global_settings()
        └── Tui     → tui::run()
```

### `config.rs` — Configuration Layer

Owns two data structures and the directory layout:

**`GlobalSettings`** — App-wide configuration loaded from `settings.json` next to the executable. Currently holds `instances_dir` (where instances live). Every field uses `#[serde(default)]` for forward-compatibility — new fields can be added without breaking existing files.

**`InstanceManifest`** — Per-instance metadata stored as `manifest.json` inside the instance directory. Tracks name, Neovim version, enabled features, leader key, and timestamps.

**Directory helpers** — `instances_dir()`, `instance_dir()`, and `ensure_instance_dirs()` all accept `&GlobalSettings` so the storage location is configurable.

### `instance.rs` — Instance Lifecycle

Orchestrates the full lifecycle of an instance:

**Create:**
1. Validate instance doesn't already exist
2. Create directory skeleton (`bin/`, `config/nvim/`, `data/`, `cache/`, `state/`)
3. Fetch release from GitHub (latest stable or specific tag)
4. Download asset with progress bar
5. Extract archive to temp dir
6. Install Neovim binary tree into `bin/`
7. Generate `init.lua` from enabled workloads
8. Save `manifest.json`

**Update:**
1. Load existing manifest
2. Fetch target release
3. Compare versions (skip if already up to date)
4. Download + extract new binary
5. Replace `bin/` contents
6. Update manifest version and timestamp

**Delete:** Remove the entire instance directory.

**Update Features:**
1. Load manifest
2. Modify feature list
3. Regenerate `init.lua` with new feature set
4. Save manifest

### `neovim.rs` — Binary Discovery and Launch

**Binary discovery** (`find_nvim_binary`) searches the instance's `bin/` directory recursively for `nvim` (or `nvim.exe` on Windows). This handles varying archive structures — some releases nest the binary under `bin/nvim-win64/bin/nvim.exe`.

**Launch** sets four XDG environment variables to scope Neovim entirely to the instance:

```
XDG_CONFIG_HOME → <instance>/config
XDG_DATA_HOME   → <instance>/data
XDG_CACHE_HOME  → <instance>/cache
XDG_STATE_HOME  → <instance>/state
```

This is the core isolation mechanism. Neovim uses these paths for its config, plugins, cache, and state — so each instance is a completely independent Neovim installation.

### `github.rs` — GitHub Releases API Client

An async HTTP client using `reqwest` that talks to the GitHub Releases API for `neovim/neovim`.

**Release fetching:**
- `fetch_releases()` — gets the release list
- `fetch_latest_stable()` — finds the first non-prerelease
- `fetch_release_by_tag(tag)` — gets a specific release

**Asset selection** (`select_asset`) picks the right download for the current platform:

| Platform | Match Criteria |
|----------|---------------|
| Windows | Name contains `win64`, ends with `.zip` |
| Linux | Name contains `linux` + arch (`x86_64`/`arm64`), ends with `.tar.gz` |
| macOS | Name contains `macos` + arch, ends with `.tar.gz` |

Architecture mapping: Rust's `std::env::consts::ARCH` value `aarch64` is translated to `arm64` to match GitHub asset naming.

**Downloading** supports both simple (`download_asset`) and progress-reporting (`download_asset_with_progress`) variants. The progress variant streams the response and calls a closure with `(bytes_downloaded, total_bytes)`.

### `archive.rs` — Extraction and Installation

Handles two archive formats:

- **ZIP** (Windows releases) — extracted using the `zip` crate with Unix permission preservation
- **tar.gz** (Linux/macOS releases) — decompressed with `flate2`, extracted with `tar`

**`install_nvim_binary`** takes the extracted temp directory and copies the Neovim file tree into the instance's `bin/` directory using recursive directory copying. It finds the top-level extracted folder (e.g., `nvim-win64`) and copies its entire contents.

### `workload.rs` — Feature/Plugin Registry

A workload defines a composable feature:

```rust
struct Workload {
    id: String,              // "Lsp", "TreeView", etc.
    name: String,            // Display name
    description: String,     // User-facing text
    base: bool,              // Always included?
    depends_on: Vec<String>, // Required workload IDs
    plugins: Vec<String>,    // Lazy.nvim plugin spec strings
    config_lua: Option<String>, // Lua config block
    cli_aliases: Vec<String>,   // CLI aliases (case-insensitive)
}
```

**Loading:** On first run, `workloads.json` is generated next to the executable from built-in defaults. On subsequent runs it's loaded from disk. If parsing fails, built-in defaults are used. Users can edit `workloads.json` to customize plugins or add new workloads.

**Dependency tracking:** The registry tracks which workloads depend on which. The TUI uses `dependents_of()` to auto-disable features when their dependency is turned off, and the reverse when enabling.

### `plugins/init_lua.rs` — Lua Code Generation

Generates a complete `init.lua` for an instance:

1. **Lazy.nvim bootstrap** — clones lazy.nvim into the instance's data directory on first launch
2. **Leader key** — sets `mapleader` and `maplocalleader`
3. **Plugin specs** — collects specs from all active workloads (base + enabled optional) into a `lazy.setup({...})` call
4. **Feature configs** — appends each workload's `config_lua` block (keymaps, plugin setup calls)
5. **Base settings** — line numbers, termguicolors, clipboard, undo, search
6. **User hook** — loads `user.lua` from the config directory if it exists

The init.lua is regenerated whenever features or the leader key change. Users should put customizations in `user.lua` rather than editing `init.lua` directly.

### `font.rs` — Nerd Font Installer

Downloads and installs JetBrainsMono Nerd Font from GitHub. Platform-specific install locations:

- **Windows:** `%LOCALAPPDATA%\Microsoft\Windows\Fonts`
- **macOS:** `~/Library/Fonts`
- **Linux:** `~/.local/share/fonts` (runs `fc-cache -f` after install)

Checks for existing font files before downloading to avoid reinstallation.

### `tui/` — Terminal User Interface

Built with `ratatui` (rendering) and `crossterm` (terminal events).

**`app.rs`** — State machine with four screens:

```
InstanceList ──Enter──▶ InstanceDetail
     │                       │
     │ f                     │ f
     ▼                       ▼
EditFeatures            EditFeatures
     │                       │
     │ Esc                   │ Esc
     ▼                       ▼
InstanceList            InstanceDetail
```

The `App` struct holds all state: instance list, selection cursor, current screen, feature checkbox state, workload registry, and global settings.

Operations that need the terminal (launch, update, delete, font install) temporarily leave the alternate screen, run in the normal terminal, then re-enter the TUI.

**`ui.rs`** — Pure rendering functions. Each screen has a dedicated `draw_*` function that builds ratatui widgets (tables, paragraphs, styled text) and renders them to the frame.

## Data Flow: Creating an Instance

```
User: pnm create myenv --features lsp,treeview --version v0.10.4

1. cli.rs       Parse args → name="myenv", features=["lsp","treeview"], version="v0.10.4"
2. workload.rs  Resolve aliases → ["Lsp", "TreeView"]
3. config.rs    instance_dir(settings, "myenv") → ~/.portable-nvim/instances/myenv
4. config.rs    Check doesn't exist, create directory skeleton
5. github.rs    fetch_release_by_tag("v0.10.4") → Release { assets: [...] }
6. github.rs    select_asset() → "nvim-win64.zip" (on Windows)
7. github.rs    download_asset_with_progress() → bytes
8. archive.rs   extract(bytes, tmp_dir, "nvim-win64.zip")
9. archive.rs   install_nvim_binary(tmp_dir, bin_dir)
10. config.rs   InstanceManifest::new("myenv", "v0.10.4", ["Lsp","TreeView"])
11. plugins/    generate_init_lua(data_dir, registry, ["Lsp","TreeView"], " ")
12. fs::write   config/nvim/init.lua
13. manifest    manifest.json saved
```

## File Layout

### Next to the Executable

```
pnm.exe (or pnm)
workloads.json          # Generated on first run, user-editable
settings.json           # Optional, created by `pnm init`
```

### Instance Storage (default: `~/.portable-nvim/instances/`)

```
instances/
├── my-instance/
│   ├── manifest.json
│   ├── bin/
│   │   └── nvim-win64/    # Extracted Neovim distribution
│   │       ├── bin/
│   │       │   └── nvim.exe
│   │       ├── lib/
│   │       └── share/
│   ├── config/
│   │   └── nvim/
│   │       ├── init.lua   # Auto-generated
│   │       └── user.lua   # User customizations (optional)
│   ├── data/
│   │   └── lazy/          # Plugin installations
│   ├── cache/
│   └── state/
└── another-instance/
    └── ...
```

## Error Handling

Each module defines its own error type using `thiserror`:

- `ConfigError` — I/O and JSON errors for config files
- `InstanceError` — wraps ConfigError, GithubError, ArchiveError, NeovimError, and adds semantic variants (AlreadyExists, NotFound, AlreadyUpToDate)
- `GithubError` — HTTP and API errors
- `ArchiveError` — extraction failures
- `NeovimError` — binary not found, launch failures

The TUI catches errors and displays them as status messages rather than crashing.

## Testing

Tests are co-located with their modules (`#[cfg(test)] mod tests`). They use temp directories for isolation and clean up after themselves.

```bash
cargo test            # Run all tests
cargo test config     # Run config module tests only
```

Key test areas:
- Manifest save/load roundtrips and backward compatibility
- Feature alias parsing
- Directory structure creation
- GlobalSettings defaults and fallback behavior
- init.lua content verification (leader key, plugins, feature configs)
- Archive binary discovery
- Workload registry queries and JSON roundtrips
