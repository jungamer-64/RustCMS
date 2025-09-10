mod helpers;
use helpers::common::{find_gz_in_dir, make_temp_dirs, run_compressed_backup_sequence};

// Full (slower) compression test; fast variant exists in gen_biscuit_keys_compress_fast.rs
#[test]
fn compress_creates_gz_backup() {
    let (_tmp, out_dir, backup_dir) = make_temp_dirs();

    run_compressed_backup_sequence(
        out_dir.to_string_lossy().as_ref(),
        backup_dir.to_string_lossy().as_ref(),
    );

    assert!(
        find_gz_in_dir(&backup_dir),
        "expected at least one .gz backup file"
    );
}
