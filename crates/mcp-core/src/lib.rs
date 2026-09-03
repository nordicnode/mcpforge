pub mod client;
pub mod protocol;
pub mod transport;
pub mod types;

pub use client::check_server_health;
pub use protocol::{
    JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, ToolDefinition,
};
pub use types::{ClientRef, HealthStatus, Scope, ServerEntry, Transport};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_server_entry_serialization() {
        let mut env = BTreeMap::new();
        env.insert("GITHUB_TOKEN".to_string(), "secret123".to_string());

        let entry = ServerEntry::new_stdio(
            "github",
            "npx",
            vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-github".to_string(),
            ],
            env,
        );

        let json = serde_json::to_string_pretty(&entry).expect("failed to serialize");
        let parsed: ServerEntry = serde_json::from_str(&json).expect("failed to deserialize");
        assert_eq!(parsed.id, "github");
        assert!(parsed.enabled);
        if let Transport::Stdio {
            command,
            args,
            env: parsed_env,
        } = parsed.transport
        {
            assert_eq!(command, "npx");
            assert_eq!(args.len(), 2);
            assert_eq!(parsed_env.get("GITHUB_TOKEN").unwrap(), "secret123");
        } else {
            panic!("Expected stdio transport");
        }
    }

    #[test]
    fn test_jsonrpc_request_response_cycle() {
        let req = JsonRpcRequest::new(1, "initialize", Some(serde_json::json!({ "foo": "bar" })));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"initialize\""));

        let resp_json = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","serverInfo":{"name":"test-server","version":"1.0.0"}}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(resp_json).unwrap();
        assert_eq!(resp.id, serde_json::json!(1));
        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
    }
}
