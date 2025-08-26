use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn retention_keeps_only_n_backups() {
    // Create temporary directories
    let tmp = tempdir().unwrap();
    let out_dir = tmp.path().join("keys");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&out_dir).unwrap();
    fs::create_dir_all(&backup_dir).unwrap();

    // Run the binary multiple times to generate backups
    for _ in 0..NUM_BACKUPS_TO_GENERATE {
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--manifest-path").arg("Cargo.toml").arg("--bin").arg("gen_biscuit_keys").arg("--")
            .arg("--format").arg("files")
            .arg("--out-dir").arg(out_dir.to_string_lossy().as_ref())
            .arg("--backup")
            .arg("--backup-dir").arg(backup_dir.to_string_lossy().as_ref())
            .arg("--force");
        cmd.assert().success();
    }

    // Now run with max_backups = 2 and backup again
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--manifest-path").arg("Cargo.toml").arg("--bin").arg("gen_biscuit_keys").arg("--")
        .arg("--format").arg("files")
        .arg("--out-dir").arg(out_dir.to_string_lossy().as_ref())
        .arg("--backup")
        .arg("--backup-dir").arg(backup_dir.to_string_lossy().as_ref())
        .arg("--max-backups").arg("2")
        .arg("--force");
    cmd.assert().success();

    // Count backups for private key prefix
    let mut priv_backups = 0;
    for entry in fs::read_dir(&backup_dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("biscuit_private.b64.bak.") {
            priv_backups += 1;
        }
    }

    assert!(priv_backups <= 2, "expected at most 2 private backups, found {}", priv_backups);
}
