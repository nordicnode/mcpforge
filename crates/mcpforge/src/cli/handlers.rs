use crate::cli::{Commands, PackCommands};
use crate::doctor::DoctorReport;
use crate::provisioner::RuntimeCapabilities;
use crate::resolver::EnvResolver;
use crate::secrets::{is_secret_key, redact_secret};
use anyhow::{Context, Result};
use mcp_core::client::check_server_health;
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
                let all_targets: Vec<ConfigLocation> = manager
                    .detect_all()
                    .into_iter()
                    .filter(|l| l.exists)
                    .collect();
                for s in &servers {
                    let _ = manager.write_server_to_locations(s, &all_targets);
                }
            }

            let report = DoctorReport::run(&servers, timeout).await;

            if json {
                println!("{}", report.to_json()?);
            } else {
                let ok = report.print_table();
                if !ok && !fix {
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

        Commands::Verify { client, json } => {
            let verifier = SchemaVerifier::new();
            let report = if let Some(ref c) = client {
                verifier.verify_client(c)?
            } else {
                verifier.verify_all()?
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
    }

    Ok(())
}
