use anyhow::Result;
use chrono::NaiveDate;
use mcp_core::types::{Scope, ServerEntry, Transport};
use mcpforge_adapters::{AdapterManager, ConfigLocation};
use mcpforge_registry::{Registry, SERVER_PACKS};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::time::Instant;
use tempfile::tempdir;

pub const STANDARD_ADAPTER_IDS: &[&str] = &[
    "claude-desktop",
    "claude-code",
    "cursor",
    "vscode",
    "windsurf",
    "antigravity",
    "cline",
    "continue",
    "zed",
    "grok",
    "jcode",
    "freebuff",
    "opencode",
    "codex",
    "roo-code",
    "manicode",
    "goose",
    "librechat",
    "mcphub",
    "anythingllm",
    "jetbrains",
    "hermes",
    "openclaw",
    "deepseek",
    "prime",
    "letta",
    "pi",
];

const ADAPTER_FIXTURE_MAP: &[(&str, &str)] = &[
    ("claude-desktop", "claude_desktop.golden.json"),
    ("claude-code", "claude_code.golden.json"),
    ("cursor", "cursor.golden.json"),
    ("vscode", "vscode.golden.json"),
    ("windsurf", "windsurf.golden.json"),
    ("antigravity", "antigravity.golden.json"),
    ("cline", "cline.golden.json"),
    ("continue", "continue_dev.golden.json"),
    ("zed", "zed.golden.json"),
    ("grok", "grok.golden.toml"),
    ("jcode", "jcode.golden.json"),
    ("freebuff", "freebuff.golden.json"),
    ("opencode", "opencode.golden.jsonc"),
    ("codex", "codex.golden.toml"),
    ("roo-code", "roo_code.golden.json"),
    ("manicode", "manicode.golden.json"),
    ("goose", "goose.golden.yaml"),
    ("librechat", "librechat.golden.yaml"),
    ("mcphub", "mcphub.golden.json"),
    ("anythingllm", "anythingllm.golden.json"),
    ("jetbrains", "jetbrains.golden.json"),
    ("hermes", "hermes.golden.yaml"),
    ("openclaw", "openclaw.golden.json"),
    ("deepseek", "deepseek.golden.json"),
    ("prime", "prime.golden.json"),
    ("letta", "letta.golden.json"),
    ("pi", "pi.golden.json"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixFailure {
    pub client_id: String,
    pub server_id: String,
    pub error_stage: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogAudit {
    pub total_servers: usize,
    pub valid_servers: usize,
    pub categories_count: BTreeMap<String, usize>,
    pub commands_count: BTreeMap<String, usize>,
    pub total_pack_references: usize,
    pub valid_pack_references: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterAudit {
    pub total_adapters: usize,
    pub verified_adapters: usize,
    pub adapters_tested: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixAuditReport {
    pub catalog_audit: CatalogAudit,
    pub adapter_audit: AdapterAudit,
    pub matrix_combinations_tested: usize,
    pub matrix_combinations_passed: usize,
    pub matrix_combinations_failed: usize,
    pub batch_all_servers_tested: usize,
    pub batch_all_servers_passed: usize,
    pub failures: Vec<MatrixFailure>,
    pub elapsed_ms: u128,
}

impl MatrixAuditReport {
    pub fn is_success(&self) -> bool {
        self.matrix_combinations_failed == 0
            && self.catalog_audit.errors.is_empty()
            && self.adapter_audit.errors.is_empty()
            && self.batch_all_servers_passed == self.batch_all_servers_tested
    }
}

pub struct MatrixVerifier {
    manager: AdapterManager,
    registry: Registry,
}

impl Default for MatrixVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl MatrixVerifier {
    pub fn new() -> Self {
        Self {
            manager: AdapterManager::new(),
            registry: Registry::load().unwrap_or_default(),
        }
    }

    pub fn audit_catalog(&self) -> CatalogAudit {
        let entries = self.registry.entries();
        let total_servers = entries.len();
        let mut valid_servers = 0;
        let mut categories_count = BTreeMap::new();
        let mut commands_count = BTreeMap::new();
        let mut errors = Vec::new();

        let mut seen_ids = HashSet::new();

        for entry in entries {
            let mut entry_valid = true;

            // Check ID uniqueness & non-empty
            if entry.id.trim().is_empty() {
                errors.push("Discovered catalog entry with empty ID".to_string());
                entry_valid = false;
            } else if !seen_ids.insert(entry.id.clone()) {
                errors.push(format!("Duplicate server ID in catalog: '{}'", entry.id));
                entry_valid = false;
            }

            // Check name
            if entry.name.trim().is_empty() {
                errors.push(format!("Server '{}' has empty name", entry.id));
                entry_valid = false;
            }

            // Check description
            if entry.description.trim().is_empty() {
                errors.push(format!("Server '{}' has empty description", entry.id));
                entry_valid = false;
            }

            // Check category
            if entry.category.trim().is_empty() {
                errors.push(format!("Server '{}' has empty category", entry.id));
                entry_valid = false;
            } else {
                *categories_count
                    .entry(entry.category.to_lowercase())
                    .or_insert(0) += 1;
            }

            // Check command & args
            if entry.command.trim().is_empty() {
                errors.push(format!("Server '{}' has empty command", entry.id));
                entry_valid = false;
            } else {
                *commands_count
                    .entry(entry.command.to_lowercase())
                    .or_insert(0) += 1;
            }

            // Check provenance
            if let Some(ref url) = entry.source_url {
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    errors.push(format!(
                        "Server '{}' has invalid source_url: '{}'",
                        entry.id, url
                    ));
                    entry_valid = false;
                }
            } else {
                errors.push(format!("Server '{}' is missing source_url", entry.id));
                entry_valid = false;
            }

            if let Some(ref maintainer) = entry.maintainer {
                if maintainer.trim().is_empty() {
                    errors.push(format!("Server '{}' has empty maintainer field", entry.id));
                    entry_valid = false;
                }
            } else {
                errors.push(format!("Server '{}' is missing maintainer", entry.id));
                entry_valid = false;
            }

            if let Some(ref date_str) = entry.last_verified {
                if NaiveDate::parse_from_str(date_str, "%Y-%m-%d").is_err() {
                    errors.push(format!(
                        "Server '{}' has invalid last_verified format: '{}' (must be YYYY-MM-DD)",
                        entry.id, date_str
                    ));
                    entry_valid = false;
                }
            } else {
                errors.push(format!(
                    "Server '{}' is missing last_verified timestamp",
                    entry.id
                ));
                entry_valid = false;
            }

            if entry_valid {
                valid_servers += 1;
            }
        }

        // Verify Server Packs cross-references
        let mut total_pack_references = 0;
        let mut valid_pack_references = 0;
        for pack in SERVER_PACKS {
            for server_id in pack.server_ids {
                total_pack_references += 1;
                if seen_ids.contains(*server_id) {
                    valid_pack_references += 1;
                } else {
                    errors.push(format!(
                        "Pack '{}' references non-existent server ID '{}'",
                        pack.id, server_id
                    ));
                }
            }
        }

        CatalogAudit {
            total_servers,
            valid_servers,
            categories_count,
            commands_count,
            total_pack_references,
            valid_pack_references,
            errors,
        }
    }

    pub fn audit_adapters(&self) -> AdapterAudit {
        self.audit_adapters_filtered(None)
    }

    pub fn audit_adapters_filtered(&self, target_client: Option<&str>) -> AdapterAudit {
        let mut errors = Vec::new();
        let mut adapters_tested = Vec::new();
        let registered_adapters = self.manager.adapters();

        let expected_list: Vec<&str> = if let Some(target) = target_client {
            if !STANDARD_ADAPTER_IDS.contains(&target) {
                errors.push(format!("Unknown client adapter '{}'", target));
            }
            vec![target]
        } else {
            STANDARD_ADAPTER_IDS.to_vec()
        };

        let total_adapters = expected_list.len();
        let mut verified_adapters = 0;

        for expected_id in &expected_list {
            let found = registered_adapters.iter().find(|a| a.id() == *expected_id);
            match found {
                Some(adapter) => {
                    adapters_tested.push(adapter.id().to_string());
                    if adapter.display_name().trim().is_empty() {
                        errors.push(format!("Adapter '{}' has empty display_name", adapter.id()));
                    } else {
                        let locs = adapter.detect();
                        if locs.is_empty() {
                            errors.push(format!(
                                "Adapter '{}' returned 0 detection paths",
                                adapter.id()
                            ));
                        } else {
                            verified_adapters += 1;
                        }
                    }
                }
                None => {
                    errors.push(format!(
                        "Standard adapter '{}' is NOT registered in AdapterManager",
                        expected_id
                    ));
                }
            }
        }

        AdapterAudit {
            total_adapters,
            verified_adapters,
            adapters_tested,
            errors,
        }
    }

    pub fn run_full_matrix_audit(&self) -> Result<MatrixAuditReport> {
        self.run_matrix_audit(None)
    }

    pub fn run_matrix_audit(&self, target_client: Option<&str>) -> Result<MatrixAuditReport> {
        let start_time = Instant::now();
        let catalog_audit = self.audit_catalog();
        let adapter_audit = self.audit_adapters_filtered(target_client);

        let mut matrix_combinations_tested = 0;
        let mut matrix_combinations_passed = 0;
        let mut matrix_combinations_failed = 0;
        let mut failures = Vec::new();

        let fixtures_dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../mcpforge-adapters/tests/fixtures");

        let entries = self.registry.entries();

        let adapter_fixture_list: Vec<(&str, &str)> = if let Some(target) = target_client {
            let found = ADAPTER_FIXTURE_MAP
                .iter()
                .filter(|(id, _)| *id == target)
                .copied()
                .collect::<Vec<_>>();
            if found.is_empty() {
                anyhow::bail!(
                    "Unknown client adapter '{}'. Valid adapters: {}",
                    target,
                    STANDARD_ADAPTER_IDS.join(", ")
                );
            }
            found
        } else {
            ADAPTER_FIXTURE_MAP.to_vec()
        };

        for (adapter_id, fixture_filename) in &adapter_fixture_list {
            let adapter = match self
                .manager
                .adapters()
                .iter()
                .find(|a| a.id() == *adapter_id)
            {
                Some(a) => a,
                None => {
                    failures.push(MatrixFailure {
                        client_id: adapter_id.to_string(),
                        server_id: "ALL".to_string(),
                        error_stage: "Adapter Lookup".to_string(),
                        details: format!("Adapter '{}' not found in manager", adapter_id),
                    });
                    matrix_combinations_failed += entries.len();
                    continue;
                }
            };

            let fixture_path = fixtures_dir.join(fixture_filename);
            let initial_content = if fixture_path.exists() {
                std::fs::read_to_string(&fixture_path).unwrap_or_default()
            } else {
                String::new()
            };

            for cat_entry in entries {
                matrix_combinations_tested += 1;

                // Synthesize ServerEntry with mock credentials for required environment variables
                let mut env = BTreeMap::new();
                for req in &cat_entry.required_env {
                    env.insert(req.clone(), format!("{}_TEST_TOKEN_VALUE", req));
                }
                let test_entry = cat_entry.to_server_entry(env);

                let temp_dir = match tempdir() {
                    Ok(t) => t,
                    Err(e) => {
                        failures.push(MatrixFailure {
                            client_id: adapter_id.to_string(),
                            server_id: cat_entry.id.clone(),
                            error_stage: "Tempdir Creation".to_string(),
                            details: e.to_string(),
                        });
                        matrix_combinations_failed += 1;
                        continue;
                    }
                };

                let temp_file = temp_dir.path().join(fixture_filename);
                if !initial_content.is_empty() {
                    let _ = std::fs::write(&temp_file, &initial_content);
                }

                let loc = ConfigLocation {
                    client_id: adapter_id.to_string(),
                    display_name: format!("{} (Matrix Test)", adapter.display_name()),
                    path: temp_file.clone(),
                    scope: Scope::Global,
                    exists: temp_file.exists(),
                };

                // 1. Test Write
                if let Err(e) = adapter.write_servers(&loc, std::slice::from_ref(&test_entry)) {
                    failures.push(MatrixFailure {
                        client_id: adapter_id.to_string(),
                        server_id: cat_entry.id.clone(),
                        error_stage: "Write Servers".to_string(),
                        details: format!("{:#}", e),
                    });
                    matrix_combinations_failed += 1;
                    continue;
                }

                // 2. Test Read
                let read_servers = match adapter.read_servers(&loc) {
                    Ok(s) => s,
                    Err(e) => {
                        failures.push(MatrixFailure {
                            client_id: adapter_id.to_string(),
                            server_id: cat_entry.id.clone(),
                            error_stage: "Read Servers".to_string(),
                            details: format!("{:#}", e),
                        });
                        matrix_combinations_failed += 1;
                        continue;
                    }
                };

                // 3. Verify Server Existence
                let found = read_servers.iter().find(|s| s.id == test_entry.id);
                if let Some(server) = found {
                    let mut mismatch = false;

                    // Verify command matches
                    if let Transport::Stdio {
                        command,
                        args,
                        env: read_env,
                    } = &server.transport
                    {
                        if command != &cat_entry.command {
                            failures.push(MatrixFailure {
                                client_id: adapter_id.to_string(),
                                server_id: cat_entry.id.clone(),
                                error_stage: "Field Verification: Command".to_string(),
                                details: format!(
                                    "Expected command '{}', got '{}'",
                                    cat_entry.command, command
                                ),
                            });
                            mismatch = true;
                        }

                        if args != &cat_entry.args {
                            failures.push(MatrixFailure {
                                client_id: adapter_id.to_string(),
                                server_id: cat_entry.id.clone(),
                                error_stage: "Field Verification: Args".to_string(),
                                details: format!(
                                    "Expected args {:?}, got {:?}",
                                    cat_entry.args, args
                                ),
                            });
                            mismatch = true;
                        }

                        // Check environment variables preservation
                        for req_key in &cat_entry.required_env {
                            if !read_env.contains_key(req_key) {
                                failures.push(MatrixFailure {
                                    client_id: adapter_id.to_string(),
                                    server_id: cat_entry.id.clone(),
                                    error_stage: "Field Verification: Env Key".to_string(),
                                    details: format!(
                                        "Required env variable '{}' was lost during roundtrip",
                                        req_key
                                    ),
                                });
                                mismatch = true;
                            }
                        }
                    }

                    if mismatch {
                        matrix_combinations_failed += 1;
                    } else {
                        matrix_combinations_passed += 1;
                    }
                } else {
                    failures.push(MatrixFailure {
                        client_id: adapter_id.to_string(),
                        server_id: cat_entry.id.clone(),
                        error_stage: "Server Lookup".to_string(),
                        details: format!(
                            "Server '{}' not found in read_servers output: {:?}",
                            test_entry.id,
                            read_servers.iter().map(|s| &s.id).collect::<Vec<_>>()
                        ),
                    });
                    matrix_combinations_failed += 1;
                }
            }
        }

        // BATCH ALL-SERVERS TEST:
        // Test provisioning all 110 servers simultaneously into each of the 27 adapters!
        let mut batch_all_servers_tested = 0;
        let mut batch_all_servers_passed = 0;

        let all_110_server_entries: Vec<ServerEntry> = entries
            .iter()
            .map(|cat| {
                let mut env = BTreeMap::new();
                for req in &cat.required_env {
                    env.insert(req.clone(), format!("{}_TEST_VAL", req));
                }
                cat.to_server_entry(env)
            })
            .collect();

        for (adapter_id, fixture_filename) in &adapter_fixture_list {
            batch_all_servers_tested += 1;

            if let Some(adapter) = self
                .manager
                .adapters()
                .iter()
                .find(|a| a.id() == *adapter_id)
            {
                let temp_dir = tempdir()?;
                let temp_file = temp_dir.path().join(fixture_filename);
                let loc = ConfigLocation {
                    client_id: adapter_id.to_string(),
                    display_name: format!("{} (Batch All Servers)", adapter.display_name()),
                    path: temp_file.clone(),
                    scope: Scope::Global,
                    exists: false,
                };

                // Write all 110 servers
                if let Err(e) = adapter.write_servers(&loc, &all_110_server_entries) {
                    failures.push(MatrixFailure {
                        client_id: adapter_id.to_string(),
                        server_id: "BATCH_110_SERVERS".to_string(),
                        error_stage: "Batch Write All".to_string(),
                        details: format!("{:#}", e),
                    });
                    continue;
                }

                // Read all back
                match adapter.read_servers(&loc) {
                    Ok(read_all) => {
                        if read_all.len() == all_110_server_entries.len() {
                            batch_all_servers_passed += 1;
                        } else {
                            failures.push(MatrixFailure {
                                client_id: adapter_id.to_string(),
                                server_id: "BATCH_110_SERVERS".to_string(),
                                error_stage: "Batch Read All Count".to_string(),
                                details: format!(
                                    "Expected 110 servers, but adapter read back {}",
                                    read_all.len()
                                ),
                            });
                        }
                    }
                    Err(e) => {
                        failures.push(MatrixFailure {
                            client_id: adapter_id.to_string(),
                            server_id: "BATCH_110_SERVERS".to_string(),
                            error_stage: "Batch Read All".to_string(),
                            details: format!("{:#}", e),
                        });
                    }
                }
            }
        }

        let elapsed_ms = start_time.elapsed().as_millis();

        Ok(MatrixAuditReport {
            catalog_audit,
            adapter_audit,
            matrix_combinations_tested,
            matrix_combinations_passed,
            matrix_combinations_failed,
            batch_all_servers_tested,
            batch_all_servers_passed,
            failures,
            elapsed_ms,
        })
    }
}
