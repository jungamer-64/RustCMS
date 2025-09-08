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
