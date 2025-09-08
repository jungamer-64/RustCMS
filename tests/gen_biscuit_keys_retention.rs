mod helpers;
use helpers::common::{make_temp_dirs, run_gen_biscuit_keys_multiple_backups, run_cargo_gen_biscuit_keys, count_backups_with_prefix, run_compressed_backup_sequence};

// How many times to generate backups in the smoke portion of this test.
const NUM_BACKUPS_TO_GENERATE: usize = 5;

#[test]
fn retention_keeps_only_n_backups() {
    let (_tmp, out_dir, backup_dir) = make_temp_dirs();
    run_gen_biscuit_keys_multiple_backups(out_dir.to_string_lossy().as_ref(), backup_dir.to_string_lossy().as_ref(), NUM_BACKUPS_TO_GENERATE);

    run_cargo_gen_biscuit_keys(&["--format", "files", "--out-dir", out_dir.to_string_lossy().as_ref(), "--backup", "--backup-dir", backup_dir.to_string_lossy().as_ref(), "--max-backups", "2", "--force"]);

    let priv_backups = count_backups_with_prefix(&backup_dir, "biscuit_private.b64.bak.");
    assert!(priv_backups <= 2, "expected at most 2 private backups, found {}", priv_backups);
}
