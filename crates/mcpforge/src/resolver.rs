use crate::secrets::SecretsStore;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub struct EnvResolver {
    secrets: SecretsStore,
}

impl Default for EnvResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvResolver {
    pub fn new() -> Self {
        Self {
            secrets: SecretsStore::load(),
        }
    }

    pub fn resolve_for_keys(
        &self,
        required_keys: &[String],
    ) -> (BTreeMap<String, String>, Vec<String>) {
        let mut resolved = BTreeMap::new();
        let mut missing = Vec::new();

        let dot_env = Self::scan_dot_env();

        for key in required_keys {
            if let Some(val) = self.resolve_single_key(key, &dot_env) {
                resolved.insert(key.clone(), val);
            } else {
                missing.push(key.clone());
            }
        }

        (resolved, missing)
    }

    fn resolve_single_key(&self, key: &str, dot_env: &BTreeMap<String, String>) -> Option<String> {
        // 1. Check direct process env
        if let Ok(val) = std::env::var(key) {
            if !val.trim().is_empty() {
                return Some(val);
            }
        }

        // 2. Check alias env vars (e.g. GITHUB_TOKEN vs GITHUB_PERSONAL_ACCESS_TOKEN)
        if key == "GITHUB_PERSONAL_ACCESS_TOKEN" || key == "GITHUB_TOKEN" {
            for alias in ["GITHUB_TOKEN", "GITHUB_PERSONAL_ACCESS_TOKEN", "GH_TOKEN"] {
                if let Ok(val) = std::env::var(alias) {
                    if !val.trim().is_empty() {
                        return Some(val);
                    }
                }
                if let Some(val) = dot_env.get(alias) {
                    if !val.trim().is_empty() {
                        return Some(val.clone());
                    }
                }
            }

            // Automated `gh auth token` execution
            if let Some(token) = Self::try_gh_cli_token() {
                return Some(token);
            }
        }

        // 3. Database URL aliases
        if key == "POSTGRES_URL" || key == "DATABASE_URL" {
            for alias in ["POSTGRES_URL", "DATABASE_URL"] {
                if let Ok(val) = std::env::var(alias) {
                    if !val.trim().is_empty() {
                        return Some(val);
                    }
                }
                if let Some(val) = dot_env.get(alias) {
                    if !val.trim().is_empty() {
                        return Some(val.clone());
                    }
                }
            }
        }

        // 4. Check loaded .env map
        if let Some(val) = dot_env.get(key) {
            if !val.trim().is_empty() {
                return Some(val.clone());
            }
        }

        // 5. Check secrets store (~/.local/state/mcpforge/secrets.json)
        if let Some(val) = self.secrets.get(key) {
            if !val.trim().is_empty() {
                return Some(val.clone());
            }
        }

        None
    }

    fn try_gh_cli_token() -> Option<String> {
        let output = std::process::Command::new("gh")
            .args(["auth", "token"])
            .output()
            .ok()?;

        if output.status.success() {
            let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !token.is_empty() && !token.contains("not logged in") {
                return Some(token);
            }
        }
        None
    }

    fn scan_dot_env() -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        let candidate_paths = [
            PathBuf::from(".env"),
            PathBuf::from(".env.local"),
            PathBuf::from(".env.development"),
            PathBuf::from("../.env"),
        ];

        for path in candidate_paths {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for line in content.lines() {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with('#') {
                            continue;
                        }
                        if let Some((k, v)) = line.split_once('=') {
                            let k = k.trim().to_string();
                            let v = v.trim().trim_matches('"').trim_matches('\'').to_string();
                            map.entry(k).or_insert(v);
                        }
                    }
                }
            }
        }

        map
    }
}
