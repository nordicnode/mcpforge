# mcpforge-adapters

[![Crates.io](https://img.shields.io/crates/v/mcpforge-adapters.svg)](https://crates.io/crates/mcpforge-adapters)
[![Docs.rs](https://docs.rs/mcpforge-adapters/badge.svg)](https://docs.rs/mcpforge-adapters)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Supported Clients](https://img.shields.io/badge/Supported_Clients-27_Harnesses-purple.svg)](https://github.com/nordicnode/mcpforge)

Multi-client configuration adapters and format-preserving AST engines for the [Model Context Protocol (MCP)](https://modelcontextprotocol.io).

Part of the **[MCPForge](https://github.com/nordicnode/mcpforge)** workspace.

---

## Overview

Each AI assistant, code editor, and autonomous agent framework stores its MCP server definitions in distinct configuration files and formats (JSON, JSONC, YAML, and TOML).

`mcpforge-adapters` provides:
1. **27 Native Client Adapters**: Uniform read/write configuration access for 27 popular AI harnesses.
2. **Format-Preserving AST Engines**: Updates MCP server tables while strictly preserving comments (`//`, `/* */`), trailing commas, and unmanaged top-level application settings.
3. **Atomic Backups & Rollbacks**: Automatic snapshot generation before file mutations with colorized unified diff calculation.
4. **Schema Drift Verification**: Automated structural validation and cross-compatibility matrix verification across all 27 adapters and 110+ server definitions.

---

## Supported Clients (27 Harnesses)

| Category | Client / Harness | Config Path | Format | Status |
| :--- | :--- | :--- | :--- | :--- |
| **Agent** | **Freebuff Desktop & CLI** | `~/.agents/mcp.json` | JSON | Supported |
| **Agent** | **Pi Coding Agent** | `~/.pi/agent/mcp.json` | JSON | Supported |
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

## Core Traits & Usage

### 1. The `ClientAdapter` Trait

Every client implements `ClientAdapter`, exposing its configuration path discovery, read, and write operations:

```rust
use mcpforge_adapters::{AdapterManager, ClientAdapter};

let manager = AdapterManager::new();

// List all 27 registered adapters
for adapter in manager.all_adapters() {
    println!("{}: {}", adapter.id(), adapter.name());
}

// Discover detected config locations on the current machine
let detected = manager.detect_all();
for loc in detected {
    println!("Found config for {} at {:?}", loc.client_id, loc.path);
}
```

### 2. Format-Preserving Modifications

```rust
use mcpforge_adapters::AdapterManager;
use mcp_core::types::{ServerEntry, Transport};
use std::collections::BTreeMap;

let manager = AdapterManager::new();
if let Some(adapter) = manager.get_adapter("cursor") {
    let mut servers = BTreeMap::new();
    servers.insert(
        "postgres".to_string(),
        ServerEntry {
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "@modelcontextprotocol/server-postgres".to_string()],
            env: BTreeMap::new(),
            disabled: false,
            transport: Transport::Stdio,
        },
    );

    // Reads ~/.cursor/mcp.json, merges postgres into mcpServers,
    // preserves all unmanaged fields, and writes atomically with backup snapshot.
    // adapter.write_servers(&config_location, &servers)?;
}
```

### 3. Schema Drift Verification

```rust
use mcpforge_adapters::SchemaVerifier;

let verifier = SchemaVerifier::new();

// Verify all detected configurations on disk
let report = verifier.verify_all();
if report.is_healthy() {
    println!("All client configurations are valid and conformant!");
}
```

---

## License

MIT License. See [LICENSE](../../LICENSE) for details.
