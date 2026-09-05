# Contributing to MCPForge

First off, thank you for considering contributing to MCPForge! Whether you are adding a new client adapter, adding an audited MCP server to the catalog, reporting a bug, or improving the TUI, your help is warmly welcomed.

---

## Code of Conduct

We are committed to providing a friendly, safe, and welcoming environment for everyone. Please be respectful and constructive in all discussions, issues, and pull requests.

---

## Workspace Architecture

MCPForge is organized as a modular Rust Cargo workspace:

```
crates/
├── mcp-core/             # Protocol types, stdio/http transports, JSON-RPC 2.0
├── mcpforge-adapters/    # 27 client configuration adapters behind ClientAdapter trait
├── mcpforge-registry/    # 110-server curated catalog, fuzzy search, and link audit
└── mcpforge/             # Ratatui TUI app, CLI commands, secret resolver, and runner
catalog/                  # Embedded catalog database (default_registry.json)
packaging/                # Packaging formulas for Homebrew, AUR, and binary distributions
```

---

## Adding a New Client Adapter (The 4-Step Guide)

With dozens of emerging AI coding harnesses, code editors, and agent frameworks, community adapter contributions are the primary way MCPForge scales!

### Step 1: Implement the `ClientAdapter` Trait

Create a new file in `crates/mcpforge-adapters/src/<your_client>.rs`:

```rust
use crate::traits::{ClientAdapter, ConfigLocation};
use mcp_core::types::ServerEntry;
use anyhow::Result;
use std::path::Path;

pub struct MyClientAdapter;

impl ClientAdapter for MyClientAdapter {
    fn id(&self) -> &'static str {
        "my-client"
    }

    fn display_name(&self) -> &'static str {
        "My Client"
    }

    fn detect_locations(&self) -> Vec<ConfigLocation> {
        // Return detected global and project paths on Linux, macOS, and Windows
        let mut locs = Vec::new();
        if let Some(home) = dirs::home_dir() {
            let p = home.join(".my-client").join("mcp.json");
            locs.push(ConfigLocation::global("my-client", "My Client", p));
        }
        locs
    }

    fn read_servers(&self, path: &Path) -> Result<Vec<ServerEntry>> {
        crate::common::read_mcp_servers_from_json(path, "mcpServers")
    }

    fn write_server(&self, path: &Path, server: &ServerEntry) -> Result<()> {
        let mut servers = self.read_servers(path).unwrap_or_default();
        if let Some(pos) = servers.iter().position(|s| s.id == server.id) {
            servers[pos] = server.clone();
        } else {
            servers.push(server.clone());
        }
        crate::common::write_mcp_servers_to_json(path, "mcpServers", self.id(), &servers)
    }

    fn remove_server(&self, path: &Path, server_id: &str) -> Result<bool> {
        let mut servers = self.read_servers(path)?;
        let orig = servers.len();
        servers.retain(|s| s.id != server_id);
        if servers.len() != orig {
            crate::common::write_mcp_servers_to_json(path, "mcpServers", self.id(), &servers)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
```

### Step 2: Register in `AdapterManager`

In `crates/mcpforge-adapters/src/manager.rs`, add your adapter instance to `all_adapters()`:

```rust
Box::new(my_client::MyClientAdapter),
```

### Step 3: Add Golden Fixtures

Create test fixtures in `crates/mcpforge-adapters/tests/fixtures/my_client.golden.json`. Ensure the golden fixture includes both MCP server declarations and unrelated user configuration keys to verify format preservation.

### Step 4: Run Tests

Run the golden test suite:

```bash
cargo test -p mcpforge-adapters --test golden_tests
```

---

## Contributing to the Curated Catalog

To submit a new server to the curated catalog:

1. Add your server entry into `catalog/default_registry.json`.
2. Provide all provenance metadata:
   - `id`: unique kebab-case identifier
   - `name`: human-readable name
   - `category`: one of `agents`, `devtools`, `data`, `web`, `git`, `cloud`, `productivity`
   - `command` and `args`: default invocation command (e.g. `npx`, `uvx`, or binary)
   - `required_env`: list of required environment variables
   - `source_url`: official GitHub repository or documentation link
   - `last_verified`: current date (`YYYY-MM-DD`)
   - `maintainer`: upstream author or organization
3. Run catalog validation:
   ```bash
   cargo test -p mcpforge-registry
   cargo run -p mcpforge-registry --bin catalog-audit
   ```

---

## Pull Request Hygiene & Invariants

Every pull request must satisfy these automated checks:

1. **Format Check**:
   ```bash
   cargo fmt --all -- --check
   ```
2. **Clippy (Zero Warnings)**:
   ```bash
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   ```
3. **Workspace Test Suite**:
   ```bash
   cargo test --workspace --all-features
   ```
4. **Schema Drift Verification**:
   ```bash
   cargo run -- verify
   ```
5. **AST Format Preservation Invariant**:
   Modifying or removing servers must never drop unmanaged root keys or corrupt comments in user configuration files.

---

## Finding Something to Work On

Look for issues labeled with:
- [`good first issue`](https://github.com/nordicnode/mcpforge/labels/good%20first%20issue): Great for newcomers wanting to add a new client adapter or catalog entry.
- [`help wanted`](https://github.com/nordicnode/mcpforge/labels/help%20wanted): Specific improvements requested by maintainers and users.
- [`adapter`](https://github.com/nordicnode/mcpforge/labels/adapter): Requests for new client harness integrations.
