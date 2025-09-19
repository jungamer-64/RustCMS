use flate2::{Compression, write::GzEncoder};
use std::fs;
use std::io::Write;
use std::path::Path;

pub(crate) fn timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since = start.duration_since(UNIX_EPOCH).unwrap_or_default();
    since.as_secs().to_string()
}

pub(crate) fn should_backup(path: &Path, backup: bool) -> bool {
    backup && path.exists()
}

pub(crate) fn make_backup_destination(path: &Path, backup_dir: Option<&Path>) -> (std::path::PathBuf, String) {
    let ts = timestamp();
    let file_name = path
        .file_name()
    .map_or_else(|| "backup".to_string(), |s| s.to_string_lossy().into_owned());
    let bak_name = format!("{file_name}.bak.{ts}");
    let bak = if let Some(dir) = backup_dir {
        dir.join(bak_name)
    } else {
        path.with_file_name(bak_name)
    };
    (bak, file_name)
}

pub(crate) fn ensure_parent_dir(parent: Option<&Path>) {
    if let Some(parent) = parent && let Err(e) = fs::create_dir_all(parent) {
        eprintln!("Failed to create backup dir {}: {e}", parent.display());
    }
}

pub(crate) fn perform_backup(path: &Path, bak: &Path) -> std::io::Result<()> {
    if matches!(fs::rename(path, bak), Ok(())) {
        println!("Backed up {} -> {}", path.display(), bak.display());
        Ok(())
    } else {
        fs::copy(path, bak)?;
        println!("Backed up (copied) {} -> {}", path.display(), bak.display());
        Ok(())
    }
}

pub(crate) fn compress_file(path: &Path) -> std::io::Result<()> {
    let data = fs::read(path)?;
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("backup");
    let gz_name = format!("{file_name}.gz");
    let gz_path = path.with_file_name(gz_name);
    let f = fs::File::create(&gz_path)?;
    let mut encoder = GzEncoder::new(f, Compression::default());
    encoder.write_all(&data)?;
    encoder.finish()?;
    fs::remove_file(path)?;
    println!("Compressed backup to {}", gz_path.display());
    Ok(())
}

pub(crate) fn run_post_backup(bak_path: &Path, file_name: &str, max_backups: Option<usize>, compress_opt: Option<bool>) {
    enforce_retention_if_needed(bak_path, file_name, max_backups);
    compress_if_requested(bak_path, compress_opt);
}

pub(crate) fn enforce_retention_if_needed(bak_path: &Path, file_name: &str, max_backups: Option<usize>) {
    if let Some(n) = max_backups && n > 0 {
        if let Some(parent) = bak_path.parent() {
            if let Err(e) = enforce_backup_retention(parent, file_name, n) {
                eprintln!("Failed to enforce backup retention for {file_name}: {e}");
            }
        }
    }
}

pub(crate) fn compress_if_requested(bak_path: &Path, compress_opt: Option<bool>) {
    if compress_opt == Some(true) {
        if let Err(e) = compress_file(bak_path) {
            eprintln!("Failed to compress backup {}: {}", bak_path.display(), e);
        }
    }
}

pub(crate) fn enforce_backup_retention(dir: &Path, file_name: &str, keep: usize) -> std::io::Result<()> {
    let entries = collect_backup_entries(dir, file_name)?;
    prune_backup_entries(entries, keep);
    Ok(())
}

pub(crate) fn prune_backup_entries(mut entries: Vec<(u64, std::path::PathBuf)>, keep: usize) {
    if keep == 0 || entries.len() <= keep {
        return;
    }
    entries.sort_by(|a, b| b.0.cmp(&a.0));
    delete_old_backups(entries.into_iter().skip(keep));
}

pub(crate) fn delete_old_backups<I: Iterator<Item = (u64, std::path::PathBuf)>>(iter: I) {
    for (_ts, path) in iter {
        if let Err(e) = fs::remove_file(&path) {
            eprintln!("Failed to remove old backup {}: {e}", path.display());
        } else {
            println!("Removed old backup {}", path.display());
        }
    }
}

pub(crate) fn collect_backup_entries(dir: &Path, file_name: &str) -> std::io::Result<Vec<(u64, std::path::PathBuf)>> {
    let mut entries: Vec<(u64, std::path::PathBuf)> = Vec::new();
    if !dir.exists() {
        return Ok(entries);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if let Some(name_os) = p.file_name() {
            let name = name_os.to_string_lossy();
            if is_backup_name(&name, file_name) {
                if let Some(ts) = parse_backup_ts(&name) {
                    entries.push((ts, p.clone()));
                }
            }
        }
    }
    Ok(entries)
}

pub(crate) fn is_backup_name(name: &str, file_name: &str) -> bool {
    name.starts_with(file_name) && name.contains(".bak.")
}

pub(crate) fn parse_backup_ts(name: &str) -> Option<u64> {
    extract_ts_from_backup_name(name)
}

pub(crate) fn extract_ts_from_backup_name(name: &str) -> Option<u64> {
    if let Some(idx) = name.rfind('.') {
        let ts_str = &name[idx + 1..];
        if let Ok(ts) = ts_str.parse::<u64>() {
            return Some(ts);
        }
    }
    None
}

/// Convenience high-level maybe_backup_file used by the binary.
pub(crate) fn maybe_backup_file(path: &Path, backup: bool, backup_dir: Option<&Path>, max_backups: Option<usize>, compress_opt: Option<bool>) -> std::io::Result<()> {
    // Fast-path: nothing to do
    if !should_backup(path, backup) {
        return Ok(());
    }

    // Prepare destination for backup
    let (bak, file_name) = make_backup_destination(path, backup_dir);
    prepare_backup_parent(&bak);

    // Execute backup and post actions
    do_backup(path, &bak)?;
    post_backup_actions(&bak, &file_name, max_backups, compress_opt);
    Ok(())
}

fn prepare_backup_parent(bak: &Path) {
    if let Some(parent) = bak.parent() {
        ensure_parent_dir(Some(parent));
    }
}

fn do_backup(path: &Path, bak: &Path) -> std::io::Result<()> {
    perform_backup(path, bak)
}

fn post_backup_actions(bak: &Path, file_name: &str, max_backups: Option<usize>, compress_opt: Option<bool>) {
    run_post_backup(bak, file_name, max_backups, compress_opt);
}
