# Copilot Instructions

## Build & Run

```sh
cargo build
cargo run -- <subcommand>    # e.g. cargo run -- create myenv --features lsp,dap
cargo test
cargo test test_name         # run a single test
cargo clippy                 # lint
cargo fmt --check            # check formatting
```

### Build Gotchas

- **Locked binary:** On Windows, `cargo build` will fail with "Access is denied (os error 5)" if `pnm.exe` is running (e.g. TUI open, or the exe was launched directly). Stop the process first, or build to an alternate target dir.
- **Workload defaults are compiled in.** Changes to `src/workload/defaults.rs` may not trigger a recompile if the file timestamp didn't change (e.g. only data strings changed). When in doubt, run `cargo clean -p portable-neovim-manager && cargo build`.

## Versioning

The version in `Cargo.toml` and git tags must stay in sync. When bumping a version:
1. Update `version` in `Cargo.toml`
2. Create the git tag (e.g. `git tag v1.2.0`)
3. Push both: `git push && git push origin <tag>`

Tags follow semver with a `v` prefix (e.g. `v1.2.0`). The GitHub Actions release workflow triggers on tagged pushes.

## Architecture

Rust CLI/TUI tool managing portable, self-contained Neovim installations. Each instance lives in `~/.portable-nvim/instances/<name>/` with isolated `bin/`, `config/`, `data/`, `cache/`, `state/` directories. Isolation is achieved by setting XDG env vars (`XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`, `XDG_STATE_HOME`) per launch.

See `ARCHITECTURE.md` for the full module breakdown, data flow diagrams, and file layout. Below covers what you need to make changes effectively.

### Module Responsibilities

