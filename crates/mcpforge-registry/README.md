# mcpforge-registry

[![Crates.io](https://img.shields.io/crates/v/mcpforge-registry.svg)](https://crates.io/crates/mcpforge-registry)
[![Docs.rs](https://docs.rs/mcpforge-registry/badge.svg)](https://docs.rs/mcpforge-registry)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Curated Catalog](https://img.shields.io/badge/Curated_Catalog-110_Audited_Servers-green.svg)](https://github.com/nordicnode/mcpforge)

Curated catalog, fuzzy search, and provenance metadata for 110+ [Model Context Protocol (MCP)](https://modelcontextprotocol.io) servers.

Part of the **[MCPForge](https://github.com/nordicnode/mcpforge)** workspace.

---

## Overview

`mcpforge-registry` provides an offline, embedded registry of over 110 vetted MCP servers spanning developer tools, cloud integrations, databases, search engines, and autonomous agent capabilities.

Key features:
- **Audited Provenance**: Upstream repository URLs, package registry coordinates (npm, PyPI, Docker), licenses, and maintainer details.
- **Fuzzy Search**: Fast, in-memory score-ranked fuzzy query engine.
- **Categorization**: Tagged into intuitive groups (Developer Tools, Data & Databases, Search & Web, Cloud, AI & Utilities).
- **Environment & Argument Schemas**: Pre-configured environment variables and recommended CLI arguments for zero-friction provisioning.
- **Catalog Auditor**: Includes `catalog-audit` binary to test registry entries and schema consistency in CI.

---

## Usage

```rust
use mcpforge_registry::{Catalog, RegistryServer};

// Access the embedded catalog
let catalog = Catalog::embedded();
println!("Loaded {} audited MCP servers", catalog.len());

// Search catalog servers
let results = catalog.search("postgres");
for server in results {
    println!("Found server: {} - {}", server.id, server.name);
    println!("Description: {}", server.description);
    println!("Command: {} {:?}", server.command, server.args);
}
```

---

## License

MIT License. See [LICENSE](../../LICENSE) for details.
