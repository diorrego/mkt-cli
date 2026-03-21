//! Shared HTTP client builder.

use std::time::Duration;

use reqwest::Client;

use crate::error::Result;

/// Default request timeout in seconds.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Build a configured `reqwest::Client` with sensible defaults.
///
/// Features:
/// - `rustls-tls` (no OpenSSL)
/// - Gzip + Brotli decompression
/// - Configurable timeout (default: 30s)
/// - Custom User-Agent
pub fn build_http_client(timeout_secs: Option<u64>) -> Result<Client> {
    let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));

    let client = Client::builder()
        .timeout(timeout)
        .user_agent(format!("mkt/{}", env!("CARGO_PKG_VERSION")))
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
