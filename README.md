# MCPForge

<p align="center">
  <strong>The TUI that discovers every MCP client on your machine and syncs them all.</strong><br>
  <em>One native binary. 26 client adapters. 110 audited servers. Zero config fragmentation.</em>
</p>

<p align="center">
  <img src="https://github.com/nordicnode/mcpforge/actions/workflows/ci.yml/badge.svg" alt="CI" />
  <img src="https://img.shields.io/badge/Rust-2021_Edition-orange.svg?style=flat-square&logo=rust" alt="Rust 2021" />
  <img src="https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square" alt="License MIT" />
  <img src="https://img.shields.io/badge/Supported_Clients-26_Harnesses-purple.svg?style=flat-square" alt="26 Supported Clients" />
  <img src="https://img.shields.io/badge/Curated_Catalog-110_Audited_Servers-green.svg?style=flat-square" alt="110 Audited Servers" />
  <img src="https://img.shields.io/badge/Architecture-Modular_Workspace-blueviolet.svg?style=flat-square" alt="Modular Workspace" />
</p>

<p align="center">
  <a href="#overview">Overview</a> •
  <a href="#why-mcpforge">Why MCPForge?</a> •
  <a href="#screenshots">Screenshots</a> •
  <a href="#supported-clients">Supported Clients</a> •
  <a href="#installation">Installation</a> •
  <a href="#usage">Usage</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#license">License</a>
</p>

<p align="center">
  <img src="assets/screenshots/dashboard.png" alt="MCPForge Main Dashboard" width="95%" />
</p>

---

## Overview

As the Model Context Protocol ecosystem has grown, every AI client, autonomous agent harness, and code editor has introduced its own configuration format and path. Your tools end up fragmented across:

- `~/.agents/mcp.json` (Freebuff Desktop & CLI)
- `~/.claude.json` (Claude Code)
- `~/.deepseek/config.json` (DeepSeek Harness)
- `~/.config/goose/config.yaml` (Goose - YAML)
- `~/.hermes/config.yaml` (Hermes Agent - YAML)
- `~/.codex/config.toml` (Codex - TOML)
- `~/.grok/config.toml` (Grok Build - TOML)
- `~/.config/opencode/opencode.jsonc` (OpenCode - JSONC)
- Plus VS Code, Cursor, Windsurf, Zed, JetBrains, Continue.dev, Cline, and 13 others.

