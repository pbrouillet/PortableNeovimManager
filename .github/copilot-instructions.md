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

Rust CLI/TUI tool managing portable, self-contained Neovim installations. Each instance lives in `~/.portable-nvim/instances/<name>/` with isolated `bin/`, `config/`, `data/`, `cache/`, `state/` directories. Isolation is achieved by setting XDG env vars per launch.

### Module Responsibilities

- **cli** — `clap` subcommands: `create`, `list`, `info`, `launch`, `update`, `delete`, `features`, `tui`
- **config** — `InstanceManifest` (persisted as `manifest.json`), leader key helpers, directory helpers
- **workload** — `Workload` struct, `WorkloadRegistry` with lookup methods, loads `workloads.json` from next to the executable (falls back to built-in defaults)
- **github** — Fetch Neovim releases from GitHub API, select platform-correct asset, stream downloads
- **archive** — Extract `.zip` (Windows) / `.tar.gz` (Linux/macOS), install full Neovim tree into instance `bin/`
- **neovim** — Find nvim binary (handles nested extraction dirs), launch with XDG isolation, parse `--version`
- **instance** — Orchestrates lifecycle (create/update/delete/feature-toggle) by combining the above modules
- **font** — Downloads and installs JetBrainsMono Nerd Font to platform-specific user font directories
- **plugins/** — Generates `init.lua` with lazy.nvim bootstrap using workload plugin specs and config blocks from the registry
- **tui/** — `ratatui`/`crossterm` interactive interface with instance list, detail view, feature checkboxes, and leader key editing

### Key Patterns

- All async operations use `tokio`. GitHub downloads use `reqwest` with streaming progress.
- The `instance` module is the main orchestrator — CLI and TUI both call into it.
- **Workloads are data-driven.** All feature definitions (plugin specs, Lua config blocks, descriptions, dependencies, CLI aliases) live in `workloads.json` next to the executable. The `WorkloadRegistry` is loaded at startup and threaded through to all subsystems.
- `InstanceManifest.features` stores workload IDs as `Vec<String>` (e.g. `["Lsp", "Dap", "TreeView"]`).
- Base workloads (`Telescope`, `Treesitter`) are always included in every instance.
- Language workloads (`OmniSharp`, `Node`, `Python`) declare `depends_on: ["Lsp"]` — the TUI auto-enables/disables dependencies.
- `InstanceManifest` includes a `leader_key` field (defaults to space, `serde(default)` for backward compat with old manifests).
- TUI leaves alternate screen for operations that need the real terminal (launch, update, delete), then re-enters.

## Generated Neovim Lua Code

The `plugins/` module generates `init.lua` as Lua code. Plugin specs and config blocks are now loaded from `workloads.json` (via `WorkloadRegistry`) rather than hardcoded in Rust. This means Lua code quality depends on the JSON file contents.

### How init.lua generation works

`generate_init_lua()` in `plugins/init_lua.rs` takes `&WorkloadRegistry` and a list of enabled workload IDs. For each enabled workload (plus always-on base workloads), it reads `workload.plugins` for lazy.nvim spec strings and `workload.config_lua` for post-setup Lua blocks.

### Critical rules for generated Lua

- **Use `(vim.uv or vim.loop).fs_stat(path)`** for filesystem checks — NOT `vim.loop.isdirectory()` (doesn't exist) or `vim.fn.isdirectory()` inside the bootstrap path.
- **Do NOT use deprecated plugin APIs.** For example, `require("nvim-treesitter.configs").setup()` was removed in recent nvim-treesitter. Always verify against the plugin's current README.
- **Nerd Font fallbacks:** TreeView config detects Nerd Font via glyph display width and falls back to ASCII icons. Maintain this pattern for any new UI-facing plugin configs.
- **Test any Lua changes by actually launching a Neovim instance** — Rust compilation succeeding does not mean the Lua is valid.

## Cross-Platform Considerations

- **Asset selection** (`github.rs`): Must filter by both OS and architecture. `std::env::consts::ARCH` returns `"aarch64"` but Neovim assets use `"arm64"` — there's a mapping for this.
- **Archive format**: `.zip` for Windows, `.tar.gz` for Linux/macOS.
- **Font paths** (`font.rs`): Windows uses `%LOCALAPPDATA%\Microsoft\Windows\Fonts\`, macOS uses `~/Library/Fonts/`, Linux uses `~/.local/share/fonts/` + `fc-cache`.
- **Icon rendering**: Windows Terminal may not render Nerd Font glyphs without explicit font configuration. Generated Lua must always provide ASCII fallbacks.

## Workflow

After completing feature work, proactively commit, push, and bump the version tag (updating both `Cargo.toml` and the git tag) unless the user indicates otherwise.
