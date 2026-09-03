# MCPForge

> **A terminal UI and CLI for discovering, installing, configuring, and health-checking MCP servers across every local AI client.**

---

## What makes MCPForge different?

MCPForge is designed to be **as automated as possible**:
1. **Automated Client & Process Discovery**: Scans running OS processes and standard config locations to find all installed and active AI clients (Antigravity, Claude Desktop, Claude Code, Cursor, VS Code, Cline, Continue.dev, Windsurf, Zed).
2. **Automated Credential & Secret Resolution**: Automatically extracts required tokens and credentials using local tooling (`gh auth token`), `.env` files, and environment variables—no manual token copy-pasting.
3. **One-Command Setup & Verification**: `mcpforge setup <server>` checks runtimes, fetches tokens, installs into all detected clients, and immediately tests live health via the MCP JSON-RPC handshake.
4. **Curated Server Packs**: Install complete developer stacks (`dev-core`, `data`, `web-research`, `cloud-dev`) in a single command.
5. **Self-Healing Diagnostics (`doctor --fix`)**: Detects missing tokens or configuration drift and automatically repairs them.
6. **Zero-Diff Safety**: Always creates timestamped `.bak` backups before modifying files and preserves existing formatting, comments, and unknown keys with atomic writes.

---

## Supported AI Clients & Harnesses

| Client | Auto-Detected Paths | Transport Support | Active Process Detection |
|---|---|---|---|
| **Antigravity / Gemini** | `~/.gemini/config/mcp_config.json` | stdio, Streamable HTTP | ✓ |
| **Claude Desktop** | `~/.config/Claude/claude_desktop_config.json` (Linux/Mac/Win) | stdio | ✓ |
| **Claude Code** | `~/.claude.json`, `.mcp.json` | stdio, Streamable HTTP | ✓ |
| **Cursor** | `~/.cursor/mcp.json`, `.cursor/mcp.json` | stdio, Streamable HTTP, SSE | ✓ |
| **VS Code / Copilot** | `~/.vscode/mcp.json`, `.vscode/mcp.json` | stdio, Streamable HTTP | ✓ |
| **Cline** | VS Code & Cursor extension settings | stdio, Streamable HTTP | ✓ |
| **Windsurf** | `~/.codeium/windsurf/mcp_config.json` | stdio | ✓ |
| **Continue.dev** | `~/.continue/config.json` | stdio | ✓ |
| **Zed** | `~/.config/zed/settings.json` | stdio | ✓ |
| **Custom Harnesses** | `mcpforge.toml` | all | User-defined |

---

## Automated CLI Commands

```bash
# 1. Audit and discover all AI clients & running processes on the machine
mcpforge discover

# 2. Automated one-command setup: auto-resolves tokens (e.g. from `gh auth token` or `.env`),
#    installs to all detected clients, and immediately tests health
mcpforge setup github
mcpforge setup filesystem
mcpforge setup postgres

# 3. View and install curated server packs
mcpforge pack list
mcpforge pack install dev-core       # installs filesystem, git, memory, and fetch
mcpforge pack install data           # installs postgres, sqlite, and memory
mcpforge pack install web-research   # installs brave-search, fetch, and puppeteer

# 4. List all configured servers across all clients
mcpforge list

# 5. Automatically sync all configured servers across all detected clients
mcpforge sync --auto

# 6. Run diagnostic health checks with automated repair
mcpforge doctor --fix

# 7. Export/Import configurations (secrets masked by default)
mcpforge export --output mcp-backup.json
mcpforge import mcp-backup.json
```

---

## Interactive Terminal UI (TUI)

Launch the dashboard:

```bash
cargo run -p mcpforge
```

### Keybindings
- `j` / `Down`: Move cursor down
- `k` / `Up`: Move cursor up
- `u`: **Auto-Sync** all servers across all detected clients in one keystroke
- `r`: Run diagnostic health check on selected server
- `a`: Open Add Server Wizard (Registry / JSON paste / Manual) with diff preview
- `d`: Delete selected server
- `Space`: Toggle server enable / disable
- `/`: Filter servers
- `?`: Help overlay
- `q`: Quit

---

## Testing & Quality Gates

```bash
# Workspace unit and integration tests
cargo test --workspace

# Strict clippy linting
cargo clippy --workspace --all-targets -- -D warnings

# Formatting check
cargo fmt --all -- --check
```

---

## License

MIT
