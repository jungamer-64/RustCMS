// mod helpers; // Remove duplicate
mod helpers;
use helpers::common::{
    find_gz_in_dir, make_temp_dirs, run_compressed_backup_sequence,
};

// Fast path compression test gated by env flag to skip full two-run sequence.
#[test]
fn compress_creates_gz_backup_fast() {
    if std::env::var("FAST_KEY_TESTS").ok().as_deref() != Some("1") {
        // Skip in normal (full) run; only active when FAST_KEY_TESTS=1
        eprintln!("skipping fast compression test (set FAST_KEY_TESTS=1 to enable)");
        return;
    }
    let (_tmp, out_dir, backup_dir) = make_temp_dirs();
    // Use same shared helper (two-pass); fast variant still gated by env
    run_compressed_backup_sequence(
        out_dir.to_string_lossy().as_ref(),
        backup_dir.to_string_lossy().as_ref(),
    );

    assert!(
        find_gz_in_dir(&backup_dir),
        "expected at least one .gz backup file (fast test)"
    );
}
