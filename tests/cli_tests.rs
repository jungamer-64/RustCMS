use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn help_shows_commands() {
    let mut cmd = Command::cargo_bin("cms-migrate").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("migrate").and(predicate::str::contains("status")));
}

#[test]
fn migrate_allows_no_seed_flag() {
    let mut cmd = Command::cargo_bin("cms-migrate").unwrap();
    cmd.args(&["migrate", "--no-seed"]);
    // We cannot actually connect to DB in CI; assert that the binary accepts the args and prints something
    let assert = cmd.assert();
    // Either success or any error that indicates it parsed args; look for the 'Running database migrations' text
    assert.failure().stderr(predicate::str::contains("Running database migrations").or(predicate::str::contains("Connecting to database")));
}
