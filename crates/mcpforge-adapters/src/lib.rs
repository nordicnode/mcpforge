pub mod antigravity;
pub mod anythingllm;
pub mod backup;
pub mod claude_code;
pub mod claude_desktop;
pub mod cline;
pub mod codex;
pub mod common;
pub mod continue_dev;
pub mod cursor;
pub mod custom;
pub mod discovery;
pub mod freebuff;
pub mod goose;
pub mod grok;
pub mod jcode;
pub mod jetbrains;
pub mod librechat;
pub mod manager;
pub mod manicode;
pub mod mcphub;
pub mod opencode;
pub mod roo_code;
pub mod traits;
pub mod vscode;
pub mod windsurf;
pub mod zed;

pub use backup::{atomic_write, compute_diff, create_backup, default_backup_dir};
pub use discovery::{DiscoveredHarness, DiscoveryEngine};
pub use manager::AdapterManager;
pub use traits::{ClientAdapter, ConfigLocation, TransportSupport};

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_core::types::{Scope, ServerEntry};
    use std::collections::BTreeMap;

    #[test]
    fn test_roundtrip_mcp_servers_json() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("cursor_mcp.json");

        let initial_json = r#"{
  "some_setting": true,
  "mcpServers": {
    "existing-server": {
      "command": "node",
      "args": ["server.js"],
      "env": {
        "PORT": "3000"
      }
    }
  }
}
"#;
        std::fs::write(&config_path, initial_json).unwrap();

        let loc = ConfigLocation {
            client_id: "cursor".to_string(),
            display_name: "Cursor".to_string(),
            path: config_path.clone(),
            scope: Scope::Global,
            exists: true,
        };

        // Read
        let servers = common::read_mcp_servers_from_json(&config_path, "mcpServers", &loc).unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].id, "existing-server");

        // Add a new server
        let mut new_servers = servers;
        let mut env = BTreeMap::new();
        env.insert("DEBUG".to_string(), "1".to_string());
        new_servers.push(ServerEntry::new_stdio(
            "github",
            "npx",
            vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-github".to_string(),
            ],
            env,
        ));

        // Write
        common::write_mcp_servers_to_json(&config_path, "mcpServers", "cursor", &new_servers)
            .unwrap();

        // Check that initial setting "some_setting" is preserved!
        let updated_content = std::fs::read_to_string(&config_path).unwrap();
        assert!(updated_content.contains("\"some_setting\": true"));
        assert!(updated_content.contains("\"existing-server\""));
        assert!(updated_content.contains("\"github\""));

        // Verify sidecar .bak was created
        let bak_path = config_path.with_extension("json.bak");
        assert!(bak_path.exists());
    }
}
