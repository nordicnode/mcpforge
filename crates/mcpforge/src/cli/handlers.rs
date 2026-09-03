use crate::cli::{BackupCommands, Commands, PackCommands};
use crate::doctor::DoctorReport;
use crate::provisioner::RuntimeCapabilities;
use crate::resolver::EnvResolver;
use crate::secrets::{is_secret_key, redact_secret};
use anyhow::{Context, Result};
use mcp_core::client::{call_server_tool, check_server_health, list_server_tools};
use mcp_core::types::{Scope, ServerEntry};
use mcpforge_adapters::{AdapterManager, ConfigLocation, DiscoveryEngine, SchemaVerifier};
use mcpforge_registry::{find_pack, Registry, SERVER_PACKS};
use std::collections::BTreeMap;
use std::io::{self, Read};

pub async fn execute(cmd: Commands) -> Result<()> {
    let manager = AdapterManager::new();
    let registry = Registry::load().unwrap_or_default();
    let resolver = EnvResolver::new();
    let runtimes = RuntimeCapabilities::detect();

    match cmd {
        Commands::Discover { json } => {
            let engine = DiscoveryEngine::new();
            let harnesses = engine.discover_all();

            if json {
                println!("{}", serde_json::to_string_pretty(&harnesses)?);
            } else {
                println!(
                    "\n{:<26} {:<14} {:<45} {:<8}",
                    "CLIENT / HARNESS", "STATUS", "CONFIG PATH", "SERVERS"
                );
                println!("{}", "-".repeat(95));
                for h in &harnesses {
                    let status = if h.is_running && h.is_installed {
                        "ACTIVE (RUNNING)"
                    } else if h.is_running {
                        "RUNNING (UNCONFIGURED)"
                    } else if h.is_installed {
                        "INSTALLED"
                    } else {
                        "AVAILABLE"
                    };

                    println!(
                        "{:<26} {:<14} {:<45} {:<8}",
                        h.display_name,
                        status,
                        h.config_path.display(),
                        h.server_count
                    );
                }
                println!();
            }
        }

        Commands::List { client, json } => {
            let mut servers = manager.read_all_servers()?;
            if let Some(ref c) = client {
                servers.retain(|s| s.clients.iter().any(|cr| cr.client_id == *c));
            }

            if json {
                println!("{}", serde_json::to_string_pretty(&servers)?);
            } else {
                println!(
                    "\n{:<22} {:<16} {:<30}",
                    "SERVER", "TRANSPORT", "INSTALLED IN"
                );
                println!("{}", "-".repeat(70));
                for s in &servers {
                    let mut installed: Vec<String> =
                        s.clients.iter().map(|c| c.display_name.clone()).collect();
                    installed.dedup();
                    println!(
                        "{:<22} {:<16} {:<30}",
                        s.id,
                        s.transport.transport_type_str(),
                        installed.join(", ")
                    );
                }
                println!();
            }
        }

        Commands::Setup { server, to } => {
            let cat_entry = registry
                .find_by_id(&server)
                .with_context(|| format!("Server '{}' not found in registry catalog", server))?;

            println!(
                "Setting up MCP server '{}' ({})",
                cat_entry.name, cat_entry.id
            );

            // 1. Validate runtime capability
            if let Err(e) = runtimes.validate_command(&cat_entry.command) {
                eprintln!("[Warning] Runtime check: {}", e);
            }

            // 2. Auto-resolve environment variables and secrets
            let (resolved_env, missing) = resolver.resolve_for_keys(&cat_entry.required_env);
            for k in resolved_env.keys() {
                println!("  ✓ Auto-resolved environment secret '{}'", k);
            }
            if !missing.is_empty() {
                eprintln!(
                    "  [Note] Required env var(s) {:?} not found in environment, .env, or gh CLI.",
                    missing
                );
            }

            // 3. Target clients
            let all_locations = manager.detect_all();
            let targets: Vec<ConfigLocation> = if let Some(ref client_ids) = to {
                all_locations
                    .into_iter()
                    .filter(|l| client_ids.contains(&l.client_id))
                    .collect()
            } else {
                all_locations.into_iter().filter(|l| l.exists).collect()
            };

            if targets.is_empty() {
                eprintln!("Error: No installed or detected client configs found to target.");
                std::process::exit(1);
            }

            let server_entry = cat_entry.to_server_entry(resolved_env);
            manager.write_server_to_locations(&server_entry, &targets)?;
            println!(
                "  ✓ Installed to {} client(s): {}",
                targets.len(),
                targets
                    .iter()
                    .map(|t| t.display_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            // 4. Immediate health check
            print!("  Running diagnostic health check... ");
            let status = check_server_health(&server_entry, 10).await;
            println!("{}", status.status_text());
        }

        Commands::Pack { command } => match command {
            PackCommands::List => {
                println!("\n{:<16} {:<24} {:<30}", "PACK ID", "NAME", "SERVERS");
                println!("{}", "-".repeat(70));
                for p in SERVER_PACKS {
                    println!(
                        "{:<16} {:<24} {:<30}",
                        p.id,
                        p.name,
                        p.server_ids.join(", ")
                    );
                    println!("   └─ {}", p.description);
                }
                println!();
            }
            PackCommands::Install { name, to } => {
                let pack = find_pack(&name).with_context(|| {
                    format!(
                        "Pack '{}' not found. Run 'mcpforge pack list' to view available packs.",
                        name
                    )
                })?;

                println!("\nInstalling server pack '{}' ({})", pack.name, pack.id);
                for server_id in pack.server_ids {
                    if let Some(cat_entry) = registry.find_by_id(server_id) {
                        let (resolved_env, _) = resolver.resolve_for_keys(&cat_entry.required_env);
                        let server_entry = cat_entry.to_server_entry(resolved_env);

                        let all_locations = manager.detect_all();
                        let targets: Vec<ConfigLocation> = if let Some(ref client_ids) = to {
                            all_locations
                                .into_iter()
                                .filter(|l| client_ids.contains(&l.client_id))
                                .collect()
                        } else {
                            all_locations.into_iter().filter(|l| l.exists).collect()
                        };

                        let _ = manager.write_server_to_locations(&server_entry, &targets);
                        println!(
                            "  ✓ Installed '{}' to {} client(s)",
                            server_id,
                            targets.len()
                        );
                    }
                }
                println!("Pack installation complete!\n");
            }
        },

        Commands::Doctor { fix, json, timeout } => {
            let mut servers = manager.read_all_servers()?;
            println!(
                "Running doctor checks on {} configured servers...",
                servers.len()
            );

            if fix {
                println!("Auto-healing enabled: attempting resolution of missing environment variables...");
                for server in &mut servers {
                    if let mcp_core::types::Transport::Stdio { env, .. } = &mut server.transport {
                        if let Some(cat_entry) = registry.find_by_id(&server.id) {
                            let (resolved, _) = resolver.resolve_for_keys(&cat_entry.required_env);
                            for (k, v) in resolved {
                                env.entry(k).or_insert(v);
                            }
                        }
                    }
                }
                let all_locations = manager.detect_all();
                for s in &servers {
                    // CRITICAL INVARIANT: Only write back to locations where THIS server was already installed
                    let target_locs: Vec<ConfigLocation> = all_locations
                        .iter()
                        .filter(|l| s.clients.iter().any(|c| c.config_path == l.path))
                        .cloned()
                        .collect();

                    if !target_locs.is_empty() {
                        let _ = manager.write_server_to_locations(s, &target_locs);
                    }
                }
            }

            let report = DoctorReport::run(&servers, timeout).await;

            if json {
                println!("{}", report.to_json()?);
            } else {
                let ok = report.print_table();
                if fix {
                    let healed = report.auto_heal(&manager, &servers)?;
                    if healed > 0 {
                        println!("✓ Self-healing complete: {} issue(s) resolved.\n", healed);
                    } else {
                        println!("No auto-fixable issues detected.\n");
                    }
                } else if !ok {
                    std::process::exit(1);
                }
            }
        }

        Commands::Add {
            name,
            from_registry,
            stdin,
            command,
            args,
            to,
        } => {
            let all_locations = manager.detect_all();
            let targets: Vec<ConfigLocation> = if to.is_empty() {
                all_locations.into_iter().filter(|l| l.exists).collect()
            } else {
                all_locations
                    .into_iter()
                    .filter(|l| to.contains(&l.client_id))
                    .collect()
            };

            if targets.is_empty() {
                eprintln!("Error: No matching client targets found.");
                std::process::exit(1);
            }

            let server: ServerEntry = if from_registry {
                let id = name.context("Server name required when using --from-registry")?;
                let cat_entry = registry
                    .find_by_id(&id)
                    .with_context(|| format!("Server '{}' not found in registry", id))?;
                let (env, _) = resolver.resolve_for_keys(&cat_entry.required_env);
                cat_entry.to_server_entry(env)
            } else if stdin {
                let mut buf = String::new();
                io::stdin().read_to_string(&mut buf)?;
                let id = name.unwrap_or_else(|| "custom-server".to_string());
                let val: serde_json::Value = serde_json::from_str(&buf)?;
                let obj = val.as_object().context("Expected JSON object from stdin")?;
                let cmd = obj
                    .get("command")
                    .and_then(|c| c.as_str())
                    .context("Missing 'command'")?;
                let s_args = obj
                    .get("args")
                    .and_then(|a| a.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                ServerEntry::new_stdio(id, cmd, s_args, BTreeMap::new())
            } else {
                let id = name.context("Server name required")?;
                let cmd = command.context("--command is required")?;
                ServerEntry::new_stdio(id, cmd, args, BTreeMap::new())
            };

            manager.write_server_to_locations(&server, &targets)?;
            println!(
                "Successfully added server '{}' to {} client(s).",
                server.id,
                targets.len()
            );
        }

        Commands::Remove { server, from, all } => {
            let all_servers = manager.read_all_servers()?;
            let existing = all_servers.iter().find(|s| s.id == server);
            if existing.is_none() {
                eprintln!(
                    "Error: Server '{}' is not configured in any client.",
                    server
                );
                std::process::exit(1);
            }

            let all_locations = manager.detect_all();
            let targets: Vec<ConfigLocation> = if let Some(client_ids) = from {
                all_locations
                    .into_iter()
                    .filter(|l| client_ids.contains(&l.client_id) && l.exists)
                    .collect()
            } else if all {
                all_locations.into_iter().filter(|l| l.exists).collect()
            } else {
                let server_entry = existing.unwrap();
                all_locations
                    .into_iter()
                    .filter(|l| server_entry.clients.iter().any(|c| c.config_path == l.path))
                    .collect()
            };

            if targets.is_empty() {
                eprintln!("Error: No matching client configurations found to remove from.");
                std::process::exit(1);
            }

            println!(
                "Removing server '{}' from {} client(s)...",
                server,
                targets.len()
            );
            manager.remove_server_from_locations(&server, &targets)?;
            for t in &targets {
                println!("  ✓ Removed from {} ({})", t.display_name, t.path.display());
            }
            println!("Removal complete.\n");
        }

        Commands::Sync { auto, target, from } => {
            if auto {
                let all_servers = manager.read_all_servers()?;
                let running_clients = DiscoveryEngine::scan_running_processes();
                let all_targets: Vec<ConfigLocation> = manager
                    .detect_all()
                    .into_iter()
                    .filter(|l| {
                        l.exists
                            || (l.scope == Scope::Global
                                && (running_clients.contains(&l.client_id)
                                    || DiscoveryEngine::is_client_installed(&l.client_id)))
                    })
                    .collect();

                for s in &all_servers {
                    manager.write_server_to_locations(s, &all_targets)?;
                }

                println!(
                    "Auto-synced {} servers across {} client(s).",
                    all_servers.len(),
                    all_targets.len()
                );
                return Ok(());
            }

            let src_name = from.context("Source client (--from) or --auto is required")?;
            let tgt_name = target.context("Target client or --auto is required")?;

            let all_locs = manager.detect_all();
            let src_loc = all_locs
                .iter()
                .find(|l| l.client_id == src_name)
                .with_context(|| format!("Source client '{}' not found", src_name))?;
            let tgt_loc = all_locs
                .iter()
                .find(|l| l.client_id == tgt_name)
                .with_context(|| format!("Target client '{}' not found", tgt_name))?;

            let mut src_servers = Vec::new();
            for adapter in manager.adapters() {
                if adapter.id() == src_name {
                    src_servers = adapter.read_servers(src_loc)?;
                    break;
                }
            }

            for s in &src_servers {
                manager.write_server_to_locations(s, std::slice::from_ref(tgt_loc))?;
            }

            println!(
                "Synced {} servers from '{}' to '{}'.",
                src_servers.len(),
                src_name,
                tgt_name
            );
        }

        Commands::Export {
            output,
            include_secrets,
        } => {
            let mut servers = manager.read_all_servers()?;
            if !include_secrets {
                for s in &mut servers {
                    if let mcp_core::types::Transport::Stdio { env, .. } = &mut s.transport {
                        for (k, v) in env.iter_mut() {
                            if is_secret_key(k) {
                                *v = redact_secret(v);
                            }
                        }
                    }
                }
            }

            let json = serde_json::to_string_pretty(&servers)?;
            if let Some(path) = output {
                std::fs::write(&path, json)?;
                println!("Exported {} servers to {:?}", servers.len(), path);
            } else {
                println!("{}", json);
            }
        }

        Commands::Import { input, to } => {
            let content = std::fs::read_to_string(&input)?;
            let servers: Vec<ServerEntry> = serde_json::from_str(&content)?;

            let all_locations = manager.detect_all();
            let targets: Vec<ConfigLocation> = if let Some(ref client_ids) = to {
                all_locations
                    .into_iter()
                    .filter(|l| client_ids.contains(&l.client_id))
                    .collect()
            } else {
                all_locations.into_iter().filter(|l| l.exists).collect()
            };

            for s in &servers {
                manager.write_server_to_locations(s, &targets)?;
            }

            println!(
                "Imported {} servers into {} client(s).",
                servers.len(),
                targets.len()
            );
        }

        Commands::Verify { client, all, json } => {
            let verifier = SchemaVerifier::new();
            let report = if let Some(ref c) = client {
                verifier.verify_client(c)?
            } else {
                verifier.verify_all(all)?
            };

            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("\nCLIENT CONFIGURATION SCHEMA AUDIT REPORT");
                println!(
                    "{:<28} {:<8} {:<10} {:<12} {:<30}",
                    "CLIENT / HARNESS", "FORMAT", "SYNTAX", "SCHEMA", "STATUS"
                );
                println!("{}", "-".repeat(90));
                for r in &report.results {
                    let syntax_str = if r.syntax_valid {
                        "VALID"
                    } else {
                        "SYNTAX ERR"
                    };
                    let schema_str = if r.schema_compliant {
                        "COMPLIANT"
                    } else {
                        "DRIFT"
                    };
                    let status_str = if r.schema_compliant {
                        format!("{} servers configured", r.server_count)
                    } else {
                        r.errors
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "Schema mismatch".to_string())
                    };
                    println!(
                        "{:<28} {:<8} {:<10} {:<12} {:<30}",
                        r.display_name,
                        r.format.to_uppercase(),
                        syntax_str,
                        schema_str,
                        status_str
                    );
                }
                println!("{}", "-".repeat(90));
                println!(
                    "Audit summary: {} checked, {} compliant, {} drift(s) detected.\n",
                    report.total_checked, report.compliant_count, report.drift_detected_count
                );
            }

            if !report.is_all_compliant() {
                std::process::exit(1);
            }
        }

        Commands::Test {
            server,
            command,
            args,
            timeout,
        } => {
            let test_entry = if let Some(cmd) = command {
                println!("Testing direct command: {} {:?}", cmd, args);
                ServerEntry::new_stdio("direct-test", &cmd, args, BTreeMap::new())
            } else if let Some(id) = server {
                let configured = manager.read_all_servers()?;
                if let Some(entry) = configured.into_iter().find(|s| s.id == id) {
                    println!("Testing configured server '{}'...", id);
                    entry
                } else if let Some(cat_entry) = registry.find_by_id(&id) {
                    println!("Testing registry server '{}'...", id);
                    let (env, missing) = resolver.resolve_for_keys(&cat_entry.required_env);
                    if !missing.is_empty() {
                        eprintln!(
                            "Warning: Missing required environment variables: {}",
                            missing.join(", ")
                        );
                    }
                    cat_entry.to_server_entry(env)
                } else {
                    anyhow::bail!("Server '{}' not found in configured servers or catalog", id);
                }
            } else {
                anyhow::bail!("Specify either a server name (e.g. 'mcpforge test fetch') or a direct command (--command <cmd>)");
            };

            let status = check_server_health(&test_entry, timeout).await;
            match status {
                mcp_core::types::HealthStatus::Healthy {
                    latency_ms,
                    tool_count,
                    server_name,
                    server_version,
                } => {
                    println!("\n● OK: Server Handshake Succeeded");
                    println!("  Server Info:   {} v{}", server_name, server_version);
                    println!("  Tools Exposed: {}", tool_count);
                    println!("  Roundtrip:     {}ms", latency_ms);
                    println!("Status: OPERATIONAL\n");
                }
                mcp_core::types::HealthStatus::Degraded { reason, latency_ms } => {
                    let ms_str = latency_ms.map_or(String::new(), |m| format!(" ({}ms)", m));
                    println!("\n▲ WARN: Degraded Performance: {}{}\n", reason, ms_str);
                }
                mcp_core::types::HealthStatus::Broken { error } => {
                    eprintln!("\n✖ FAIL: Handshake Failed");
                    eprintln!("  Diagnostic Error: {}\n", error);
                    std::process::exit(1);
                }
                mcp_core::types::HealthStatus::Disabled => {
                    println!("\n○ Server is currently disabled.\n");
                }
                mcp_core::types::HealthStatus::Unknown => {
                    println!("\n? Server status unknown.\n");
                }
            }
        }

        Commands::Rollback { client } => {
            let all_locations = manager.detect_all();

            let (target_loc, backup) = if let Some(client_id) = client {
                let loc = all_locations
                    .into_iter()
                    .find(|l| l.client_id == client_id)
                    .with_context(|| format!("Unknown client identifier '{}'", client_id))?;
                let b = mcpforge_adapters::find_latest_backup_for_client(&client_id)?
                    .with_context(|| {
                        format!("No backup snapshots found for client '{}'", client_id)
                    })?;
                (loc, b)
            } else {
                let backups = mcpforge_adapters::list_backups()?;
                let b = backups
                    .into_iter()
                    .next()
                    .context("No configuration backups found on system")?;
                let loc = all_locations
                    .into_iter()
                    .find(|l| l.client_id == b.client_id)
                    .with_context(|| format!("Target client '{}' not found", b.client_id))?;
                (loc, b)
            };

            println!(
                "Rolling back {} configuration from snapshot {:?} ({}) ...",
                target_loc.display_name, backup.backup_path, backup.timestamp
            );

            mcpforge_adapters::restore_backup(&backup.backup_path, &target_loc.path)?;
            println!(
                "✓ Restored {} configuration to {:?}\n",
                target_loc.display_name, target_loc.path
            );
        }

        Commands::Backup { command } => match command {
            BackupCommands::List { json } => {
                let backups = mcpforge_adapters::list_backups()?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&backups)?);
                } else {
                    println!("\nCONFIGURATION BACKUP SNAPSHOTS");
                    println!(
                        "{:<18} {:<24} {:<20} {:<10} {:<30}",
                        "CLIENT", "TIMESTAMP", "FILE", "SIZE", "SNAPSHOT PATH"
                    );
                    println!("{}", "-".repeat(105));
                    for b in &backups {
                        let size_str = format!("{} B", b.size_bytes);
                        println!(
                            "{:<18} {:<24} {:<20} {:<10} {:<30}",
                            b.client_id,
                            b.timestamp,
                            b.original_file,
                            size_str,
                            b.backup_path.display()
                        );
                    }
                    println!("{}", "-".repeat(105));
                    println!("Total snapshots found: {}\n", backups.len());
                }
            }

            BackupCommands::Diff { target } => {
                let all_locations = manager.detect_all();
                let backup = if std::path::Path::new(&target).is_file() {
                    let path = std::path::PathBuf::from(&target);
                    mcpforge_adapters::list_backups()?
                        .into_iter()
                        .find(|b| b.backup_path == path)
                        .with_context(|| format!("Backup file '{}' not recognized", target))?
                } else {
                    mcpforge_adapters::find_latest_backup_for_client(&target)?
                        .with_context(|| format!("No backup found for client '{}'", target))?
                };

                let loc = all_locations
                    .into_iter()
                    .find(|l| l.client_id == backup.client_id)
                    .with_context(|| {
                        format!("No config location found for client '{}'", backup.client_id)
                    })?;

                let backup_content = std::fs::read_to_string(&backup.backup_path)?;
                let current_content = if loc.path.exists() {
                    std::fs::read_to_string(&loc.path)?
                } else {
                    String::new()
                };

                let diff = mcpforge_adapters::compute_diff(
                    &current_content,
                    &backup_content,
                    &loc.path.file_name().unwrap_or_default().to_string_lossy(),
                );

                if diff.trim().is_empty() {
                    println!(
                        "Current config is identical to backup snapshot ({})",
                        backup.timestamp
                    );
                } else {
                    println!("\nDiff (Current -> Backup Snapshot {}):", backup.timestamp);
                    println!("{}", diff);
                }
            }

            BackupCommands::Restore {
                backup_file,
                target,
            } => {
                let target_path = match target {
                    Some(p) => p,
                    None => {
                        let backups = mcpforge_adapters::list_backups()?;
                        let b = backups
                            .into_iter()
                            .find(|b| b.backup_path == backup_file)
                            .with_context(|| format!("Cannot infer target: backup file {:?} not found in index. Please specify --target", backup_file))?;
                        let all_locations = manager.detect_all();
                        let loc = all_locations
                            .into_iter()
                            .find(|l| l.client_id == b.client_id)
                            .with_context(|| {
                                format!("Cannot find client config location for '{}'", b.client_id)
                            })?;
                        loc.path
                    }
                };

                println!("Restoring {:?} to {:?}...", backup_file, target_path);
                mcpforge_adapters::restore_backup(&backup_file, &target_path)?;
                println!("✓ Restored successfully!\n");
            }
        },

        Commands::Tools {
            server,
            json,
            timeout,
        } => {
            let server_entry = resolve_server_or_catalog(&server, &manager, &registry, &resolver)?;
            if !json {
                println!(
                    "Connecting to '{}' to query exposed tools...",
                    server_entry.id
                );
            }
            let tools = list_server_tools(&server_entry, timeout).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&tools)?);
            } else {
                println!("\n{:<25} {:<55}", "TOOL NAME", "DESCRIPTION");
                println!("{}", "-".repeat(80));
                for t in &tools {
                    let desc = t.description.as_deref().unwrap_or("-");
                    println!("{:<25} {:<55}", t.name, desc);
                }
                println!("{}", "-".repeat(80));
                println!("Total tools exposed: {}\n", tools.len());
            }
        }

        Commands::Call {
            server,
            tool,
            params,
            json,
            timeout,
        } => {
            let server_entry = resolve_server_or_catalog(&server, &manager, &registry, &resolver)?;
            let parsed_params: serde_json::Value = serde_json::from_str(&params)
                .with_context(|| format!("Invalid JSON arguments: '{}'", params))?;

            if !json {
                println!("Executing tool '{}/{}'...", server_entry.id, tool);
            }
            let start = std::time::Instant::now();
            let result = call_server_tool(&server_entry, &tool, parsed_params, timeout).await?;
            let elapsed_ms = start.elapsed().as_millis();

            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let status_str = if result.is_error { "ERROR" } else { "SUCCESS" };
                println!(
                    "\nTOOL EXECUTION RESULT ({} - {}ms)",
                    status_str, elapsed_ms
                );
                println!("{}", "-".repeat(60));
                for c in &result.content {
                    if let Some(ref text) = c.text {
                        println!("{}", text);
                    } else if let Some(ref data) = c.data {
                        println!("[Binary data: {} bytes]", data.len());
                    }
                }
                println!();
            }
        }

        Commands::Completions { shell } => {
            use clap::CommandFactory;
            let mut cmd = crate::cli::Cli::command();
            clap_complete::generate(shell, &mut cmd, "mcpforge", &mut io::stdout());
        }

        Commands::Watch { sync, interval } => {
            println!("\nMCPFORGE CONFIGURATION WATCHER DAEMON ACTIVE");
            println!(
                "Monitoring 26 client harnesses across system (polling every {}s)...",
                interval
            );
            println!("Real-time syntax validation, automatic snapshotting, and corruption defense active.");
            if sync {
                println!("Auto-sync mode: ON (newly added servers will be mirrored across active clients).");
            }
            println!("Press Ctrl+C to stop.\n");

            let mut mtimes: BTreeMap<std::path::PathBuf, std::time::SystemTime> = BTreeMap::new();
            for loc in manager.detect_all() {
                if let Ok(meta) = std::fs::metadata(&loc.path) {
                    if let Ok(mtime) = meta.modified() {
                        mtimes.insert(loc.path, mtime);
                    }
                }
            }

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                for loc in manager.detect_all() {
                    if let Ok(meta) = std::fs::metadata(&loc.path) {
                        if let Ok(mtime) = meta.modified() {
                            let was_changed =
                                mtimes.get(&loc.path).is_some_and(|old| *old != mtime);
                            if was_changed {
                                mtimes.insert(loc.path.clone(), mtime);
                                let now = chrono::Local::now().format("%H:%M:%S");
                                println!(
                                    "[{}] External modification detected: {}",
                                    now, loc.display_name
                                );

                                // 1. Verify syntax
                                if let Ok(content) = std::fs::read_to_string(&loc.path) {
                                    let ext = loc
                                        .path
                                        .extension()
                                        .and_then(|e| e.to_str())
                                        .unwrap_or("json");
                                    let is_valid = match ext {
                                        "json" | "jsonc" => {
                                            let clean =
                                                mcpforge_adapters::common::strip_jsonc_comments(
                                                    &content,
                                                );
                                            serde_json::from_str::<serde_json::Value>(&clean)
                                                .is_ok()
                                        }
                                        "yaml" | "yml" => {
                                            serde_yaml::from_str::<serde_yaml::Value>(&content)
                                                .is_ok()
                                        }
                                        "toml" => toml::from_str::<toml::Value>(&content).is_ok(),
                                        _ => true,
                                    };

                                    if !is_valid {
                                        eprintln!("  ▲ [SYNTAX ERROR] Corrupted {} detected! Run 'mcpforge rollback --client {}' to restore.", ext.to_uppercase(), loc.client_id);
                                    } else {
                                        println!(
                                            "  ● [SYNTAX VALID] Configuration passed AST checks."
                                        );
                                        if let Ok(Some(backup_path)) =
                                            mcpforge_adapters::create_backup(
                                                &loc.path,
                                                &loc.client_id,
                                            )
                                        {
                                            println!(
                                                "  ✓ [SNAPSHOT] Automatically captured snapshot: {:?}",
                                                backup_path.file_name().unwrap_or_default()
                                            );
                                        }

                                        if sync {
                                            if let Ok(servers) = manager.read_all_servers() {
                                                let other_targets: Vec<_> = manager
                                                    .detect_existing()
                                                    .into_iter()
                                                    .filter(|l| l.client_id != loc.client_id)
                                                    .collect();
                                                for s in &servers {
                                                    let _ = manager.write_server_to_locations(
                                                        s,
                                                        &other_targets,
                                                    );
                                                }
                                                println!(
                                                    "  ↺ [SYNCED] Replicated across {} client(s).",
                                                    other_targets.len()
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn resolve_server_or_catalog(
    id: &str,
    manager: &AdapterManager,
    registry: &Registry,
    resolver: &EnvResolver,
) -> Result<ServerEntry> {
    let servers = manager.read_all_servers()?;
    if let Some(mut s) = servers.into_iter().find(|s| s.id == id) {
        resolver.enrich_server_entry(&mut s, registry);
        return Ok(s);
    }
    if let Some(cat) = registry.find_by_id(id) {
        let (env, _) = resolver.resolve_for_keys(&cat.required_env);
        return Ok(cat.to_server_entry(env));
    }
    anyhow::bail!(
        "Server '{}' not found in active configurations or catalog",
        id
    )
}
