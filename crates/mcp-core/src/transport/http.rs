use anyhow::{anyhow, Context};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::time::timeout;

use crate::protocol::{
    ClientInfo, InitializeParams, InitializeResult, JsonRpcRequest, JsonRpcResponse,
    LATEST_PROTOCOL_VERSION,
};
use crate::types::HealthStatus;

pub async fn check_http_health(
    url: &str,
    headers: &BTreeMap<String, String>,
    timeout_duration: Duration,
) -> HealthStatus {
    let start = std::time::Instant::now();

    let check = async {
        let client = reqwest::Client::builder()
            .timeout(timeout_duration)
            .build()?;

        let mut req_builder = client.post(url);
        for (k, v) in headers {
            req_builder = req_builder.header(k, v);
        }

        let init_params = serde_json::to_value(InitializeParams {
            protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
            capabilities: serde_json::json!({}),
            client_info: ClientInfo {
                name: "mcpforge".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        })?;

        let req = JsonRpcRequest::new(1, "initialize", Some(init_params));
        let resp = req_builder
            .json(&req)
            .send()
            .await
            .context("Failed to send HTTP request to MCP endpoint")?;

        if !resp.status().is_success() {
            return Err(anyhow!("HTTP error status: {}", resp.status()));
        }

        let json_resp: JsonRpcResponse = resp
            .json()
            .await
            .context("Failed to parse JSON-RPC response from HTTP endpoint")?;

        if let Some(err) = json_resp.error {
            return Err(anyhow!("JSON-RPC error {}: {}", err.code, err.message));
        }

        let init_val = json_resp
            .result
            .ok_or_else(|| anyhow!("Missing result in initialize response"))?;
        let init_result: InitializeResult =
            serde_json::from_value(init_val).context("Failed to parse InitializeResult")?;

        let mut tool_count = 0;
        let mut tools_builder = client.post(url);
        for (k, v) in headers {
            tools_builder = tools_builder.header(k, v);
        }
        let tools_req = JsonRpcRequest::new(2, "tools/list", Some(serde_json::json!({})));
        if let Ok(tools_resp) = tools_builder.json(&tools_req).send().await {
            if tools_resp.status().is_success() {
                if let Ok(tools_json) = tools_resp.json::<JsonRpcResponse>().await {
                    if let Some(res_val) = tools_json.result {
                        if let Ok(tools_res) =
                            serde_json::from_value::<crate::protocol::ToolsListResult>(res_val)
                        {
                            tool_count = tools_res.tools.len();
                        }
                    }
                }
            }
        }

        Ok((
            init_result.server_info.name,
            init_result.server_info.version,
            tool_count,
        ))
    };

    match timeout(timeout_duration, check).await {
        Ok(Ok((server_name, server_version, tool_count))) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            HealthStatus::Healthy {
                latency_ms,
                tool_count,
                server_name,
                server_version,
            }
        }
        Ok(Err(e)) => HealthStatus::Broken {
            error: format!("{:#}", e),
        },
        Err(_) => HealthStatus::Broken {
            error: format!("Timeout after {}s", timeout_duration.as_secs()),
        },
    }
}

pub async fn check_sse_health(url: &str, timeout_duration: Duration) -> HealthStatus {
    let start = std::time::Instant::now();
    let check = async {
        let client = reqwest::Client::builder()
            .timeout(timeout_duration)
            .build()?;

        let resp = client
            .get(url)
            .send()
            .await
            .context("Failed to connect to SSE endpoint")?;
        if resp.status().is_success() || resp.status().as_u16() == 405 {
            Ok(())
        } else {
            Err(anyhow!("SSE endpoint returned status {}", resp.status()))
        }
    };

    match timeout(timeout_duration, check).await {
        Ok(Ok(())) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            HealthStatus::Degraded {
                reason: "Legacy SSE transport (deprecated, consider Streamable HTTP)".to_string(),
                latency_ms: Some(latency_ms),
            }
        }
        Ok(Err(e)) => HealthStatus::Broken {
            error: format!("{:#}", e),
        },
        Err(_) => HealthStatus::Broken {
            error: format!("Timeout after {}s", timeout_duration.as_secs()),
        },
    }
}
