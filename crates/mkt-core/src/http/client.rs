//! Shared HTTP client builder.

use std::time::Duration;

use reqwest::Client;

use crate::error::Result;

/// Default request timeout in seconds.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Connection establishment (TCP + TLS) timeout, separate from the total
/// request timeout so a hung handshake fails fast.
const CONNECT_TIMEOUT_SECS: u64 = 10;

/// Build a configured `reqwest::Client` with sensible defaults.
///
/// Features:
/// - `rustls-tls` (no OpenSSL)
/// - Gzip + Brotli decompression
/// - Configurable timeout (default: 30s) plus a 10s connect timeout
/// - TCP keepalive so pooled connections survive NAT idling
/// - Custom User-Agent
///
/// # Errors
///
/// Returns an error if the HTTP client cannot be built (e.g. TLS init failure).
pub fn build_http_client(timeout_secs: Option<u64>) -> Result<Client> {
    let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));

    let client = Client::builder()
        .timeout(timeout)
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .tcp_keepalive(Duration::from_secs(60))
        .pool_idle_timeout(Duration::from_secs(90))
        .user_agent(format!(
            "mkt/{} (+https://mktcli.com)",
            env!("CARGO_PKG_VERSION")
        ))
        .gzip(true)
        .brotli(true)
        .build()?;

    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_client_with_defaults() {
        let client = build_http_client(None);
        assert!(client.is_ok());
    }

    #[test]
    fn build_client_with_custom_timeout() {
        let client = build_http_client(Some(60));
        assert!(client.is_ok());
    }
}
