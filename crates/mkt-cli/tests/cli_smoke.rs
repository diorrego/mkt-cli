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
