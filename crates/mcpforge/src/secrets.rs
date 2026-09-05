use anyhow::Result;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecretsStore {
    #[serde(default)]
    pub secrets: BTreeMap<String, String>,
}

#[allow(dead_code)]
impl SecretsStore {
    pub fn default_path() -> Option<PathBuf> {
        let base = if let Some(state_dir) = dirs::state_dir() {
            state_dir.join("mcpforge")
        } else {
            let home = dirs::home_dir()?;
            home.join(".local").join("state").join("mcpforge")
        };
        Some(base.join("secrets.json"))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::default_path() {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(store) = serde_json::from_str(&content) {
                        return store;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<()> {
        if let Some(path) = Self::default_path() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string_pretty(self)?;
            std::fs::write(&path, &json)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&path)?.permissions();
                perms.set_mode(0o600);
                std::fs::set_permissions(&path, perms)?;
            }
        }
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.secrets.get(key)
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) -> Result<()> {
        self.secrets.insert(key.into(), value.into());
        self.save()
    }

    pub fn remove(&mut self, key: &str) -> Result<bool> {
        let removed = self.secrets.remove(key).is_some();
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    pub fn list(&self) -> &BTreeMap<String, String> {
        &self.secrets
    }
}

pub fn redact_secret(val: &str) -> String {
    if val.is_empty() {
        return String::new();
    }
    if val.len() <= 6 {
        return "••••••••".to_string();
    }
    format!("{}••••{}", &val[..2], &val[val.len() - 2..])
}

pub fn is_secret_key(key: &str) -> bool {
    let lower = key.to_lowercase();
    lower.contains("token")
        || lower.contains("key")
        || lower.contains("secret")
        || lower.contains("password")
        || lower.contains("auth")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_secret() {
        assert_eq!(redact_secret(""), "");
        assert_eq!(redact_secret("12345"), "••••••••");
        assert_eq!(redact_secret("123456"), "••••••••");
        assert_eq!(redact_secret("ghp_1234567890abcdef"), "gh••••ef");
    }

    #[test]
    fn test_is_secret_key() {
        assert!(is_secret_key("GITHUB_TOKEN"));
        assert!(is_secret_key("API_KEY"));
        assert!(is_secret_key("CLIENT_SECRET"));
        assert!(is_secret_key("DB_PASSWORD"));
        assert!(is_secret_key("AUTH_BEARER"));
        assert!(!is_secret_key("PORT"));
        assert!(!is_secret_key("HOST"));
    }

    #[test]
    fn test_secrets_store_memory_roundtrip() {
        let mut store = SecretsStore::default();
        store.secrets.insert("FOO".to_string(), "bar".to_string());
        assert_eq!(store.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(store.list().len(), 1);

        let removed = store.secrets.remove("FOO").is_some();
        assert!(removed);
        assert_eq!(store.get("FOO"), None);
    }
}
