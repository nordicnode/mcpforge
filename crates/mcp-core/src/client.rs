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
