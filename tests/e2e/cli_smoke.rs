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
        .stdout(predicate::str::contains("does not exist").or(predicate::str::contains("not found")));
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