- **cli** — `clap` subcommands dispatched from `main.rs`
- **config** — `GlobalSettings` (next to exe as `settings.json`) + `InstanceManifest` (per-instance `manifest.json`), directory helpers
- **instance** — Orchestrates lifecycle (create/update/delete/feature-toggle). CLI and TUI both call into it — this is the main coordination layer.
- **workload/** — Data-driven feature registry: `model.rs` (Feature/Workload/Preset/Tutorial types + `WorkloadRegistry`), `loader.rs` (JSON loading from disk), `defaults.rs` (built-in defaults compiled into binary)
- **plugins/** — `init_lua.rs` generates complete `init.lua` from registry + enabled features. `init_defaults.rs` produces smart per-feature Lua defaults.
- **github** — GitHub Releases API client, platform asset selection, streaming downloads
- **archive** — ZIP/tar.gz extraction, Neovim binary installation into instance `bin/`
- **neovim** — Binary discovery (handles nested extraction dirs), XDG-isolated launch, PID/RPC file management
- **font/** — Nerd Font installer + terminal provider abstraction (`wt.rs`, `alacritty.rs`, `kitty.rs`, `gnome_terminal.rs`, `iterm2.rs`, `konsole.rs`). All file-based providers create `.pnm-backup` before modifying configs.
- **mason_registry/** — Fetches, caches, and queries the mason-registry (LSP servers, DAP adapters, formatters, linters)
- **monitor** — Process memory monitoring via `sysinfo` + Lua heap via Neovim RPC
- **runtime** — JS runtime shim system (transparently replaces Node with Bun via `shims/` directory + PATH manipulation)
- **tui/** — `ratatui`/`crossterm` interactive UI. `app.rs` is the state machine, `ui.rs` has render functions, `screens/*.rs` has per-screen event handlers, `lua_highlight.rs` is a Lua syntax tokenizer for TUI display.

### Error Handling

Each module defines its own `thiserror` error enum (e.g. `ConfigError`, `GithubError`, `ArchiveError`). `InstanceError` wraps them all via `#[error(transparent)]` + `#[from]`. Follow this pattern when adding new modules.

Config/settings loading uses **fallback-to-defaults** instead of bubbling errors — if `settings.json` or `workloads.json` is missing or malformed, the app prints a warning and uses defaults. This is intentional for first-run and forward-compatibility.

### Config Persistence

Two-level persistence:
- **`GlobalSettings`** → `settings.json` next to the executable. All fields use `#[serde(default)]` for forward-compat.
- **`InstanceManifest`** → `manifest.json` inside each instance dir. Uses `#[serde(alias = "features")]` for backward-compat with old manifests that stored `features` instead of `workloads`.

The standard mutation cycle is: load manifest → modify fields → save manifest → regenerate `init.lua`. That final regeneration is a key coupling — any change to features, leader key, runtime, or init config overrides must trigger `generate_init_lua()`.

### Workload Data Model

Workloads are data-driven. All feature definitions live in `workloads.json` next to the executable (generated from `defaults.rs` on first run, user-editable after).

Hierarchy: `WorkloadRegistry` → `Workload` → `Feature`. Each `Feature` contains `plugins` (lazy.nvim spec strings) and `config_lua` (Lua config block). Workloads can declare `depends_on` for transitive dependency resolution. Base workloads (`Telescope`, `Treesitter`) are always included.

Old-format workloads (flat `plugins`/`config_lua` on workload level) are auto-migrated to feature-based shape via `normalize()` in the loader.

### TUI Architecture

Single mutable `App` struct with a `Screen` enum state machine. Screen transitions are explicit field assignments. Per-screen event handlers live in `tui/screens/*.rs`. Rendering is dispatched by matching `Screen` variants in `ui.rs` → `draw_*` functions.

The TUI leaves alternate screen for terminal-needing operations (launch, update, font install), runs them in the normal terminal, then re-enters. Mouse hit-testing uses a `LayoutCache` populated during render.

## Generated Neovim Lua Code

`plugins/init_lua.rs` generates `init.lua` as string templates (not AST). The generation order matters:

1. Lazy.nvim bootstrap
2. Leader key assignment
3. JS runtime shim detection block
4. **Pre-plugins** user/auto Lua (`init_lua_pre` from manifest)
5. `require("lazy").setup({...})` with collected plugin specs
6. Feature config blocks (`config_lua` from each active feature)
7. Mason auto-install block (if packages selected)
8. **Post-plugins** user/auto Lua (`init_lua_post` from manifest)
9. Base vim options (line numbers, clipboard, undo, search)
10. `user.lua` sourcing (optional user customization file)

Plugin specs and config blocks come from the `WorkloadRegistry`. Individual features can be disabled via `disabled_features` (stored as `"WorkloadId/FeatureId"` paths).

### Critical rules for generated Lua

- **Use `(vim.uv or vim.loop).fs_stat(path)`** for filesystem checks — NOT `vim.loop.isdirectory()` (doesn't exist) or `vim.fn.isdirectory()` inside the bootstrap path.
- **Do NOT use deprecated plugin APIs.** For example, `require("nvim-treesitter.configs").setup()` was removed in recent nvim-treesitter. Always verify against the plugin's current README.
- **Nerd Font fallbacks:** TreeView config detects Nerd Font via glyph display width and falls back to ASCII icons. Maintain this pattern for any new UI-facing plugin configs.
- **Test any Lua changes by actually launching a Neovim instance** — Rust compilation succeeding does not mean the Lua is valid.

## Cross-Platform Considerations

- **Asset selection** (`github.rs`): Must filter by both OS and architecture. `std::env::consts::ARCH` returns `"aarch64"` but Neovim assets use `"arm64"` — there's a mapping for this.
- **Archive format**: `.zip` for Windows, `.tar.gz` for Linux/macOS.
- **Font paths** (`font.rs`): Windows uses `%LOCALAPPDATA%\Microsoft\Windows\Fonts\`, macOS uses `~/Library/Fonts/`, Linux uses `~/.local/share/fonts/` + `fc-cache`. Terminal font providers create `.pnm-backup` before editing config files.
- **Icon rendering**: Windows Terminal may not render Nerd Font glyphs without explicit font configuration. Generated Lua must always provide ASCII fallbacks.
- **JS runtime shims** (`runtime.rs`): Windows uses `.cmd` batch files + `.exe` copy; Unix uses shell scripts with `exec`. The shims directory is prepended to PATH at launch.

## Testing

Tests are co-located with their modules via `#[cfg(test)] mod tests` — there is no separate `tests/` directory. Tests use real filesystem operations with temp directories for isolation. No mocking framework is used.

```sh
cargo test                    # all tests
cargo test config             # tests in the config module
cargo test test_function_name # a specific test
```

## Workflow

After completing feature work, proactively commit, push, and bump the version tag (updating both `Cargo.toml` and the git tag) unless the user indicates otherwise.
