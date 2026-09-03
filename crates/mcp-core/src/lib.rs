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

    #[test]
    fn test_resolve_executable_path_respects_permissions() {
        use std::fs::File;
        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempfile::tempdir().unwrap();
        let bin_path = temp_dir.path().join("my-test-bin");
        File::create(&bin_path).unwrap();

        #[cfg(unix)]
        {
            // Initially not executable (0o644)
            let mut perms = std::fs::metadata(&bin_path).unwrap().permissions();
            perms.set_mode(0o644);
            std::fs::set_permissions(&bin_path, perms).unwrap();

            // Direct path resolution should fail because executable bit is not set
            assert!(
                transport::stdio::resolve_executable_path(bin_path.to_str().unwrap()).is_none()
            );

            // Make executable (0o755)
            let mut perms = std::fs::metadata(&bin_path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&bin_path, perms).unwrap();

            // Now it should resolve!
            assert_eq!(
                transport::stdio::resolve_executable_path(bin_path.to_str().unwrap()),
                Some(bin_path)
            );
        }
    }

    #[test]
    fn test_tool_call_serialization() {
        use crate::protocol::{CallToolParams, CallToolResult, ToolContent};

        let params = CallToolParams {
            name: "read_file".to_string(),
            arguments: serde_json::json!({ "path": "/test/path.txt" }),
        };
        let serialized = serde_json::to_string(&params).unwrap();
        assert!(serialized.contains("\"name\":\"read_file\""));
        assert!(serialized.contains("\"arguments\":{\"path\":\"/test/path.txt\"}"));

        let res_json = r#"{
            "content": [
                { "type": "text", "text": "file content here" }
            ],
            "isError": false
        }"#;
        let res: CallToolResult = serde_json::from_str(res_json).unwrap();
        assert!(!res.is_error);
        assert_eq!(res.content.len(), 1);
        let expected_content = ToolContent {
            content_type: "text".to_string(),
            text: Some("file content here".to_string()),
            data: None,
            mime_type: None,
        };
        assert_eq!(res.content[0].content_type, expected_content.content_type);
        assert_eq!(res.content[0].text, expected_content.text);
    }
}
