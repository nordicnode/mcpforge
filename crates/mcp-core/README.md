# mcpforge-core

[![Crates.io](https://img.shields.io/crates/v/mcpforge-core.svg)](https://crates.io/crates/mcpforge-core)
[![Docs.rs](https://docs.rs/mcpforge-core/badge.svg)](https://docs.rs/mcpforge-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Core protocol types, JSON-RPC 2.0 messaging primitives, and stdio/http transports for the [Model Context Protocol (MCP)](https://modelcontextprotocol.io).

Part of the **[MCPForge](https://github.com/nordicnode/mcpforge)** workspace.

---

## Overview

`mcpforge-core` defines the foundational data types and communication interfaces for interacting with Model Context Protocol servers.

Key components:
- **Protocol Types**: Canonical `ServerEntry`, `Transport` (`Stdio`, `Http`, `Sse`), `Scope` (`Global`, `Project`), and environment mapping.
- **Transports**: Asynchronous, non-blocking transports for launching and managing stdio subprocesses or querying remote HTTP/SSE endpoints.
- **Diagnostics & Health Checks**: Lightweight protocol ping/handshake execution to measure roundtrip latency and discover supported tools.
- **Robust Error Handling**: Domain-specific error types via `thiserror`.

---

## Usage

```rust
use mcp_core::types::{ServerEntry, Transport};
use std::collections::BTreeMap;

// Construct a server definition
let mut env = BTreeMap::new();
env.insert("DEBUG".to_string(), "1".to_string());

let server = ServerEntry {
    command: "npx".to_string(),
    args: vec!["-y".to_string(), "@modelcontextprotocol/server-memory".to_string()],
    env,
    disabled: false,
    transport: Transport::Stdio,
};

println!("Configured: {} with {} args", server.command, server.args.len());
```

---

## License

MIT License. See [LICENSE](../../LICENSE) for details.
