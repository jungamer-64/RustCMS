mod helpers;
use helpers::common::{make_temp_dirs, run_cargo_gen_biscuit_keys, find_gz_in_dir};

// Full (slower) compression test; fast variant exists in gen_biscuit_keys_compress_fast.rs
#[test]
fn compress_creates_gz_backup() {
    let (_tmp, out_dir, backup_dir) = make_temp_dirs();

    // First run to create initial files via `cargo run --bin gen_biscuit_keys -- ...`
    run_cargo_gen_biscuit_keys(&["--format", "files", "--out-dir", out_dir.to_string_lossy().as_ref(), "--backup", "--backup-dir", backup_dir.to_string_lossy().as_ref(), "--force"]);

    // run again with compress (no artificial delay; binary should be fast)
    run_cargo_gen_biscuit_keys(&["--format", "files", "--out-dir", out_dir.to_string_lossy().as_ref(), "--backup", "--backup-dir", backup_dir.to_string_lossy().as_ref(), "--backup-compress", "--force"]);

    assert!(find_gz_in_dir(&backup_dir), "expected at least one .gz backup file");
}
