use anyhow::{Context, Result};
use mcp_core::types::ServerEntry;
use std::collections::BTreeMap;

use crate::antigravity::AntigravityAdapter;
use crate::anythingllm::AnythingLlmAdapter;
use crate::claude_code::ClaudeCodeAdapter;
use crate::claude_desktop::ClaudeDesktopAdapter;
use crate::cline::ClineAdapter;
use crate::codex::CodexAdapter;
use crate::continue_dev::ContinueAdapter;
use crate::cursor::CursorAdapter;
use crate::custom::CustomHarnessAdapter;
use crate::deepseek::DeepSeekAdapter;
use crate::freebuff::FreebuffAdapter;
use crate::goose::GooseAdapter;
use crate::grok::GrokAdapter;
use crate::hermes::HermesAdapter;
use crate::jcode::JcodeAdapter;
use crate::jetbrains::JetBrainsAdapter;
use crate::letta::LettaAdapter;
use crate::librechat::LibreChatAdapter;
use crate::manicode::ManicodeAdapter;
use crate::mcphub::McpHubAdapter;
use crate::openclaw::OpenClawAdapter;
use crate::opencode::OpenCodeAdapter;
use crate::prime::PrimeAdapter;
use crate::roo_code::RooCodeAdapter;
use crate::traits::{ClientAdapter, ConfigLocation};
use crate::vscode::VsCodeAdapter;
use crate::windsurf::WindsurfAdapter;
use crate::zed::ZedAdapter;

pub struct AdapterManager {
    adapters: Vec<Box<dyn ClientAdapter>>,
}

impl Default for AdapterManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AdapterManager {
    pub fn new() -> Self {
        let adapters: Vec<Box<dyn ClientAdapter>> = vec![
            Box::new(ClaudeDesktopAdapter::new()),
            Box::new(ClaudeCodeAdapter::new()),
            Box::new(CursorAdapter::new()),
            Box::new(VsCodeAdapter::new()),
            Box::new(WindsurfAdapter::new()),
            Box::new(AntigravityAdapter::new()),
            Box::new(ClineAdapter::new()),
            Box::new(ContinueAdapter::new()),
            Box::new(ZedAdapter::new()),
            Box::new(GrokAdapter::new()),
            Box::new(JcodeAdapter::new()),
            Box::new(FreebuffAdapter::new()),
            Box::new(OpenCodeAdapter::new()),
            Box::new(CodexAdapter::new()),
            Box::new(RooCodeAdapter::new()),
            Box::new(ManicodeAdapter::new()),
            Box::new(GooseAdapter::new()),
            Box::new(LibreChatAdapter::new()),
            Box::new(McpHubAdapter::new()),
            Box::new(AnythingLlmAdapter::new()),
            Box::new(JetBrainsAdapter::new()),
            Box::new(HermesAdapter::new()),
            Box::new(OpenClawAdapter::new()),
            Box::new(DeepSeekAdapter::new()),
            Box::new(PrimeAdapter::new()),
            Box::new(LettaAdapter::new()),
            Box::new(CustomHarnessAdapter::load()),
        ];
        Self { adapters }
    }

    pub fn adapters(&self) -> &[Box<dyn ClientAdapter>] {
        &self.adapters
    }

    pub fn detect_all(&self) -> Vec<ConfigLocation> {
        let mut locs = Vec::new();
        for adapter in &self.adapters {
            locs.extend(adapter.detect());
        }
        locs
    }

    pub fn detect_existing(&self) -> Vec<ConfigLocation> {
        self.detect_all().into_iter().filter(|l| l.exists).collect()
    }

    pub fn read_all_servers(&self) -> Result<Vec<ServerEntry>> {
        let mut merged: BTreeMap<String, ServerEntry> = BTreeMap::new();

        for adapter in &self.adapters {
            for loc in adapter.detect() {
                if !loc.exists {
                    continue;
                }
                if let Ok(entries) = adapter.read_servers(&loc) {
                    for entry in entries {
                        if let Some(existing) = merged.get_mut(&entry.id) {
                            // Merge clients list
                            for c in entry.clients {
                                if !existing
                                    .clients
                                    .iter()
                                    .any(|ec| ec.config_path == c.config_path)
                                {
                                    existing.clients.push(c);
                                }
                            }
                        } else {
                            merged.insert(entry.id.clone(), entry);
                        }
                    }
                }
            }
        }

        Ok(merged.into_values().collect())
    }

    pub fn write_server_to_locations(
        &self,
        server: &ServerEntry,
        locations: &[ConfigLocation],
    ) -> Result<()> {
        for loc in locations {
            for adapter in &self.adapters {
                let locs = adapter.detect();
                if locs.iter().any(|l| l.path == loc.path) {
                    let mut existing = adapter.read_servers(loc).unwrap_or_default();
                    if let Some(idx) = existing.iter().position(|e| e.id == server.id) {
                        existing[idx] = server.clone();
                    } else {
                        existing.push(server.clone());
                    }
                    adapter
                        .write_servers(loc, &existing)
                        .with_context(|| format!("Failed to write to client {:?}", loc.path))?;
                }
            }
        }
        Ok(())
    }

    pub fn remove_server_from_locations(
        &self,
        server_id: &str,
        locations: &[ConfigLocation],
    ) -> Result<()> {
        for loc in locations {
            for adapter in &self.adapters {
                let locs = adapter.detect();
                if locs.iter().any(|l| l.path == loc.path) {
                    let mut existing = adapter.read_servers(loc).unwrap_or_default();
                    if let Some(idx) = existing.iter().position(|e| e.id == server_id) {
                        existing.remove(idx);
                        adapter.write_servers(loc, &existing).with_context(|| {
                            format!("Failed to remove server from {:?}", loc.path)
                        })?;
                    }
                }
            }
        }
        Ok(())
    }
}
