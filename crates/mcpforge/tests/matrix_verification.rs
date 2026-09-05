use anyhow::Result;
use mcp_core::types::{Scope, ServerEntry, Transport};
use mcpforge::matrix::MatrixVerifier;
use mcpforge_adapters::{AdapterManager, ConfigLocation, DiscoveryEngine};
use std::collections::BTreeMap;
use tempfile::tempdir;

#[test]
fn test_catalog_110_servers_integrity() {
    let verifier = MatrixVerifier::new();
    let audit = verifier.audit_catalog();

    assert_eq!(
        audit.total_servers, 110,
        "Expected exactly 110 catalog servers, got {}",
        audit.total_servers
    );
    assert_eq!(
        audit.valid_servers, 110,
        "Not all servers are valid. Errors: {:?}",
        audit.errors
    );
    assert!(
        audit.errors.is_empty(),
        "Catalog audit errors: {:?}",
        audit.errors
    );

    // Verify pack references integrity
    assert_eq!(
        audit.valid_pack_references, audit.total_pack_references,
        "Some server packs reference non-existent servers"
    );
    assert!(
        audit.total_pack_references >= 40,
        "Expected at least 40 server pack references, got {}",
        audit.total_pack_references
    );
}

#[test]
fn test_all_27_adapters_registered_and_detected() {
    let verifier = MatrixVerifier::new();
    let audit = verifier.audit_adapters();

    assert_eq!(
        audit.total_adapters, 27,
        "Expected 27 standard adapters, got {}",
        audit.total_adapters
    );
    assert_eq!(
        audit.verified_adapters, 27,
        "Not all adapters verified. Errors: {:?}",
        audit.errors
    );
    assert!(
        audit.errors.is_empty(),
        "Adapter audit errors: {:?}",
        audit.errors
    );
}

#[test]
fn test_continue_dev_env_and_sse_preservation() -> Result<()> {
    let manager = AdapterManager::new();
    let continue_adapter = manager
        .adapters()
        .iter()
        .find(|a| a.id() == "continue")
        .expect("Continue adapter must be registered");

    let temp = tempdir()?;
    let temp_file = temp.path().join("config.json");
    let loc = ConfigLocation {
        client_id: "continue".to_string(),
        display_name: "Continue.dev".to_string(),
        path: temp_file,
        scope: Scope::Global,
        exists: false,
    };

    // 1. Test stdio with environment variables
    let mut env = BTreeMap::new();
    env.insert("GITHUB_TOKEN".to_string(), "ghp_secret_12345".to_string());
    env.insert("DEBUG".to_string(), "1".to_string());

    let stdio_server = ServerEntry::new_stdio(
        "github",
        "npx",
        vec![
            "-y".to_string(),
            "@modelcontextprotocol/server-github".to_string(),
        ],
        env.clone(),
    );

    // 2. Test SSE server
    let sse_server = ServerEntry {
        id: "remote-sse".to_string(),
        transport: Transport::Sse {
            url: "https://api.example.com/sse".to_string(),
        },
        enabled: true,
        clients: Vec::new(),
        tags: Vec::new(),
        notes: None,
    };

    let write_servers = vec![stdio_server.clone(), sse_server.clone()];
    continue_adapter.write_servers(&loc, &write_servers)?;

    // Read back
    let read_servers = continue_adapter.read_servers(&loc)?;
    assert_eq!(read_servers.len(), 2, "Both servers must be read back");

    // Assert stdio server preserved env
    let read_github = read_servers
        .iter()
        .find(|s| s.id == "github")
        .expect("github server must exist");

    if let Transport::Stdio { env: read_env, .. } = &read_github.transport {
        assert_eq!(
            read_env.get("GITHUB_TOKEN").map(|s| s.as_str()),
            Some("ghp_secret_12345"),
            "GITHUB_TOKEN was lost during Continue write/read"
        );
        assert_eq!(
            read_env.get("DEBUG").map(|s| s.as_str()),
            Some("1"),
            "DEBUG env was lost during Continue write/read"
        );
    } else {
        panic!("github server transport is not Stdio");
    }

    // Assert SSE server preserved
    let read_sse = read_servers
        .iter()
        .find(|s| s.id == "remote-sse")
        .expect("remote-sse server must exist");

    if let Transport::Sse { url } = &read_sse.transport {
        assert_eq!(url, "https://api.example.com/sse");
    } else {
        panic!("remote-sse server transport is not Sse");
    }

    Ok(())
}

#[test]
fn test_discovery_engine_maps_all_27_clients() {
    // Test process name mappings for antigravity, cline, roo, continue
    let mut detected = std::collections::HashSet::new();

    // agy mappings
    DiscoveryEngine::map_comm_to_client("agy", &mut detected);
    assert!(
        detected.contains("antigravity"),
        "agy must map to antigravity"
    );
    detected.clear();

    DiscoveryEngine::map_comm_to_client("antigravity", &mut detected);
    assert!(
        detected.contains("antigravity"),
        "antigravity must map to antigravity"
    );
    detected.clear();

    // cline mapping
    DiscoveryEngine::map_comm_to_client("cline", &mut detected);
    assert!(detected.contains("cline"), "cline must map to cline");
    detected.clear();

    // roo mapping
    DiscoveryEngine::map_comm_to_client("roo", &mut detected);
    assert!(detected.contains("roo-code"), "roo must map to roo-code");
    detected.clear();

    // continue mapping
    DiscoveryEngine::map_comm_to_client("continue", &mut detected);
    assert!(
        detected.contains("continue"),
        "continue must map to continue"
    );
}

#[test]
fn test_full_27_adapters_x_110_servers_matrix() -> Result<()> {
    let verifier = MatrixVerifier::new();
    let report = verifier.run_full_matrix_audit()?;

    if !report.is_success() {
        eprintln!(
            "Matrix audit encountered {} failures:",
            report.failures.len()
        );
        for f in &report.failures {
            eprintln!(
                "  [{}] Server '{}' at '{}': {}",
                f.client_id, f.server_id, f.error_stage, f.details
            );
        }
    }

    assert_eq!(
        report.matrix_combinations_tested, 2970,
        "Expected 2970 combinations (27 adapters x 110 servers)"
    );
    assert_eq!(
        report.matrix_combinations_passed, 2970,
        "All 2970 combinations must pass without error"
    );
    assert_eq!(
        report.matrix_combinations_failed, 0,
        "Expected 0 matrix failures"
    );

    // Verify batch test
    assert_eq!(
        report.batch_all_servers_tested, 27,
        "Expected 27 batch tests"
    );
    assert_eq!(
        report.batch_all_servers_passed, 27,
        "All 27 adapters must pass loading all 110 servers at once"
    );

    assert!(report.is_success(), "Full matrix audit must succeed");
    Ok(())
}

#[test]
fn test_filtered_matrix_audit_single_adapter() -> Result<()> {
    let verifier = MatrixVerifier::new();
    let report = verifier.run_matrix_audit(Some("continue"))?;

    assert_eq!(
        report.matrix_combinations_tested, 110,
        "Expected 110 combinations for single adapter"
    );
    assert_eq!(
        report.matrix_combinations_passed, 110,
        "All 110 combinations must pass"
    );
    assert_eq!(report.matrix_combinations_failed, 0);
    assert_eq!(report.batch_all_servers_tested, 1);
    assert_eq!(report.batch_all_servers_passed, 1);
    assert!(report.is_success());

    // Test invalid client name yields error
    let invalid_res = verifier.run_matrix_audit(Some("invalid_client_xyz"));
    assert!(
        invalid_res.is_err(),
        "Invalid client ID must return an error"
    );

    Ok(())
}
