mod helpers;
use assert_cmd::Command;
// Needed for `Command::cargo_bin` extension method
#[allow(unused_imports)]
use assert_cmd::cargo::CommandCargoExt;
use helpers::common::{make_temp_dirs, find_gz_in_dir};

// Fast path compression test gated by env flag to skip full two-run sequence.
#[test]
fn compress_creates_gz_backup_fast() {
    if std::env::var("FAST_KEY_TESTS").ok().as_deref() != Some("1") {
        // Skip in normal (full) run; only active when FAST_KEY_TESTS=1
        eprintln!("skipping fast compression test (set FAST_KEY_TESTS=1 to enable)");
        return;
    }
    let (_tmp, out_dir, backup_dir) = make_temp_dirs();

    // First create initial key files (no backup yet)
    let mut first = Command::cargo_bin("gen_biscuit_keys").expect("build binary");
    first.arg("--format").arg("files")
        .arg("--out-dir").arg(out_dir.to_string_lossy().as_ref())
        .arg("--force");
    first.assert().success();

    // Second run triggers backup + compression in one shot
    let mut second = Command::cargo_bin("gen_biscuit_keys").expect("build binary");
    second.arg("--format").arg("files")
        .arg("--out-dir").arg(out_dir.to_string_lossy().as_ref())
        .arg("--backup")
        .arg("--backup-dir").arg(backup_dir.to_string_lossy().as_ref())
        .arg("--backup-compress")
        .arg("--force");
    second.assert().success();

    assert!(find_gz_in_dir(&backup_dir), "expected at least one .gz backup file (fast test)");
}
