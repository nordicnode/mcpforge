use crate::traits::ConfigLocation;
use crate::AdapterManager;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVerificationResult {
    pub client_id: String,
    pub display_name: String,
    pub path: String,
    pub format: String,
    pub is_installed: bool,
    pub syntax_valid: bool,
    pub schema_compliant: bool,
    pub server_count: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VerificationReport {
    pub total_checked: usize,
    pub compliant_count: usize,
    pub drift_detected_count: usize,
    pub results: Vec<SchemaVerificationResult>,
}

impl VerificationReport {
    pub fn is_all_compliant(&self) -> bool {
        self.drift_detected_count == 0
    }
}

pub struct SchemaVerifier {
    manager: AdapterManager,
}

impl Default for SchemaVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaVerifier {
    pub fn new() -> Self {
        Self {
            manager: AdapterManager::new(),
        }
    }

    pub fn verify_all(&self, include_uninstalled: bool) -> Result<VerificationReport> {
        let mut report = VerificationReport::default();
        let fixtures_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures");

        for adapter in self.manager.adapters() {
            let locations = adapter.detect();
            let mut installed_found = false;
            for loc in &locations {
                if loc.path.exists() {
                    installed_found = true;
                    let res = self.verify_location(adapter.id(), adapter.display_name(), loc);
                    report.total_checked += 1;
                    if res.schema_compliant {
                        report.compliant_count += 1;
                    } else {
                        report.drift_detected_count += 1;
                    }
                    report.results.push(res);
                }
            }

            if !installed_found && include_uninstalled {
                let id_clean = adapter.id().replace('-', "_");
                let candidate_names = [
                    format!("{}.golden.json", id_clean),
                    format!("{}.golden.jsonc", id_clean),
                    format!("{}.golden.yaml", id_clean),
                    format!("{}.golden.toml", id_clean),
                    format!("{}_dev.golden.json", id_clean),
                ];

                for c_name in candidate_names {
                    let fix_path = fixtures_dir.join(&c_name);
                    if fix_path.exists() {
                        let loc = ConfigLocation {
                            client_id: adapter.id().to_string(),
                            display_name: format!("{} (Fixture)", adapter.display_name()),
                            path: fix_path,
                            scope: mcp_core::types::Scope::Global,
                            exists: true,
                        };
                        let res = self.verify_location(adapter.id(), &loc.display_name, &loc);
                        report.total_checked += 1;
                        if res.schema_compliant {
                            report.compliant_count += 1;
                        } else {
                            report.drift_detected_count += 1;
                        }
                        report.results.push(res);
                        break;
                    }
                }
            }
        }

        Ok(report)
    }

    pub fn verify_client(&self, target_client: &str) -> Result<VerificationReport> {
        let mut report = VerificationReport::default();

        for adapter in self.manager.adapters() {
            if !adapter.id().eq_ignore_ascii_case(target_client) {
                continue;
            }
            let locations = adapter.detect();
            for loc in locations {
                let res = self.verify_location(adapter.id(), adapter.display_name(), &loc);
                report.total_checked += 1;
                if res.schema_compliant {
                    report.compliant_count += 1;
                } else {
                    report.drift_detected_count += 1;
                }
                report.results.push(res);
            }
        }

        Ok(report)
    }

    fn verify_location(
        &self,
        client_id: &str,
        display_name: &str,
        loc: &ConfigLocation,
    ) -> SchemaVerificationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let is_installed = loc.path.exists();

        if !is_installed {
            return SchemaVerificationResult {
                client_id: client_id.to_string(),
                display_name: display_name.to_string(),
                path: loc.path.display().to_string(),
                format: "unknown".to_string(),
                is_installed: false,
                syntax_valid: true,
                schema_compliant: true,
                server_count: 0,
                errors,
                warnings,
            };
        }

        let content = match std::fs::read_to_string(&loc.path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("Failed to read file: {e}"));
                return SchemaVerificationResult {
                    client_id: client_id.to_string(),
                    display_name: display_name.to_string(),
                    path: loc.path.display().to_string(),
                    format: "unknown".to_string(),
                    is_installed: true,
                    syntax_valid: false,
                    schema_compliant: false,
                    server_count: 0,
                    errors,
                    warnings,
                };
            }
        };

        let ext = loc
            .path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("json")
            .to_lowercase();

        let mut syntax_valid = true;
        let mut schema_compliant = true;
        let mut server_count = 0;

        match ext.as_str() {
            "json" | "jsonc" => {
                let clean = crate::common::strip_jsonc_comments(&content);
                match serde_json::from_str::<serde_json::Value>(&clean) {
                    Ok(val) => {
                        if !val.is_object() {
                            errors.push("Root JSON must be an object".to_string());
                            schema_compliant = false;
                        } else {
                            // Check adapter-specific root keys
                            let root = val.as_object().unwrap();
                            let expected_key = match client_id {
                                "zed" => "context_servers",
                                "opencode" => "mcp",
                                "continue" => "experimental",
                                _ => "mcpServers",
                            };

                            if client_id == "continue" {
                                if let Some(exp) =
                                    root.get("experimental").and_then(|e| e.as_object())
                                {
                                    if let Some(arr) = exp
                                        .get("modelContextProtocolServers")
                                        .and_then(|a| a.as_array())
                                    {
                                        server_count = arr.len();
                                        for (idx, s_cfg) in arr.iter().enumerate() {
                                            if !s_cfg.is_object() {
                                                errors.push(format!(
                                                    "Server at index {idx} must be an object"
                                                ));
                                                schema_compliant = false;
                                            }
                                        }
                                    }
                                }
                            } else if let Some(servers) = root.get(expected_key) {
                                if let Some(map) = servers.as_object() {
                                    server_count = map.len();
                                    for (s_id, s_cfg) in map {
                                        if !s_cfg.is_object() {
                                            errors.push(format!(
                                                "Server '{s_id}' configuration must be an object"
                                            ));
                                            schema_compliant = false;
                                        } else if s_cfg.get("command").is_none()
                                            && s_cfg.get("url").is_none()
                                        {
                                            warnings.push(format!("Server '{s_id}' has neither 'command' nor 'url' property"));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(format!("JSON syntax error: {e}"));
                        syntax_valid = false;
                        schema_compliant = false;
                    }
                }
            }
            "yaml" | "yml" => match serde_yaml::from_str::<serde_yaml::Value>(&content) {
                Ok(val) => {
                    if !val.is_mapping() {
                        errors.push("Root YAML must be a mapping".to_string());
                        schema_compliant = false;
                    } else {
                        let root = val.as_mapping().unwrap();
                        let expected_key = match client_id {
                            "goose" => "extensions",
                            "hermes" => "mcp_servers",
                            _ => "mcpServers",
                        };
                        let key_val = serde_yaml::Value::String(expected_key.to_string());
                        if let Some(servers) = root.get(&key_val) {
                            if let Some(map) = servers.as_mapping() {
                                server_count = map.len();
                            }
                        }
                    }
                }
                Err(e) => {
                    errors.push(format!("YAML syntax error: {e}"));
                    syntax_valid = false;
                    schema_compliant = false;
                }
            },
            "toml" => match toml::from_str::<toml::Value>(&content) {
                Ok(val) => {
                    if !val.is_table() {
                        errors.push("Root TOML must be a table".to_string());
                        schema_compliant = false;
                    } else {
                        let root = val.as_table().unwrap();
                        if let Some(servers) = root.get("mcp_servers").and_then(|s| s.as_table()) {
                            server_count = servers.len();
                        }
                    }
                }
                Err(e) => {
                    errors.push(format!("TOML syntax error: {e}"));
                    syntax_valid = false;
                    schema_compliant = false;
                }
            },
            _ => {
                warnings.push(format!("Unrecognized file extension: {ext}"));
            }
        }

        SchemaVerificationResult {
            client_id: client_id.to_string(),
            display_name: display_name.to_string(),
            path: loc.path.display().to_string(),
            format: ext,
            is_installed: true,
            syntax_valid,
            schema_compliant,
            server_count,
            errors,
            warnings,
        }
    }
}
