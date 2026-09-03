# MCPForge

> **A high-performance terminal UI and automation CLI for discovering, installing, configuring, and health-checking MCP servers across 16+ local AI clients and harnesses.**

---

## What makes MCPForge different?

MCPForge is built for **zero-friction automation**:
1. **Automated Client & Process Discovery**: Scans running OS processes (`/proc` and `pgrep`) and standard configuration paths to locate all installed and active AI clients on the machine.
2. **50+ Curated Server Catalog**: Built-in, offline-ready registry covering filesystems, databases (Postgres, MySQL, SQLite, Mongo, Redis, Supabase, Qdrant, Neo4j), web search (Brave, Tavily, Perplexity, Fetch, Puppeteer, Playwright), cloud devops (Docker, Kubernetes, AWS, Cloudflare, Sentry, Datadog), productivity (Slack, Discord, Linear, Jira, Notion, Obsidian, Google Drive, Todoist), and reasoning agents.
3. **Automated Credential & Secret Resolution**: Automatically extracts tokens and credentials from local developer tooling (`gh auth token`), `.env` files, and encrypted secret stores.
4. **One-Command Setup & Verification**: `mcpforge setup <server>` checks execution runtimes, resolves tokens, writes configurations across all detected clients, and immediately tests live health via the MCP JSON-RPC protocol.
5. **Curated Multi-Server Packs**: Install complete developer suites (`dev-core`, `data`, `web-research`, `cloud-dev`, `productivity`, `ai-agent`, `full-stack`, `enterprise`) in one shot.
6. **Cross-Client Auto-Sync (`mcpforge sync --auto`)**: Aggregates all servers configured across any client and synchronizes them to every installed client with a single keystroke (`[u]` in TUI).
7. **Zero-Diff Safety**: Creates timestamped `.bak` backups before modifying files and preserves formatting, comments, and unknown keys with atomic writes.

---

## Supported AI Clients & Harnesses (16+)

| Client / Harness | Default Config Paths | Transport Support | Live Process Detection |
|---|---|---|---|
| **Freebuff** | `~/.config/freebuff-desktop/mcp.json`, `~/.freebuff/mcp.json`, `.freebuff/mcp.json` | stdio, Streamable HTTP | ✓ |
| **Grok Build / Grok CLI** | `~/.grok/config.toml`, `.grok/config.toml` | stdio, Streamable HTTP | ✓ |
| **J-Code** | `~/.jcode/servers.json`, `~/.config/jcode/servers.json` | stdio, Streamable HTTP | ✓ |
| **OpenCode** | `~/.config/opencode/mcp.json`, `~/.opencode/mcp.json`, `.opencode/mcp.json` | stdio, Streamable HTTP | ✓ |
| **Codex** | `~/.codex/config.json`, `~/.config/codex/mcp.json`, `.codex/mcp.json` | stdio, Streamable HTTP | ✓ |
| **Roo Code** | `cline_mcp_settings.json` (VS Code & Cursor), `.roo/mcp.json` | stdio, Streamable HTTP | ✓ |
| **Manicode** | `~/.config/manicode/mcp.json`, `.manicode/mcp.json` | stdio, Streamable HTTP | ✓ |
| **Antigravity / Gemini** | `~/.gemini/config/mcp_config.json` | stdio, Streamable HTTP | ✓ |
| **Cline** | `saoudrizwan.claude-dev/settings/cline_mcp_settings.json` | stdio, Streamable HTTP | ✓ |
| **Claude Desktop** | `~/.config/Claude/claude_desktop_config.json` (Linux/Mac/Win) | stdio | ✓ |
| **Claude Code** | `~/.claude.json`, `.mcp.json` | stdio, Streamable HTTP | ✓ |
| **Cursor** | `~/.cursor/mcp.json`, `.cursor/mcp.json` | stdio, Streamable HTTP, SSE | ✓ |
| **VS Code / Copilot** | `~/.vscode/mcp.json`, `.vscode/mcp.json` | stdio, Streamable HTTP | ✓ |
| **Windsurf** | `~/.codeium/windsurf/mcp_config.json` | stdio | ✓ |
| **Continue.dev** | `~/.continue/config.json` | stdio | ✓ |
| **Zed** | `~/.config/zed/settings.json` | stdio | ✓ |
| **Custom Harnesses** | `mcpforge.toml` | all | User-defined |

---

## Automated CLI Commands

```bash
# 1. Audit and discover all AI clients & running processes
mcpforge discover

# 2. Automated one-command setup: auto-resolves tokens (e.g. from `gh auth token` or `.env`),
#    installs to all detected clients, and runs immediate verification
mcpforge setup github
mcpforge setup sequential-thinking --to freebuff
mcpforge setup postgres

# 3. View and install curated server packs
mcpforge pack list
mcpforge pack install dev-core       # filesystem, git, memory, fetch, sequential-thinking
mcpforge pack install data           # postgres, mysql, sqlite, mongodb, redis, memory
mcpforge pack install web-research   # brave-search, tavily, fetch, puppeteer, playwright
mcpforge pack install cloud-dev      # docker, kubernetes, aws, cloudflare, sentry, datadog
mcpforge pack install productivity   # linear, notion, slack, discord, google-drive, todoist
mcpforge pack install ai-agent       # memory, sequential-thinking, context7, time, fetch, filesystem
mcpforge pack install full-stack     # filesystem, git, github, postgres, redis, docker, fetch
mcpforge pack install enterprise     # github, gitlab, jira, slack, sentry, kubernetes

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
