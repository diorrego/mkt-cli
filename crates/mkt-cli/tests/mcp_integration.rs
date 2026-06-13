//! Integration tests for the MCP server's `tools/call` surface.
//!
//! `cli_smoke.rs` covers `initialize` + `tools/list`; these tests exercise
//! actual tool invocations over stdio and pin down the error contract that
//! chat agents rely on: structured `[error_type]` prefixes, recovery
//! suggestions, and no credential leakage.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use assert_cmd::Command;

/// Build the JSON-RPC handshake followed by a single `tools/call` request.
fn handshake_then_call(tool: &str, arguments: &serde_json::Value) -> String {
    let call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {"name": tool, "arguments": arguments}
    });
    format!(
        "{}\n{}\n{}\n",
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        call,
    )
}

/// Run `mkt mcp serve` with no credentials configured and return stdout.
fn run_mcp_without_credentials(input: String) -> String {
    let output = Command::cargo_bin("mkt")
        .unwrap()
        .args(["mcp", "serve"])
        .env("MKT_CONFIG_DIR", "/tmp/mkt-test-nonexistent")
        .env_remove("MKT_META_ACCESS_TOKEN")
        .env_remove("MKT_GOOGLE_DEVELOPER_TOKEN")
        .env_remove("MKT_TIKTOK_ACCESS_TOKEN")
        .env_remove("MKT_LINKEDIN_ACCESS_TOKEN")
        .write_stdin(input)
        .timeout(std::time::Duration::from_secs(20))
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The response to request id 2 (the `tools/call`), parsed as JSON.
fn call_response(stdout: &str) -> serde_json::Value {
    stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|v| v["id"] == 2)
        .unwrap_or_else(|| panic!("no response with id 2 in: {stdout}"))
}

/// An unknown provider must be rejected as invalid params, naming the
/// valid options so the agent can self-correct.
#[test]
fn tools_call_unknown_provider_is_invalid_params() {
    let stdout = run_mcp_without_credentials(handshake_then_call(
        "campaign_list",
        &serde_json::json!({"provider": "myspace"}),
    ));
    let response = call_response(&stdout);
    let message = response["error"]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("unknown provider"),
        "error should say the provider is unknown: {response}"
    );
    assert!(
        message.contains("meta") && message.contains("linkedin"),
        "error should list valid providers for self-correction: {response}"
    );
}

/// Without credentials, a read-only tool must fail with the structured
/// auth error contract: `[auth_error]` type tag plus a recovery suggestion.
#[test]
fn tools_call_without_credentials_reports_structured_auth_error() {
    let stdout = run_mcp_without_credentials(handshake_then_call(
        "campaign_list",
        &serde_json::json!({"provider": "meta"}),
    ));
    let response = call_response(&stdout);
    let message = response["error"]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("auth_error"),
        "auth failures must carry the stable error_type tag: {response}"
    );
    assert!(
        message.contains("doctor") || message.contains("MKT_"),
        "auth failures must include an actionable suggestion: {response}"
    );
}

/// The mutating status tool must fail the same way — never silently
/// succeed — when credentials are missing.
#[test]
fn tools_call_set_status_without_credentials_fails() {
    let stdout = run_mcp_without_credentials(handshake_then_call(
        "campaign_set_status",
        &serde_json::json!({"provider": "meta", "campaign_id": "c1", "status": "active"}),
    ));
    let response = call_response(&stdout);
    assert!(
        response.get("error").is_some(),
        "activating a campaign without credentials must be an error: {response}"
    );
}

/// `provider_health` must degrade to a clean error without credentials and
/// must never echo environment values back to the client.
#[test]
fn tools_call_health_never_leaks_token_values() {
    let input = handshake_then_call("provider_health", &serde_json::json!({"provider": "meta"}));
    let output = Command::cargo_bin("mkt")
        .unwrap()
        .args(["mcp", "serve"])
        .env("MKT_CONFIG_DIR", "/tmp/mkt-test-nonexistent")
        .env("MKT_META_ACCESS_TOKEN", "super-secret-value-do-not-print")
        .write_stdin(input)
        .timeout(std::time::Duration::from_secs(20))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.contains("super-secret-value-do-not-print")
            && !stderr.contains("super-secret-value-do-not-print"),
        "MCP output must never contain raw token values"
    );
}

/// Tool calls for every provider must answer (not hang) without
/// credentials — the dispatch macro covers all four arms.
#[test]
fn tools_call_answers_for_all_providers() {
    for provider in ["meta", "google", "tiktok", "linkedin"] {
        let stdout = run_mcp_without_credentials(handshake_then_call(
            "provider_health",
            &serde_json::json!({"provider": provider}),
        ));
        let response = call_response(&stdout);
        assert!(
            response.get("error").is_some() || response.get("result").is_some(),
            "{provider}: expected a JSON-RPC response, got: {response}"
        );
    }
}

/// `dry_run: true` on a mutating tool must succeed WITHOUT credentials
/// and describe the action instead of executing it — the same preview
/// contract the CLI offers with --dry-run.
#[test]
fn tools_call_create_dry_run_previews_without_credentials() {
    let stdout = run_mcp_without_credentials(handshake_then_call(
        "campaign_create",
        &serde_json::json!({
            "provider": "meta",
            "name": "Preview",
            "objective": "OUTCOME_TRAFFIC",
            "dry_run": true
        }),
    ));
    let response = call_response(&stdout);
    assert!(
        response.get("error").is_none(),
        "dry run must not need credentials: {response}"
    );
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(
        text.contains("dry_run") && text.contains("Preview"),
        "dry run must echo the would-be action: {text}"
    );
    assert!(
        text.contains("PAUSED") || text.contains("paused"),
        "the preview must state the paused-by-default contract: {text}"
    );
}

/// `dry_run: true` on campaign_set_status must preview the transition.
#[test]
fn tools_call_set_status_dry_run_previews() {
    let stdout = run_mcp_without_credentials(handshake_then_call(
        "campaign_set_status",
        &serde_json::json!({
            "provider": "meta",
            "campaign_id": "c1",
            "status": "active",
            "dry_run": true
        }),
    ));
    let response = call_response(&stdout);
    assert!(
        response.get("error").is_none(),
        "dry run must not need credentials: {response}"
    );
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(
        text.contains("active") && text.contains("c1"),
        "the preview must describe the transition: {text}"
    );
}
