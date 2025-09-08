// mod helpers; // Remove duplicate
mod helpers;
use helpers::common::{make_temp_dirs, run_cargo_gen_biscuit_keys, find_gz_in_dir};

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
    run_cargo_gen_biscuit_keys(&["--format", "files", "--out-dir", out_dir.to_string_lossy().as_ref(), "--force"]);
    // Second run triggers backup + compression in one shot
    run_cargo_gen_biscuit_keys(&["--format", "files", "--out-dir", out_dir.to_string_lossy().as_ref(), "--backup", "--backup-dir", backup_dir.to_string_lossy().as_ref(), "--backup-compress", "--force"]);

    assert!(find_gz_in_dir(&backup_dir), "expected at least one .gz backup file (fast test)");
}
