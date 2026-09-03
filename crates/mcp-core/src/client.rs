use crate::transport::stdio::check_stdio_health;
use crate::types::{HealthStatus, ServerEntry, Transport};
use std::time::Duration;

pub async fn check_server_health(entry: &ServerEntry, timeout_secs: u64) -> HealthStatus {
    if !entry.enabled {
        return HealthStatus::Disabled;
    }

    let dur = Duration::from_secs(timeout_secs);

    match &entry.transport {
        Transport::Stdio { command, args, env } => {
            check_stdio_health(command, args, env, dur).await
        }
        #[cfg(feature = "http")]
        Transport::StreamableHttp { url, headers } => {
            crate::transport::http::check_http_health(url, headers, dur).await
        }
        #[cfg(not(feature = "http"))]
        Transport::StreamableHttp { .. } => HealthStatus::Degraded {
            reason: "HTTP transport not enabled in build".to_string(),
            latency_ms: None,
        },
        #[cfg(feature = "http")]
        Transport::Sse { url } => crate::transport::http::check_sse_health(url, dur).await,
        #[cfg(not(feature = "http"))]
        Transport::Sse { .. } => HealthStatus::Degraded {
            reason: "SSE transport not enabled in build".to_string(),
            latency_ms: None,
        },
    }
}

pub async fn list_server_tools(
    entry: &ServerEntry,
    timeout_secs: u64,
) -> anyhow::Result<Vec<crate::protocol::ToolDefinition>> {
    let dur = Duration::from_secs(timeout_secs);
    match &entry.transport {
        Transport::Stdio { command, args, env } => {
            let mut client =
                crate::transport::stdio::StdioClient::spawn(command, args, env).await?;
            client.initialize().await?;
            tokio::time::timeout(dur, client.list_tools())
                .await
                .map_err(|_| anyhow::anyhow!("Timeout listing tools after {}s", timeout_secs))?
        }
        #[cfg(feature = "http")]
        Transport::StreamableHttp { url, headers } => {
            crate::transport::http::list_http_tools(url, headers, dur).await
        }
        #[cfg(not(feature = "http"))]
        Transport::StreamableHttp { .. } => {
            Err(anyhow::anyhow!("HTTP transport not enabled in build"))
        }
        Transport::Sse { .. } => Err(anyhow::anyhow!(
            "SSE transport does not support direct tools/list"
        )),
    }
}

pub async fn call_server_tool(
    entry: &ServerEntry,
    tool_name: &str,
    arguments: serde_json::Value,
    timeout_secs: u64,
) -> anyhow::Result<crate::protocol::CallToolResult> {
    let dur = Duration::from_secs(timeout_secs);
    match &entry.transport {
        Transport::Stdio { command, args, env } => {
            let mut client =
                crate::transport::stdio::StdioClient::spawn(command, args, env).await?;
            client.initialize().await?;
            tokio::time::timeout(dur, client.call_tool(tool_name, arguments))
                .await
                .map_err(|_| anyhow::anyhow!("Timeout calling tool after {}s", timeout_secs))?
        }
        #[cfg(feature = "http")]
        Transport::StreamableHttp { url, headers } => {
            crate::transport::http::call_http_tool(url, headers, tool_name, arguments, dur).await
        }
        #[cfg(not(feature = "http"))]
        Transport::StreamableHttp { .. } => {
            Err(anyhow::anyhow!("HTTP transport not enabled in build"))
        }
        Transport::Sse { .. } => Err(anyhow::anyhow!(
            "SSE transport does not support direct tools/call"
        )),
    }
}
