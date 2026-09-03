pub mod model;
pub mod packs;
pub mod search;

use anyhow::{Context, Result};
pub use model::CatalogEntry;
pub use packs::{find_pack, ServerPack, SERVER_PACKS};
pub use search::CatalogSearch;
use std::path::PathBuf;

const EMBEDDED_CATALOG: &str = include_str!("../catalog/default_registry.json");

pub struct Registry {
    entries: Vec<CatalogEntry>,
    searcher: CatalogSearch,
}

impl Default for Registry {
    fn default() -> Self {
        Self::load().unwrap_or_else(|_| Self {
            entries: serde_json::from_str(EMBEDDED_CATALOG).unwrap_or_default(),
            searcher: CatalogSearch::new(),
        })
    }
}

impl Registry {
    pub fn local_cache_path() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("mcpforge").join("registry.json"))
    }

    pub fn load() -> Result<Self> {
        let mut entries: Vec<CatalogEntry> = serde_json::from_str(EMBEDDED_CATALOG)
            .context("Failed to parse embedded MCP registry catalog")?;

        // If a local cached version exists at ~/.local/share/mcpforge/registry.json, merge or prefer it
        if let Some(cache_path) = Self::local_cache_path() {
            if cache_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&cache_path) {
                    if let Ok(custom_entries) = serde_json::from_str::<Vec<CatalogEntry>>(&content)
                    {
                        for ce in custom_entries {
                            if let Some(idx) = entries.iter().position(|e| e.id == ce.id) {
                                entries[idx] = ce;
                            } else {
                                entries.push(ce);
                            }
                        }
                    }
                }
            }
        }

        Ok(Self {
            entries,
            searcher: CatalogSearch::new(),
        })
    }

    pub fn entries(&self) -> &[CatalogEntry] {
        &self.entries
    }

    pub fn find_by_id(&self, id: &str) -> Option<&CatalogEntry> {
        self.entries.iter().find(|e| e.id.eq_ignore_ascii_case(id))
    }

    pub fn search(&self, query: &str) -> Vec<&CatalogEntry> {
        self.searcher
            .search(&self.entries, query)
            .into_iter()
            .map(|(entry, _score)| entry)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_registry_loads_and_searches() {
        let registry = Registry::default();
        assert!(!registry.entries().is_empty());

        let fs_server = registry.find_by_id("filesystem");
        assert!(fs_server.is_some());
        assert_eq!(fs_server.unwrap().command, "npx");

        let git_results = registry.search("git");
        assert!(!git_results.is_empty());
        assert!(git_results
            .iter()
            .any(|e| e.id == "git" || e.id == "github"));
    }

    #[test]
    fn test_all_catalog_entries_have_provenance_and_valid_schemas() {
        let registry = Registry::default();
        assert_eq!(registry.entries().len(), 110);

        for entry in registry.entries() {
            assert!(!entry.id.trim().is_empty(), "Entry has empty ID");
            assert!(
                !entry.name.trim().is_empty(),
                "Entry {} has empty name",
                entry.id
            );
            assert!(
                !entry.description.trim().is_empty(),
                "Entry {} has empty description",
                entry.id
            );
            assert!(
                !entry.command.trim().is_empty(),
                "Entry {} has empty command",
                entry.id
            );
            assert!(
                entry
                    .source_url
                    .as_ref()
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(false),
                "Entry {} is missing source_url provenance",
                entry.id
            );
            assert!(
                entry
                    .last_verified
                    .as_ref()
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(false),
                "Entry {} is missing last_verified timestamp",
                entry.id
            );
            assert!(
                entry
                    .maintainer
                    .as_ref()
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(false),
                "Entry {} is missing maintainer attribution",
                entry.id
            );
        }
    }
}