**MCPForge** eliminates this fragmentation. Built in Rust with a fast, zero-flicker [Ratatui](https://ratatui.rs) terminal UI, MCPForge gives you an interactive command center to inspect live client processes, provision audited MCP servers, verify schema drift, and sync configurations across all 26 clients simultaneously.

---

## Why MCPForge?

| Capability | **MCPForge** | mcpm.sh / mcpman | mcps / APM |
| :--- | :---: | :---: | :---: |
| **Interface** | **Interactive TUI + Scriptable CLI** | Pure CLI | Pure CLI |
| **Runtime** | **Single native binary (Rust, zero Node.js)** | Node.js / npm | Python or Node.js |
| **Client Process Discovery** | **Live OS process watcher + scanner** | Manual | None |
| **Client Ecosystem** | **26 Clients (JSON, JSONC, YAML, TOML)** | 1–3 Clients | 1–4 Clients |
| **Format Preservation** | **Non-destructive AST round-tripping** | Overwrites or clobbers | Clobbers root keys |
| **Diff Preview** | **Unified color diff before disk write** | None | None |
| **Schema Drift Audit** | **`mcpforge verify` built-in** | None | None |
| **Catalog Provenance** | **110 audited servers with source URL & audit dates** | Unverified / Partial | Unverified |
| **Self-Healing Doctor** | **Sub-millisecond ping + auto-fix** | Basic ping | None |

---

## Key Features

- **Keyboard-Driven Terminal UI**: High-speed, zero-flicker TUI with instant filtering, intuitive split views, and vim/arrow navigation.
- **26 First-Class Client Adapters**: Native read and write support for autonomous agents, terminal CLIs, full IDEs, and chat desktop clients.
- **110+ Curated MCP Servers**: Instant one-click provisioning across AI agents, analytical databases, developer tools, cloud infrastructure, git workflows, search engines, and enterprise productivity platforms.
- **Adapter-Accurate Diff Previews**: Every configuration modification is simulated using the target client's actual format engine before touching disk, guaranteeing that existing configurations are never overwritten or corrupted.
- **Atomic Safety & Backups**: Automatic `.bak` sidecar snapshots are created before any file is updated, with atomic file swap semantics.
- **Live Telemetry & Diagnostics**: Background ping engine queries stdio subprocesses and HTTP/SSE endpoints with sub-millisecond precision, reporting latency, active tool counts, and server versions.
- **Interactive Server Removal**: Safely purge servers across all clients at once or interactively pick and choose targets.
- **Portable Profiles & Packs**: Export your entire multi-tool MCP environment into reproducible `mcpforge-pack.json` bundles and import them on new machines with automated secret resolution.

---

## Screenshots

### 1. Unified Dashboard & Runtime Telemetry
Inspect configured servers, execution arguments, environment variables, client installation matrices, and live sub-millisecond diagnostics.

<p align="center">
  <img src="assets/screenshots/dashboard.png" alt="MCPForge Dashboard" width="95%" />
</p>

---

### 2. Supported Clients & Agent Harness Matrix
View all 26 supported AI harnesses, categorized by lifecycle state (`ACTIVE`, `RUNNING`, `READY`, `AVAILABLE`), with binary detection, disk paths, and configured servers.

<p align="center">
  <img src="assets/screenshots/clients.png" alt="Clients & Harnesses Matrix" width="95%" />
</p>

---

### 3. Curated 110-Server Catalog with Environment Checks
Filter pre-tested MCP servers by category (`Agents`, `Dev Tools`, `Data & DBs`, `Web`, `Git`, `Cloud`, `Productivity`) with real-time environment variable validation.

<p align="center">
  <img src="assets/screenshots/catalog.png" alt="Catalog with Category Filtering" width="95%" />
</p>

---

### 4. Adapter-Accurate Unified Diff Preview
Preview the exact unified diff generated for each client adapter before applying, guaranteeing zero accidental deletions or syntax corruption.

<p align="center">
  <img src="assets/screenshots/diff_preview.png" alt="Unified Diff Preview" width="95%" />
</p>

---

### 5. Interactive Removal Modal
Safely decommission servers from all clients at once, or use the interactive checklist to remove from specific clients with automatic `.bak` backups.

<p align="center">
  <img src="assets/screenshots/removal_modal.png" alt="Interactive Server Removal Modal" width="95%" />
</p>

---

## Supported Clients

MCPForge provides native, format-preserving adapters for **26 distinct AI clients and harnesses**:

| Category | Client / Harness | Config Path | Format | Status |
| :--- | :--- | :--- | :--- | :--- |
| **Agent** | **Freebuff Desktop & CLI** | `~/.agents/mcp.json` | JSON | Supported |
| **Agent** | **DeepSeek Harness** | `~/.deepseek/config.json` | JSON | Supported |
| **Agent** | **Goose** | `~/.config/goose/config.yaml` | YAML | Supported |
| **Agent** | **Hermes Agent** | `~/.hermes/config.yaml` | YAML | Supported |
| **Agent** | **OpenClaw** | `~/.openclaw/openclaw.json` | JSON | Supported |
| **Agent** | **Prime Agent** | `~/.prime/agent/mcp.json` | JSON | Supported |
| **Agent** | **Letta Agent** | `~/.letta/mcp.json` | JSON | Supported |
| **CLI** | **Claude Code** | `~/.claude.json` | JSON | Supported |
| **CLI** | **Codex** | `~/.codex/config.toml` | TOML | Supported |
| **CLI** | **Grok Build** | `~/.grok/config.toml` | TOML | Supported |
| **CLI** | **OpenCode** | `~/.config/opencode/opencode.jsonc` | JSONC | Supported |
| **CLI** | **Antigravity / Gemini** | `~/.gemini/config/mcp_config.json` | JSON | Supported |
| **CLI** | **J-Code** | `~/.jcode/servers.json` | JSON | Supported |
| **CLI** | **Manicode** | `~/.config/manicode/mcp.json` | JSON | Supported |
| **IDE** | **Cursor** | `~/.cursor/mcp.json` | JSON | Supported |
| **IDE** | **VS Code** | `~/.vscode/mcp.json` | JSON | Supported |
| **IDE** | **Windsurf** | `~/.codeium/windsurf/mcp_config.json` | JSON | Supported |
| **IDE** | **Zed** | `~/.config/zed/settings.json` | JSON | Supported |
| **IDE** | **JetBrains IDEs** | `~/.config/JetBrains/mcp.json` | JSON | Supported |
| **IDE** | **Neovim (MCPHub)** | `~/.config/mcphub/servers.json` | JSON | Supported |
| **IDE** | **Cline** | `~/.config/Code/.../cline_mcp_settings.json` | JSON | Supported |
| **IDE** | **Roo Code** | `~/.config/Code/.../cline_mcp_settings.json` | JSON | Supported |
| **IDE** | **Continue.dev** | `~/.continue/config.json` | JSON | Supported |
| **Chat** | **Claude Desktop** | `~/.config/Claude/claude_desktop_config.json` | JSON | Supported |
| **Chat** | **LibreChat** | `~/.librechat/librechat.yaml` | YAML | Supported |
| **Chat** | **AnythingLLM** | `~/.config/anythingllm-desktop/...` | JSON | Supported |

---

## Installation

### From Source (Recommended)

Ensure you have Rust 1.80+ and `cargo` installed:

```bash
git clone https://github.com/nordicnode/mcpforge.git
cd mcpforge
cargo build --release
sudo cp target/release/mcpforge /usr/local/bin/
```

### Quick Cargo Install

```bash
cargo install --git https://github.com/nordicnode/mcpforge.git
```

---

## Usage

### Interactive TUI

Launch the full interactive command center:

```bash
mcpforge
```

#### Keybindings

| Key | Context | Action |
| :--- | :--- | :--- |
| `Tab` / `1` / `2` | Global | Switch between `[1] Servers` and `[2] Clients` views |
| `j` / `k` or `Down` / `Up` | Navigation | Move cursor through server or client lists |
| `Space` | Servers View | Toggle server enabled / disabled |
| `a` | Global | Open Add Server Wizard (4-step provisioning) |
| `d` / `Delete` / `x` | Global | Open Interactive Server Removal Modal |
| `u` | Global | Sync all servers across all detected clients |
| `r` | Global | Run instant diagnostic health checks & ping latencies |
| `/` | Dashboard | Fuzzy search configured servers and tags |
| `?` | Global | Open full interactive keyboard guide |
| `q` / `Esc` | Global | Quit application |

---

### Command Line Interface

MCPForge provides a scriptable CLI for automation, CI/CD pipelines, and dotfile management:

```bash
# Discover all installed AI harnesses and client configuration files
mcpforge discover

# Audit all detected client configuration files for syntax errors and schema drift
mcpforge verify

# Audit a specific client adapter only
mcpforge verify --client codex

# Auto-synchronize all configured servers across every detected client
mcpforge sync --auto

# List all configured MCP servers and client associations
mcpforge list

# Run diagnostic health checks and measure latency for all servers
mcpforge doctor

# Add a server from the curated catalog to all installed clients
mcpforge setup postgres

# Add a server to specific clients only
mcpforge setup github --to freebuff,deepseek,claude-code

# Remove a server across all clients
mcpforge remove brave-search --all

# Export multi-client setup to a portable JSON file (with secrets redacted)
mcpforge export --output my-team-mcp.json

# Import and provision servers onto a new system
mcpforge import --input my-team-mcp.json
```

---

## Architecture

MCPForge is built as a modular Cargo workspace designed for speed, safety, and extensibility:

```
mcptui/
├── crates/
│   ├── mcp-core/              # MCP protocol primitives, JSON-RPC 2.0, Transports (Stdio, HTTP, SSE)
│   ├── mcpforge-adapters/     # 26 client adapters, format AST engines, atomic backup system
│   ├── mcpforge-registry/     # Embedded registry with 110+ curated server catalog entries
│   └── mcpforge/              # Ratatui TUI application, CLI dispatch, process watcher
├── catalog/
│   └── default_registry.json  # Curated catalog definitions and environment mappings
├── assets/
│   └── screenshots/           # High-resolution retina terminal captures
└── scripts/
    └── render_screenshots.py  # Headless PIL screenshot generation engine
```

---

## Contributing

Contributions, bug reports, and new client adapter submissions are welcome!

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/my-new-adapter`)
3. Commit your changes (`git commit -m 'feat(adapter): add support for MyClient'`)
4. Verify tests and linting:
   ```bash
   cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
   ```
5. Push to your branch and open a Pull Request

---

## License

Distributed under the MIT License. See `LICENSE` for details.
