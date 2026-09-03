use anyhow::Result;
use mcp_core::types::{Scope, ServerEntry};
use mcpforge_adapters::{AdapterManager, ConfigLocation};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn fixture_path(filename: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(filename)
}

fn copy_to_temp(filename: &str, temp_dir: &Path) -> (PathBuf, ConfigLocation) {
    let src = fixture_path(filename);
    let dst = temp_dir.join(filename);
    fs::copy(&src, &dst).unwrap_or_else(|e| panic!("Failed to copy fixture {filename}: {e}"));
    let loc = ConfigLocation {
        client_id: filename.split('.').next().unwrap().to_string(),
        display_name: filename.to_string(),
        path: dst.clone(),
        scope: Scope::Global,
        exists: true,
    };
    (dst, loc)
}

fn extract_root_keys(content: &str, ext: &str) -> HashSet<String> {
    match ext {
        "json" | "jsonc" => {
            // Strip comments if jsonc
            let clean = content
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            if let Ok(serde_json::Value::Object(map)) = serde_json::from_str(&clean) {
                map.keys().cloned().collect()
            } else {
                HashSet::new()
            }
        }
        "yaml" => {
            if let Ok(serde_yaml::Value::Mapping(map)) = serde_yaml::from_str(content) {
                map.keys()
                    .filter_map(|k| k.as_str().map(|s| s.to_string()))
                    .collect()
            } else {
                HashSet::new()
            }
        }
        "toml" => {
            if let Ok(toml::Value::Table(tbl)) = toml::from_str(content) {
                tbl.keys().cloned().collect()
            } else {
                HashSet::new()
            }
        }
        _ => HashSet::new(),
    }
}

const ADAPTER_FIXTURES: &[(&str, &str)] = &[
    ("claude-code", "claude_code.golden.json"),
    ("deepseek", "deepseek.golden.json"),
    ("freebuff", "freebuff.golden.json"),
    ("goose", "goose.golden.yaml"),
    ("hermes", "hermes.golden.yaml"),
    ("codex", "codex.golden.toml"),
    ("grok", "grok.golden.toml"),
    ("opencode", "opencode.golden.jsonc"),
    ("zed", "zed.golden.json"),
    ("jetbrains", "jetbrains.golden.json"),
    ("continue", "continue_dev.golden.json"),
    ("antigravity", "antigravity.golden.json"),
    ("cursor", "cursor.golden.json"),
    ("vscode", "vscode.golden.json"),
    ("windsurf", "windsurf.golden.json"),
    ("cline", "cline.golden.json"),
    ("roo-code", "roo_code.golden.json"),
    ("jcode", "jcode.golden.json"),
    ("manicode", "manicode.golden.json"),
    ("openclaw", "openclaw.golden.json"),
    ("prime", "prime.golden.json"),
    ("letta", "letta.golden.json"),
    ("librechat", "librechat.golden.yaml"),
    ("anythingllm", "anythingllm.golden.json"),
    ("mcphub", "mcphub.golden.json"),
    ("claude-desktop", "claude_desktop.golden.json"),
];

#[test]
fn test_all_fixtures_exist() {
    for (_, fixture) in ADAPTER_FIXTURES {
        let p = fixture_path(fixture);
        assert!(p.exists(), "Fixture does not exist: {}", p.display());
    }
}

#[test]
fn test_all_adapters_read_fixtures() -> Result<()> {
    let manager = AdapterManager::new();
    let temp = tempdir()?;

    for (adapter_id, fixture) in ADAPTER_FIXTURES {
        let adapter = manager
            .adapters()
            .iter()
            .find(|a| a.id() == *adapter_id)
            .unwrap_or_else(|| panic!("Adapter not found: {adapter_id}"));

        let (_, loc) = copy_to_temp(fixture, temp.path());
        let servers = adapter.read_servers(&loc)?;

        assert!(
            !servers.is_empty(),
            "Adapter '{adapter_id}' parsed 0 servers from fixture '{fixture}'"
        );
    }

    Ok(())
}

#[test]
fn test_golden_add_server_preserves_existing_servers() -> Result<()> {
    let manager = AdapterManager::new();
    let temp = tempdir()?;

    let new_server = ServerEntry::new_stdio(
        "postgres",
        "npx",
        vec![
            "-y".to_string(),
            "@modelcontextprotocol/server-postgres".to_string(),
        ],
        BTreeMap::new(),
    );

    for (adapter_id, fixture) in ADAPTER_FIXTURES {
        let adapter = manager
            .adapters()
            .iter()
            .find(|a| a.id() == *adapter_id)
            .unwrap_or_else(|| panic!("Adapter not found: {adapter_id}"));

        let (dst, loc) = copy_to_temp(fixture, temp.path());
        let initial_servers = adapter.read_servers(&loc)?;
        let initial_ids: Vec<String> = initial_servers.iter().map(|s| s.id.clone()).collect();

        // Add postgres server
        let mut updated = initial_servers.clone();
        updated.push(new_server.clone());
        adapter.write_servers(&loc, &updated)?;

        // Re-read and assert
        let re_read = adapter.read_servers(&loc)?;
        let re_read_ids: Vec<String> = re_read.iter().map(|s| s.id.clone()).collect();

        // 1. Newly added server must exist
        assert!(
            re_read_ids.contains(&"postgres".to_string()),
            "Adapter '{adapter_id}' failed to write new server into '{dst:?}'"
        );

        // 2. All initial servers MUST STILL EXIST
        for init_id in &initial_ids {
            assert!(
                re_read_ids.contains(init_id),
                "DATA LOSS: Adapter '{adapter_id}' dropped existing server '{init_id}' when adding new server!"
            );
        }
    }

    Ok(())
}

