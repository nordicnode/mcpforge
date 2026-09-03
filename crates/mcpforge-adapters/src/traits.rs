use anyhow::Result;
use mcp_core::types::{Scope, ServerEntry};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigLocation {
    pub client_id: String,
    pub display_name: String,
    pub path: PathBuf,
    pub scope: Scope,
    pub exists: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportSupport {
    pub stdio: bool,
    pub http: bool,
    pub sse: bool,
}

impl TransportSupport {
    pub const fn stdio_only() -> Self {
        Self {
            stdio: true,
            http: false,
            sse: false,
        }
    }

    pub const fn all() -> Self {
        Self {
            stdio: true,
            http: true,
            sse: true,
        }
    }

    pub const fn stdio_and_http() -> Self {
        Self {
            stdio: true,
            http: true,
            sse: false,
        }
    }
}

pub trait ClientAdapter: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn detect(&self) -> Vec<ConfigLocation>;
    fn read_servers(&self, loc: &ConfigLocation) -> Result<Vec<ServerEntry>>;
    fn write_servers(&self, loc: &ConfigLocation, entries: &[ServerEntry]) -> Result<()>;
    fn supports(&self) -> TransportSupport;
    fn backup_path(&self, loc: &ConfigLocation) -> PathBuf;
}
