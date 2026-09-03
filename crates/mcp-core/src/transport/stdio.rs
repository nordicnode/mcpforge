use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::time::timeout;
use tracing::debug;

use crate::protocol::{
    ClientInfo, InitializeParams, InitializeResult, JsonRpcNotification, JsonRpcRequest,
    JsonRpcResponse, ToolsListResult, LATEST_PROTOCOL_VERSION,
};
use crate::types::HealthStatus;

pub struct StdioClient {
    child: Child,
    stdin: ChildStdin,
    reader: tokio::io::Lines<BufReader<ChildStdout>>,
    next_id: u64,
}

impl StdioClient {
    pub async fn spawn(
        command: &str,
        args: &[String],
        env: &BTreeMap<String, String>,
    ) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args);
        cmd.envs(env);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().with_context(|| {
            format!("Failed to spawn process '{}' with args {:?}", command, args)
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to open child stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to open child stdout"))?;
        let reader = BufReader::new(stdout).lines();

        Ok(Self {
            child,
            stdin,
            reader,
            next_id: 1,
        })
    }

    pub async fn send_request(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<JsonRpcResponse> {
        let id = self.next_id;
        self.next_id += 1;

        let req = JsonRpcRequest::new(id, method, params);
        let mut json_str = serde_json::to_string(&req)?;
        json_str.push('\n');

        self.stdin.write_all(json_str.as_bytes()).await?;
        self.stdin.flush().await?;

        // Read response, skipping any non-JSON log lines
        while let Some(line) = self.reader.next_line().await? {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(line) {
                if resp.id == id {
                    return Ok(resp);
                }
            } else {
                debug!("Ignoring non-JSON or unrelated output on stdout: {}", line);
            }
        }

        Err(anyhow!(
            "Process stream closed before receiving response for id {}",
            id
        ))
    }

    pub async fn send_notification(&mut self, method: &str, params: Option<Value>) -> Result<()> {
        let notif = JsonRpcNotification::new(method, params);
        let mut json_str = serde_json::to_string(&notif)?;
        json_str.push('\n');

        self.stdin.write_all(json_str.as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    pub async fn close(&mut self) {
        let _ = self.child.kill().await;
    }
}

impl Drop for StdioClient {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

pub fn resolve_executable_path(command: &str) -> Option<std::path::PathBuf> {
    if command.contains('/') || (cfg!(windows) && command.contains('\\')) {
        let p = std::path::PathBuf::from(command);
        if p.is_file() {
            return Some(p);
        }
        #[cfg(windows)]
        {
            for ext in [".exe", ".cmd", ".bat"] {
                let with_ext = p.with_extension(&ext[1..]);
                if with_ext.is_file() {
                    return Some(with_ext);
                }
            }
        }
        return None;
    }

    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let candidate = dir.join(command);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            for ext in [".cmd", ".exe", ".bat"] {
                let candidate_ext = dir.join(format!("{}{}", command, ext));
                if candidate_ext.is_file() {
                    return Some(candidate_ext);
                }
            }
        }
    }
    None
}

pub async fn check_stdio_health(
    command: &str,
    args: &[String],
    env: &BTreeMap<String, String>,
    timeout_duration: Duration,
) -> HealthStatus {
    let start = std::time::Instant::now();

    // Check if binary command exists cross-platform
    if resolve_executable_path(command).is_none() {
        return HealthStatus::Broken {
            error: format!("Executable '{}' not found in PATH", command),
        };
    }

    let handshake = async {
        let mut client = StdioClient::spawn(command, args, env).await?;

        // 1. Initialize
        let init_params = serde_json::to_value(InitializeParams {
            protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
            capabilities: serde_json::json!({}),
            client_info: ClientInfo {
                name: "mcpforge".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        })?;

        let init_resp = client.send_request("initialize", Some(init_params)).await?;
        if let Some(err) = init_resp.error {
            return Err(anyhow!("Initialize error {}: {}", err.code, err.message));
        }

        let init_val = init_resp
            .result
            .ok_or_else(|| anyhow!("Missing result in initialize response"))?;
        let init_result: InitializeResult =
            serde_json::from_value(init_val).context("Failed to parse InitializeResult")?;

        // 2. Initialized notification
        client
            .send_notification("notifications/initialized", None)
            .await?;

        // 3. List tools
        let mut tool_count = 0;
        let tools_resp = client
            .send_request("tools/list", Some(serde_json::json!({})))
            .await;
        if let Ok(resp) = tools_resp {
            if let Some(res_val) = resp.result {
                if let Ok(tools_res) = serde_json::from_value::<ToolsListResult>(res_val) {
                    tool_count = tools_res.tools.len();
                }
            }
        }

        client.close().await;

        Ok((
            init_result.server_info.name,
            init_result.server_info.version,
            tool_count,
        ))
    };

    match timeout(timeout_duration, handshake).await {
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