#[test]
fn test_golden_remove_server_preserves_unrelated_servers() -> Result<()> {
    let manager = AdapterManager::new();
    let temp = tempdir()?;

    for (adapter_id, fixture) in ADAPTER_FIXTURES {
        let adapter = manager
            .adapters()
            .iter()
            .find(|a| a.id() == *adapter_id)
            .unwrap_or_else(|| panic!("Adapter not found: {adapter_id}"));

        let (dst, loc) = copy_to_temp(fixture, temp.path());
        let initial_servers = adapter.read_servers(&loc)?;

        if initial_servers.len() < 2 {
            continue;
        }

        let target_remove = initial_servers[0].id.clone();
        let should_remain = initial_servers[1].id.clone();

        // Remove first server
        let mut remaining = initial_servers.clone();
        remaining.remove(0);
        adapter.write_servers(&loc, &remaining)?;

        // Re-read
        let re_read = adapter.read_servers(&loc)?;
        let re_read_ids: Vec<String> = re_read.iter().map(|s| s.id.clone()).collect();

        assert!(
            !re_read_ids.contains(&target_remove),
            "Adapter '{adapter_id}' failed to remove server '{target_remove}' in '{dst:?}'"
        );

        assert!(
            re_read_ids.contains(&should_remain),
            "DATA LOSS: Adapter '{adapter_id}' dropped server '{should_remain}' when removing '{target_remove}'!"
        );
    }

    Ok(())
}

#[test]
fn test_property_never_drops_unrelated_root_keys() -> Result<()> {
    let manager = AdapterManager::new();
    let temp = tempdir()?;

    let new_server = ServerEntry::new_stdio(
        "telemetry_collector",
        "uvx",
        vec!["mcp-telemetry".to_string()],
        BTreeMap::new(),
    );

    for (adapter_id, fixture) in ADAPTER_FIXTURES {
        let adapter = manager
            .adapters()
            .iter()
            .find(|a| a.id() == *adapter_id)
            .unwrap_or_else(|| panic!("Adapter not found: {adapter_id}"));

        let (dst, loc) = copy_to_temp(fixture, temp.path());
        let initial_content = fs::read_to_string(&dst)?;

        let ext = dst
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("json")
            .to_string();

        let keys_before = extract_root_keys(&initial_content, &ext);
        assert!(
            !keys_before.is_empty(),
            "Fixture '{fixture}' has empty root keys before write"
        );

        // Perform write operation (add server)
        let mut servers = adapter.read_servers(&loc)?;
        servers.push(new_server.clone());
        adapter.write_servers(&loc, &servers)?;

        let after_content = fs::read_to_string(&dst)?;
        let keys_after = extract_root_keys(&after_content, &ext);

        // Core Invariant: Every unrelated top-level key must be preserved
        for k in &keys_before {
            assert!(
                keys_after.contains(k),
                "CRITICAL INVARIANT VIOLATION: Adapter '{adapter_id}' DROPPED root key '{k}' in fixture '{fixture}'!"
            );
        }
    }

    Ok(())
}

#[test]
fn test_golden_roundtrip_idempotency() -> Result<()> {
    let manager = AdapterManager::new();
    let temp = tempdir()?;

    for (adapter_id, fixture) in ADAPTER_FIXTURES {
        let adapter = manager
            .adapters()
            .iter()
            .find(|a| a.id() == *adapter_id)
            .unwrap_or_else(|| panic!("Adapter not found: {adapter_id}"));

        let (_, loc) = copy_to_temp(fixture, temp.path());
        let initial_servers = adapter.read_servers(&loc)?;

        // Re-write exact same servers without modifications
        adapter.write_servers(&loc, &initial_servers)?;

        // Re-read and assert 100% identity
        let re_read_servers = adapter.read_servers(&loc)?;
        assert_eq!(
            initial_servers.len(),
            re_read_servers.len(),
            "Idempotency violation in adapter '{adapter_id}': server count changed"
        );

        let initial_ids: Vec<String> = initial_servers.iter().map(|s| s.id.clone()).collect();
        let re_read_ids: Vec<String> = re_read_servers.iter().map(|s| s.id.clone()).collect();
        assert_eq!(
            initial_ids, re_read_ids,
            "Idempotency violation in adapter '{adapter_id}': server IDs altered"
        );
    }

    Ok(())
}
