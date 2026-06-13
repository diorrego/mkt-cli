#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn shows_help_with_no_args() {
    Command::cargo_bin("mkt")
        .unwrap()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage: mkt"));
}

#[test]
fn version_flag() {
    Command::cargo_bin("mkt")
        .unwrap()
        .args(["--version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("mkt"));
}

#[test]
fn providers_command_lists_meta() {
    Command::cargo_bin("mkt")
        .unwrap()
        .args(["providers"])
        .assert()
        .success()
        .stdout(predicate::str::contains("meta"));
}

/// All four providers ship in the default feature set; a regression in
/// feature gating must not silently drop one from the binary.
#[test]
fn providers_command_lists_all_default_providers() {
    Command::cargo_bin("mkt")
        .unwrap()
        .args(["providers"])
        .assert()
        .success()
        .stdout(predicate::str::contains("meta"))
        .stdout(predicate::str::contains("google"))
        .stdout(predicate::str::contains("tiktok"))
        .stdout(predicate::str::contains("linkedin"));
}

#[test]
fn doctor_reports_missing_config() {
    Command::cargo_bin("mkt")
        .unwrap()
        .args(["doctor"])
        .env("MKT_CONFIG_DIR", "/tmp/mkt-test-nonexistent")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("does not exist").or(predicate::str::contains("not found")),
        );
}

#[test]
fn help_subcommand() {
    Command::cargo_bin("mkt")
        .unwrap()
        .args(["help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Multi-platform marketing CLI"));
}

#[test]
fn meta_campaign_help() {
    Command::cargo_bin("mkt")
        .unwrap()
        .args(["meta", "campaign", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"));
}

#[test]
fn profile_list_with_no_config() {
    Command::cargo_bin("mkt")
        .unwrap()
        .args(["profile", "list"])
        .env("MKT_CONFIG_DIR", "/tmp/mkt-test-nonexistent")
        .assert()
        .success()
        .stdout(predicate::str::contains("No profiles"));
}

#[test]
fn unknown_subcommand_shows_error() {
    Command::cargo_bin("mkt")
        .unwrap()
        .args(["nonexistent"])
        .assert()
        .failure();
}

/// `--output` must honor the `MKT_OUTPUT` env var for flag/env parity in CI.
#[test]
fn output_format_resolves_from_env() {
    let output = Command::cargo_bin("mkt")
        .unwrap()
        .args(["providers"])
        .env("MKT_OUTPUT", "json")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<serde_json::Value>(stdout.trim())
        .expect("MKT_OUTPUT=json must produce JSON on stdout");
}

/// The env/flag parity must be discoverable: `--help` documents the env
/// var names, and an invalid env value is rejected like an invalid flag.
#[test]
fn env_parity_is_documented_and_validated() {
    Command::cargo_bin("mkt")
        .unwrap()
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MKT_PROFILE"))
        .stdout(predicate::str::contains("MKT_OUTPUT"));

    Command::cargo_bin("mkt")
        .unwrap()
        .args(["providers"])
        .env("MKT_OUTPUT", "bogus-format")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid value 'bogus-format'"));
}

/// --quiet and --verbose contradict each other and must be rejected.
#[test]
fn quiet_conflicts_with_verbose() {
    Command::cargo_bin("mkt")
        .unwrap()
        .args(["--quiet", "--verbose", "providers"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

/// Short flags for the most common agent/script options.
#[test]
fn short_flags_work() {
    let output = Command::cargo_bin("mkt")
        .unwrap()
        .args(["-o", "json", "providers"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<serde_json::Value>(stdout.trim()).expect("-o json must produce JSON");
}

/// `mkt ... | head` must exit 0 when downstream closes the pipe, not
/// panic or report an error (clig.dev: be a good pipe citizen).
#[test]
#[cfg(unix)]
fn closed_stdout_pipe_exits_cleanly() {
    use std::process::{Command as StdCommand, Stdio};

    let bin = assert_cmd::cargo::cargo_bin("mkt");
    let mut child = StdCommand::new(bin)
        .args(["providers"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Close our read end immediately so the child's writes hit EPIPE.
    drop(child.stdout.take());
    let status = child.wait().unwrap();
    assert!(
        status.success(),
        "a closed pipe is a normal end of output, got {status:?}"
    );
}

/// Diagnostics must not contain ANSI escapes when stderr is not a
/// terminal (it never is under the test harness).
#[test]
fn piped_stderr_has_no_ansi_escapes() {
    let output = Command::cargo_bin("mkt")
        .unwrap()
        .args(["--verbose", "providers"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains('\u{1b}'),
        "piped stderr must be ANSI-free: {stderr:?}"
    );
}

/// NO_COLOR must also strip ANSI even if a terminal were attached.
#[test]
fn no_color_is_respected() {
    let output = Command::cargo_bin("mkt")
        .unwrap()
        .args(["--verbose", "providers"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains('\u{1b}'), "NO_COLOR must win: {stderr:?}");
}

// ── Exit code + structured error contract (for agents/scripts) ────────────────

/// Missing credentials must exit with code 3 (auth error).
#[test]
fn missing_token_exits_with_auth_code() {
    Command::cargo_bin("mkt")
        .unwrap()
        .args(["meta", "campaign", "list"])
        .env("MKT_CONFIG_DIR", "/tmp/mkt-test-nonexistent")
        .env_remove("MKT_META_ACCESS_TOKEN")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("Authentication failed"));
}

/// With --output json, errors are emitted as a structured JSON object on
/// stderr with a stable `error_type` and a recovery `suggestion`.
#[test]
fn json_output_emits_structured_error() {
    let output = Command::cargo_bin("mkt")
        .unwrap()
        .args(["--output", "json", "meta", "campaign", "list"])
        .env("MKT_CONFIG_DIR", "/tmp/mkt-test-nonexistent")
        .env_remove("MKT_META_ACCESS_TOKEN")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: serde_json::Value =
        serde_json::from_str(stderr.trim()).expect("stderr should be valid JSON");
    assert_eq!(parsed["ok"], false);
    assert_eq!(parsed["error"]["type"], "auth_error");
    assert!(parsed["error"]["message"].is_string());
    assert!(
        parsed["error"]["suggestion"]
            .as_str()
            .unwrap()
            .contains("doctor")
    );
}

/// Invalid inline JSON (e.g. targeting) must exit with code 2 (validation).
#[test]
fn invalid_targeting_json_exits_with_validation_code() {
    Command::cargo_bin("mkt")
        .unwrap()
        .args([
            "meta",
            "adset",
            "create",
            "--campaign",
            "c1",
            "--name",
            "x",
            "--targeting",
            "{not json",
        ])
        .env("MKT_CONFIG_DIR", "/tmp/mkt-test-nonexistent")
        .env("MKT_META_ACCESS_TOKEN", "dummy-token")
        .env("MKT_META_AD_ACCOUNT_ID", "act_1")
        .assert()
        .code(2);
}

/// The exit code contract must be documented in --help for agent discovery.
#[test]
fn help_documents_exit_codes() {
    Command::cargo_bin("mkt")
        .unwrap()
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Exit codes"))
        .stdout(predicate::str::contains("rate limited"));
}

/// Doctor must report per-provider credential sources without exposing values.
#[test]
fn doctor_reports_credential_sources() {
    let output = Command::cargo_bin("mkt")
        .unwrap()
        .args(["doctor"])
        .env("MKT_CONFIG_DIR", "/tmp/mkt-test-nonexistent")
        .env("MKT_META_ACCESS_TOKEN", "secret-token-value")
        .env_remove("MKT_GOOGLE_DEVELOPER_TOKEN")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("MKT_META_ACCESS_TOKEN") && stdout.contains("[ok]"),
        "doctor should flag the env var as set: {stdout}"
    );
    assert!(
        !stdout.contains("secret-token-value"),
        "doctor must never print token values: {stdout}"
    );
}

/// Shell completion generation must work for the major shells.
#[test]
fn completions_command_generates_bash_script() {
    Command::cargo_bin("mkt")
        .unwrap()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_mkt"));
}

/// The MCP server must answer initialize and tools/list over stdio with
/// the consolidated tool set.
#[test]
fn mcp_serve_lists_tools_over_stdio() {
    let requests = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        "\n",
    );

    let output = Command::cargo_bin("mkt")
        .unwrap()
        .args(["mcp", "serve"])
        .env("MKT_CONFIG_DIR", "/tmp/mkt-test-nonexistent")
        .write_stdin(requests)
        .timeout(std::time::Duration::from_secs(20))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(r#""name":"mkt""#),
        "initialize should report the server name: {stdout}"
    );
    for tool in [
        "campaign_list",
        "campaign_get",
        "campaign_create",
        "campaign_set_status",
        "insights_get",
        "provider_health",
    ] {
        assert!(stdout.contains(tool), "tools/list missing {tool}: {stdout}");
    }
    // Spend-safety must be stated where the model can see it.
    assert!(
        stdout.contains("PAUSED"),
        "tool descriptions must state the paused-by-default contract: {stdout}"
    );
}
