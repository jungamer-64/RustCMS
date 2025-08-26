use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn compress_creates_gz_backup() {
    let tmp = tempdir().unwrap();
    let out_dir = tmp.path().join("keys");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&out_dir).unwrap();
    fs::create_dir_all(&backup_dir).unwrap();

    // First run to create initial files
    let mut cmd = Command::cargo_bin("gen_biscuit_keys").unwrap();
    cmd.arg("--format").arg("files")
        .arg("--out-dir").arg(out_dir.to_string_lossy().as_ref())
        .arg("--backup")
        .arg("--backup-dir").arg(backup_dir.to_string_lossy().as_ref())
        .arg("--force");
    cmd.assert().success();

    // run again with compress
    let mut cmd = Command::cargo_bin("gen_biscuit_keys").unwrap();
    cmd.arg("--format").arg("files")
        .arg("--out-dir").arg(out_dir.to_string_lossy().as_ref())
        .arg("--backup")
        .arg("--backup-dir").arg(backup_dir.to_string_lossy().as_ref())
        .arg("--backup-compress")
        .arg("--force");
    cmd.assert().success();

    // verify there's at least one .gz file in backup_dir
    let mut found = false;
    for entry in fs::read_dir(&backup_dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".gz") {
            found = true;
            break;
        }
    }
    assert!(found, "expected at least one .gz backup file");
}
