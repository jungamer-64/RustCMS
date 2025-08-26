# Changelog

All notable changes to this repository will be documented in this file.

## Unreleased

### Added

- `gen_biscuit_keys`:
  - Add `--max-backups` to limit the number of stored backups (retention policy).
  - Add `--backup-compress` to gzip-compress created backups and remove uncompressed files.
  - Add `--backup-dir` to specify backup destination.
  - Implement `enforce_backup_retention` to remove old backups beyond configured limit.
  - Implement `compress_file` for gzip compression of backups.
  - Add integration tests for retention and compression.

### CI

- Add GitHub Actions workflow to run `cargo build` and `cargo test` on push/PR to `main`.

