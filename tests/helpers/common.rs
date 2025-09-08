use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;
use std::path::PathBuf;

pub fn make_temp_dirs() -> (TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let out_dir = tmp.path().join("keys");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&out_dir).unwrap();
    fs::create_dir_all(&backup_dir).unwrap();
    (tmp, out_dir, backup_dir)
}

pub fn run_cargo_gen_biscuit_keys(args: &[&str]) {
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--manifest-path").arg("Cargo.toml").arg("--bin").arg("gen_biscuit_keys").arg("--");
    for a in args { cmd.arg(a); }
    cmd.assert().success();
}

pub fn find_gz_in_dir(backup_dir: &PathBuf) -> bool {
    for entry in fs::read_dir(backup_dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".gz") { return true; }
    }
    false
}

/// Run gen_biscuit_keys multiple times to create backups
pub fn run_gen_biscuit_keys_multiple_backups(out_dir: &str, backup_dir: &str, times: usize) {
    for _ in 0..times {
        run_cargo_gen_biscuit_keys(&["--format", "files", "--out-dir", out_dir, "--backup", "--backup-dir", backup_dir, "--force"]);
    }
}

/// Run a two-phase (initial + compressed) backup generation used by multiple tests
pub fn run_compressed_backup_sequence(out_dir: &str, backup_dir: &str) {
    // initial generation (creates base keys + uncompressed backup)
    run_cargo_gen_biscuit_keys(&["--format", "files", "--out-dir", out_dir, "--backup", "--backup-dir", backup_dir, "--force"]);
    // second pass with compression flag
    run_cargo_gen_biscuit_keys(&["--format", "files", "--out-dir", out_dir, "--backup", "--backup-dir", backup_dir, "--backup-compress", "--force"]);
}

/// Count backups with a given prefix in a directory
pub fn count_backups_with_prefix(backup_dir: &PathBuf, prefix: &str) -> usize {
    let mut count = 0;
    for entry in fs::read_dir(backup_dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(prefix) {
            count += 1;
        }
    }
    count
}
