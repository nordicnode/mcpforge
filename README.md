# MCPForge

> **A terminal UI and CLI for discovering, installing, configuring, and health-checking MCP servers across every local AI client.**

---

## Features

- **Multi-Client Support**: Seamlessly manages MCP server configurations across:
  - Claude Desktop (`claude_desktop_config.json`)
  - Claude Code (`~/.claude.json`, `.mcp.json`)
  - Cursor (`~/.cursor/mcp.json`, `.cursor/mcp.json`)
  - VS Code / Copilot (`~/.vscode/mcp.json`, `.vscode/mcp.json`)
  - Windsurf (`~/.codeium/windsurf/mcp_config.json`)
  - Custom harnesses (`mcpforge.toml`)
- **Safety First**:
  - Timestamped backups created before every single write (`~/.local/state/mcpforge/backups/`).
  - Order-preserving JSON manipulation keeping unknown keys and user formatting intact.
  - Atomic file writes via temporary files and rename.
  - Interactive unified diff preview before applying changes.
- **Diagnostics (`doctor`)**:
  - Live protocol handshake (`initialize`, `notifications/initialized`, `tools/list`).
  - Binary existence verification on `$PATH`.
  - Latency measurement and tool count tracking.
- **Curated Local-First Registry**:
  - Offline-first catalog bundled directly into the binary with top community MCP servers.
  - Real-time fuzzy search across names, categories, and tags.
- **Secrets Management**:
  - Sensitive environment variables (`TOKEN`, `KEY`, `SECRET`, `PASSWORD`) masked by default.
  - Export mode with optional `--include-secrets` flag.
  - Restricted permissions (`0600`) for sensitive cache stores.

---

## Architecture

```
mcptui/
├── Cargo.toml                      # Root Cargo workspace
├── crates/
│   ├── mcp-core/                   # Core protocol types, stdio & HTTP transports, JSON-RPC client
│   ├── mcpforge-adapters/          # Client adapters, atomic writer, backup manager
│   ├── mcpforge-registry/          # Curated server catalog & fuzzy search
│   └── mcpforge/                   # CLI entry point & Ratatui TUI dashboard
└── catalog/
    └── default_registry.json       # Built-in offline curated catalog
```

---

## Usage

### Interactive TUI

Launch the full interactive terminal interface:

```bash
cargo run -p mcpforge
```

#### Keybindings
- `j` / `Down`: Navigate down
- `k` / `Up`: Navigate up
- `/`: Search / filter servers
- `r`: Trigger health check diagnostics
- `a`: Launch Add Server Wizard (Registry / JSON paste / Manual)
- `d`: Delete selected server
- `Space`: Toggle server enable / disable
- `?`: Toggle help overlay
- `q` / `Esc`: Quit

### CLI Commands

```bash
# List all configured servers across all detected clients
mcpforge list

# Filter list by client
mcpforge list --client cursor

# Output server list as JSON
mcpforge list --json

# Run doctor health checks
mcpforge doctor

# Add a server from the curated registry
mcpforge add filesystem --from-registry --to cursor,claude-code

# Add a server via JSON snippet on stdin
echo '{"command":"npx","args":["-y","@modelcontextprotocol/server-memory"]}' | mcpforge add memory --stdin --to cursor

# Sync servers between clients
mcpforge sync cursor --from claude-code

# Export all server configurations (secrets redacted by default)
mcpforge export --output mcp-backup.json

# Import server configurations from export file
mcpforge import mcp-backup.json --to vscode
```

---

## Testing & Quality Assurance

```bash
# Run all unit and integration tests across the workspace
cargo test --workspace

# Run strict clippy linting
cargo clippy --workspace --all-targets -- -D warnings

# Check code formatting
cargo fmt --all -- --check
```

---

## License

MIT
