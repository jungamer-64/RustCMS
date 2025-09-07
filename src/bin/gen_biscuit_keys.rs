use biscuit_auth::KeyPair;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use std::fs;
use std::io::Write;
use flate2::{write::GzEncoder, Compression};
use std::path::Path;
use std::path::PathBuf;
use clap::{Parser, ValueEnum};
// use crate when available (binary is in the same crate)
use cms_backend::utils::hash;
use serde::Serialize;

// Extract version number from filenames like biscuit_private_v12.b64
fn parse_version(name: &str) -> Option<u32> {
    if let Some(idx) = name.rfind("_v") {
        // expect pattern *_v<digits>.b64
        let ver_part = &name[idx+2..];
        if let Some(dot) = ver_part.find('.') { // remove extension
            let digits = &ver_part[..dot];
            return digits.parse().ok();
        }
    }
    None
}

fn next_version(dir: &Path) -> Option<u32> {
    let mut max_v: u32 = 0;
    if dir.exists() {
        if let Ok(read) = fs::read_dir(dir) {
            for entry in read.flatten() {
                let name = entry.file_name();
                if let Some(s) = name.to_str() {
                    if let Some(v) = parse_version(s) { if v > max_v { max_v = v; } }
                }
            }
        }
    }
    Some(max_v + 1)
}

// replaced by utils::hash::sha256_hex

#[derive(Serialize)]
struct Manifest<'a> {
    latest_version: u32,
    generated_at: String,
    private_fingerprint: &'a str,
    public_fingerprint: &'a str,
}

fn update_manifest(dir: &Path, version: u32, priv_fp: &str, pub_fp: &str) {
    let manifest_path = dir.join("manifest.json");
    let manifest = Manifest {
        latest_version: version,
        generated_at: chrono::Utc::now().to_rfc3339(),
        private_fingerprint: priv_fp,
        public_fingerprint: pub_fp,
    };
    if let Ok(json) = serde_json::to_string_pretty(&manifest) {
        if let Err(e) = fs::write(&manifest_path, json) {
            eprintln!("Failed to write manifest {}: {}", manifest_path.display(), e);
        } else {
            println!("Updated manifest at {}", manifest_path.display());
        }
    }
}

fn prune_versions(dir: &Path, keep: usize) {
    if keep == 0 { return; }
    let mut versions: Vec<u32> = Vec::new();
    if let Ok(read) = fs::read_dir(dir) {
        for entry in read.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("biscuit_private_v") && name.ends_with(".b64") {
                    if let Some(v) = parse_version(name) { versions.push(v); }
                }
            }
        }
    }
    if versions.len() <= keep { return; }
    versions.sort_unstable(); // ascending
    let to_remove: Vec<u32> = versions.iter().cloned().take(versions.len() - keep).collect();
    for v in to_remove {
        for prefix in ["biscuit_private_v", "biscuit_public_v"] { 
            let path = dir.join(format!("{}{}.b64", prefix, v));
            if path.exists() {
                if let Err(e) = fs::remove_file(&path) { eprintln!("Failed to remove old version {}: {}", path.display(), e); }
                else { println!("Pruned old version file {}", path.display()); }
            }
        }
    }
}

fn write_file_if_allowed(path: &Path, data: &str, force: bool) -> std::io::Result<()> {
    if path.exists() && !force {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("{} already exists", path.display()),
        ));
    }
    let mut f = fs::File::create(path)?;
    f.write_all(data.as_bytes())?;
    Ok(())
}

fn append_env_file(path: &Path, priv_b64: &str, pub_b64: &str, force: bool) -> std::io::Result<()> {
    // If file doesn't exist, create and write header
    let mut create = false;
    if !path.exists() {
        create = true;
    }

    if create {
        let mut f = fs::File::create(path)?;
        writeln!(f, "# Generated biscuit keys")?;
        writeln!(f, "BISCUIT_PRIVATE_KEY_B64={}", priv_b64)?;
        writeln!(f, "BISCUIT_PUBLIC_KEY_B64={}", pub_b64)?;
        return Ok(());
    }

    // If exists, check if it already contains the variables
    let content = fs::read_to_string(path)?;
    if content.contains("BISCUIT_PRIVATE_KEY_B64=") || content.contains("BISCUIT_PUBLIC_KEY_B64=") {
        if !force {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("{} already contains biscuit entries", path.display()),
            ));
        }
        // Remove existing lines and append fresh ones
        let filtered: Vec<&str> = content
            .lines()
            .filter(|l| !l.starts_with("BISCUIT_PRIVATE_KEY_B64=") && !l.starts_with("BISCUIT_PUBLIC_KEY_B64="))
            .collect();
        let mut f = fs::File::create(path)?;
        for line in filtered {
            writeln!(f, "{}", line)?;
        }
        writeln!(f, "BISCUIT_PRIVATE_KEY_B64={}", priv_b64)?;
        writeln!(f, "BISCUIT_PUBLIC_KEY_B64={}", pub_b64)?;
        return Ok(());
    } else {
        let mut f = fs::OpenOptions::new().append(true).open(path)?;
        writeln!(f, "\n# Generated biscuit keys")?;
        writeln!(f, "BISCUIT_PRIVATE_KEY_B64={}", priv_b64)?;
        writeln!(f, "BISCUIT_PUBLIC_KEY_B64={}", pub_b64)?;
        return Ok(());
    }
}

fn report_write_file_result(path: &Path, res: std::io::Result<()>, label: &str, force: bool) {
    match res {
        Ok(_) => println!("Wrote {} to {}", label, path.display()),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AlreadyExists && !force {
                eprintln!("{} already exists at {}. Use --force to overwrite.", label, path.display());
            } else {
                eprintln!("Failed to write {}: {}", label, e);
            }
        }
    }
}

fn report_env_result(envfile: &str, res: std::io::Result<()>, force: bool) {
    match res {
        Ok(_) => println!("Written keys into {}", envfile),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AlreadyExists && !force {
                eprintln!("{} already contains biscuit entries. Use --force to overwrite or choose another env file.", envfile);
            } else {
                eprintln!("Failed to write env file {}: {}", envfile, e);
            }
        }
    }
}


fn timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since = start.duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{}", since.as_secs())
}

fn maybe_backup_file(path: &Path, backup: bool, backup_dir: Option<&Path>, _max_backups: Option<usize>, _compress: Option<bool>) -> std::io::Result<()> {
    if !backup { return Ok(()); }
    if !path.exists() { return Ok(()); }
    let ts = timestamp();
    let file_name = path.file_name().map(|s| s.to_string_lossy()).unwrap_or_else(|| "backup".into());
    let bak_name = format!("{}.bak.{}", file_name, ts);
    let bak = if let Some(dir) = backup_dir {
        dir.join(bak_name)
    } else {
        path.with_file_name(bak_name)
    };
    // Ensure backup dir exists
    if let Some(parent) = bak.parent() {
        if let Err(e) = fs::create_dir_all(parent) { eprintln!("Failed to create backup dir {}: {}", parent.display(), e); }
    }
    // Prefer rename (move). If that fails (cross-device), fall back to copy.
    match fs::rename(path, &bak) {
        Ok(_) => {
            println!("Backed up {} -> {}", path.display(), bak.display());
            if let Some(n) = _max_backups {
                if n > 0 {
                    if let Some(parent) = bak.parent() {
                        if let Err(e) = enforce_backup_retention(parent, &file_name, n) {
                            eprintln!("Failed to enforce backup retention for {}: {}", file_name, e);
                        }
                    }
                }
            }
            if let Some(true) = _compress {
                if let Err(e) = compress_file(&bak) {
                    eprintln!("Failed to compress backup {}: {}", bak.display(), e);
                }
            }
            return Ok(());
        }
        Err(_) => {
            fs::copy(path, &bak)?;
            println!("Backed up (copied) {} -> {}", path.display(), bak.display());
            if let Some(n) = _max_backups {
                if n > 0 {
                    if let Some(parent) = bak.parent() {
                        if let Err(e) = enforce_backup_retention(parent, &file_name, n) {
                            eprintln!("Failed to enforce backup retention for {}: {}", file_name, e);
                        }
                    }
                }
            }
            if let Some(true) = _compress {
                if let Err(e) = compress_file(&bak) {
                    eprintln!("Failed to compress backup {}: {}", bak.display(), e);
                }
            }
            return Ok(());
        }
    }
}

fn compress_file(path: &Path) -> std::io::Result<()> {
    let data = fs::read(path)?;
    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("backup");
    let gz_name = format!("{}.gz", file_name);
    let gz_path = path.with_file_name(gz_name);
    let f = fs::File::create(&gz_path)?;
    let mut encoder = GzEncoder::new(f, Compression::default());
    encoder.write_all(&data)?;
    encoder.finish()?;
    // Optionally remove original uncompressed file
    fs::remove_file(path)?;
    println!("Compressed backup to {}", gz_path.display());
    Ok(())
}

fn maybe_backup_env(path: &Path, backup: bool, backup_dir: Option<&Path>, max_backups: Option<usize>, compress: Option<bool>) -> std::io::Result<()> {
    maybe_backup_file(path, backup, backup_dir, max_backups, compress)
}

fn enforce_backup_retention(dir: &Path, file_name: &str, keep: usize) -> std::io::Result<()> {
    // Collect backup files that match pattern: <file_name>.bak.<ts>
    let mut entries: Vec<(u64, std::path::PathBuf)> = Vec::new();
    if !dir.exists() { return Ok(()); }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if let Some(name_os) = p.file_name() {
            let name = name_os.to_string_lossy();
            // match prefix
            if name.starts_with(file_name) && name.contains(".bak.") {
                // try to parse timestamp as the suffix after last dot
                if let Some(idx) = name.rfind('.') {
                    let ts_str = &name[idx+1..];
                    if let Ok(ts) = ts_str.parse::<u64>() {
                        entries.push((ts, p.clone()));
                    }
                }
            }
        }
    }
    // If number of backups <= keep, nothing to do
    if entries.len() <= keep { return Ok(()); }
    // Sort descending by timestamp (newest first)
    entries.sort_by(|a,b| b.0.cmp(&a.0));
    // Keep the first `keep` entries, remove the rest
    for (_ts, path) in entries.into_iter().skip(keep) {
        if let Err(e) = fs::remove_file(&path) {
            eprintln!("Failed to remove old backup {}: {}", path.display(), e);
        } else {
            println!("Removed old backup {}", path.display());
        }
    }
    Ok(())
}

// clap-based argument parsing is used; helper suggestion/levenshtein removed.

#[derive(Parser)]
#[command(author, version, about = "Generate biscuit keypair (base64) and optionally write to files/.env")] 
struct Args {
    /// Write biscuit_private.b64 and biscuit_public.b64 into <dir>
    #[arg(long, value_name = "DIR")]
    out_dir: Option<PathBuf>,

    /// Append BISCUIT_PRIVATE_KEY_B64 / BISCUIT_PUBLIC_KEY_B64 to <file> (default: none)
    #[arg(long, value_name = "FILE")]
    env_file: Option<PathBuf>,

    /// Explicit output target. If omitted, defaults to previous behavior (use provided targets).
    #[arg(long, value_enum)]
    format: Option<OutputFormat>,

    /// Overwrite existing files / env entries when present
    #[arg(long)]
    force: bool,

    /// Move existing files/env to a timestamped .bak before writing
    #[arg(long)]
    backup: bool,
    /// Directory to store backups (default: same directory as target)
    #[arg(long, value_name = "DIR")]
    backup_dir: Option<PathBuf>,
    /// Maximum number of backups to keep per target (older backups will be removed). 0 means unlimited
    #[arg(long, value_name = "N")]
    max_backups: Option<usize>,
    /// Compress created backups with gzip (removes uncompressed backup file)
    #[arg(long)]
    backup_compress: bool,

    /// Save as versioned files (biscuit_private_vN.b64) and keep latest unversioned copy
    #[arg(long)]
    versioned: bool,

    /// List existing versioned keys and exit
    #[arg(long)]
    list: bool,

    /// When used with --versioned, also write/update unversioned biscuit_private.b64 / biscuit_public.b64 as alias to latest
    #[arg(long)]
    latest_alias: bool,

    /// Keep only the newest N versioned key sets (applies after writing). 0 disables pruning.
    #[arg(long, value_name="N")]
    prune: Option<usize>,

    /// Skip manifest.json update
    #[arg(long)]
    no_manifest: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum OutputFormat {
    Files,
    Env,
    Both,
    Stdout,
}

fn main() {
    let args = Args::parse();
    let out_dir: Option<String> = args.out_dir.as_ref().map(|p| p.to_string_lossy().to_string());
    let env_file: Option<String> = args.env_file.as_ref().map(|p| p.to_string_lossy().to_string());
    let format = args.format.map(|f| match f {
        OutputFormat::Files => "files".to_string(),
        OutputFormat::Env => "env".to_string(),
        OutputFormat::Both => "both".to_string(),
        OutputFormat::Stdout => "stdout".to_string(),
    });
    let force = args.force;
    let backup = args.backup;

    if args.list {
        let dir = out_dir.as_deref().unwrap_or("keys");
        let path = Path::new(dir);
        match fs::read_dir(path) {
            Ok(read) => {
                let mut versions: Vec<(u32, String)> = Vec::new();
                for entry in read.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if let Some(v) = parse_version(&name) { versions.push((v, name)); }
                }
                if versions.is_empty() {
                    println!("No versioned keys found in {}", path.display());
                } else {
                    versions.sort_by_key(|x| x.0);
                    println!("Found versions (oldest -> newest):");
                    for (v, name) in versions { println!("v{} => {}", v, name); }
                }
            }
            Err(e) => eprintln!("Cannot read directory {}: {}", path.display(), e),
        }
        return;
    }

    let kp = KeyPair::new();
    let priv_b64 = STANDARD.encode(kp.private().to_bytes());
    let pub_b64 = STANDARD.encode(kp.public().to_bytes());

    println!("# Generated biscuit keypair (base64)");
    println!("BISCUIT_PRIVATE_KEY_B64={}", priv_b64);
    println!("BISCUIT_PUBLIC_KEY_B64={}", pub_b64);

    // Decide outputs based on explicit format or provided targets
    let format = format.map(|s| s.to_ascii_lowercase());

    if let Some(ref f) = format {
        if f == "files" {
            let dir = out_dir.as_deref().unwrap_or("keys");
            let path = Path::new(dir);
            if let Err(e) = fs::create_dir_all(path) {
                eprintln!("Failed to create out-dir {}: {}", dir, e);
                std::process::exit(1);
            }
            let (priv_path, pub_path) = if args.versioned {
                let v = next_version(path).unwrap_or(1);
                (path.join(format!("biscuit_private_v{}.b64", v)), path.join(format!("biscuit_public_v{}.b64", v)))
            } else {
                (path.join("biscuit_private.b64"), path.join("biscuit_public.b64"))
            };

                if let Err(e) = maybe_backup_file(&priv_path, backup, args.backup_dir.as_deref(), args.max_backups, Some(args.backup_compress)) { eprintln!("Backup failed: {}", e); }
                if let Err(e) = maybe_backup_file(&pub_path, backup, args.backup_dir.as_deref(), args.max_backups, Some(args.backup_compress)) { eprintln!("Backup failed: {}", e); }
                report_write_file_result(&priv_path, write_file_if_allowed(&priv_path, &priv_b64, force), "private key", force);
                report_write_file_result(&pub_path, write_file_if_allowed(&pub_path, &pub_b64, force), "public key", force);

            // latest alias update
            if args.versioned && args.latest_alias {
                let latest_priv = path.join("biscuit_private.b64");
                let latest_pub = path.join("biscuit_public.b64");
                if let Err(e) = fs::write(&latest_priv, &priv_b64) { eprintln!("Failed to update latest alias {}: {}", latest_priv.display(), e); }
                if let Err(e) = fs::write(&latest_pub, &pub_b64) { eprintln!("Failed to update latest alias {}: {}", latest_pub.display(), e); }
            }

            if args.versioned {
                let v = parse_version(priv_path.file_name().unwrap().to_string_lossy().as_ref()).unwrap_or(0);
                let priv_fp = hash::sha256_hex(priv_b64.as_bytes());
                let pub_fp = hash::sha256_hex(pub_b64.as_bytes());
                println!("private_fingerprint_sha256={} public_fingerprint_sha256={}", priv_fp, pub_fp);
                if !args.no_manifest { update_manifest(path, v, &priv_fp, &pub_fp); }
                if let Some(keep) = args.prune { prune_versions(path, keep); }
            }
        } else if f == "env" {
            let envfile = env_file.as_deref().unwrap_or(".env");
            let path = Path::new(envfile);
            if let Err(e) = maybe_backup_env(path, backup, args.backup_dir.as_deref(), args.max_backups, Some(args.backup_compress)) { eprintln!("Backup failed: {}", e); }
            report_env_result(envfile, append_env_file(path, &priv_b64, &pub_b64, force), force);
        } else if f == "both" {
            let dir = out_dir.as_deref().unwrap_or("keys");
            let envfile = env_file.as_deref().unwrap_or(".env");
            let path = Path::new(dir);
            if let Err(e) = fs::create_dir_all(path) {
                eprintln!("Failed to create out-dir {}: {}", dir, e);
                std::process::exit(1);
            }
            let (priv_path, pub_path) = if args.versioned {
                let v = next_version(path).unwrap_or(1);
                (path.join(format!("biscuit_private_v{}.b64", v)), path.join(format!("biscuit_public_v{}.b64", v)))
            } else {
                (path.join("biscuit_private.b64"), path.join("biscuit_public.b64"))
            };

                if let Err(e) = maybe_backup_file(&priv_path, backup, args.backup_dir.as_deref(), args.max_backups, Some(args.backup_compress)) { eprintln!("Backup failed: {}", e); }
                if let Err(e) = maybe_backup_file(&pub_path, backup, args.backup_dir.as_deref(), args.max_backups, Some(args.backup_compress)) { eprintln!("Backup failed: {}", e); }
                report_write_file_result(&priv_path, write_file_if_allowed(&priv_path, &priv_b64, force), "private key", force);
                report_write_file_result(&pub_path, write_file_if_allowed(&pub_path, &pub_b64, force), "public key", force);

            if args.versioned && args.latest_alias {
                let latest_priv = path.join("biscuit_private.b64");
                let latest_pub = path.join("biscuit_public.b64");
                if let Err(e) = fs::write(&latest_priv, &priv_b64) { eprintln!("Failed to update latest alias {}: {}", latest_priv.display(), e); }
                if let Err(e) = fs::write(&latest_pub, &pub_b64) { eprintln!("Failed to update latest alias {}: {}", latest_pub.display(), e); }
            }

            if args.versioned {
                let v = parse_version(priv_path.file_name().unwrap().to_string_lossy().as_ref()).unwrap_or(0);
                let priv_fp = hash::sha256_hex(priv_b64.as_bytes());
                let pub_fp = hash::sha256_hex(pub_b64.as_bytes());
                println!("private_fingerprint_sha256={} public_fingerprint_sha256={}", priv_fp, pub_fp);
                if !args.no_manifest { update_manifest(path, v, &priv_fp, &pub_fp); }
                if let Some(keep) = args.prune { prune_versions(path, keep); }
            }

            let envpath = Path::new(envfile);
                if let Err(e) = maybe_backup_env(envpath, backup, args.backup_dir.as_deref(), args.max_backups, Some(args.backup_compress)) { eprintln!("Backup failed: {}", e); }
                report_env_result(envfile, append_env_file(envpath, &priv_b64, &pub_b64, force), force);
        } else if f == "stdout" {
            // explicit stdout only: already printed
        }
    } else {
        // no explicit format: previous behavior - write if targets provided
        if let Some(ref dir) = out_dir {
            let path = Path::new(&dir);
            if let Err(e) = fs::create_dir_all(path) {
                eprintln!("Failed to create out-dir {}: {}", dir, e);
                std::process::exit(1);
            }
            let (priv_path, pub_path) = if args.versioned {
                let v = next_version(path).unwrap_or(1);
                (path.join(format!("biscuit_private_v{}.b64", v)), path.join(format!("biscuit_public_v{}.b64", v)))
            } else {
                (path.join("biscuit_private.b64"), path.join("biscuit_public.b64"))
            };

            if let Err(e) = maybe_backup_file(&priv_path, backup, args.backup_dir.as_deref(), args.max_backups, Some(args.backup_compress)) { eprintln!("Backup failed: {}", e); }
            if let Err(e) = maybe_backup_file(&pub_path, backup, args.backup_dir.as_deref(), args.max_backups, Some(args.backup_compress)) { eprintln!("Backup failed: {}", e); }
            report_write_file_result(&priv_path, write_file_if_allowed(&priv_path, &priv_b64, force), "private key", force);
            report_write_file_result(&pub_path, write_file_if_allowed(&pub_path, &pub_b64, force), "public key", force);

            if args.versioned && args.latest_alias {
                let latest_priv = path.join("biscuit_private.b64");
                let latest_pub = path.join("biscuit_public.b64");
                if let Err(e) = fs::write(&latest_priv, &priv_b64) { eprintln!("Failed to update latest alias {}: {}", latest_priv.display(), e); }
                if let Err(e) = fs::write(&latest_pub, &pub_b64) { eprintln!("Failed to update latest alias {}: {}", latest_pub.display(), e); }
            }

            if args.versioned {
                let v = parse_version(priv_path.file_name().unwrap().to_string_lossy().as_ref()).unwrap_or(0);
                let priv_fp = hash::sha256_hex(priv_b64.as_bytes());
                let pub_fp = hash::sha256_hex(pub_b64.as_bytes());
                println!("private_fingerprint_sha256={} public_fingerprint_sha256={}", priv_fp, pub_fp);
                if !args.no_manifest { update_manifest(path, v, &priv_fp, &pub_fp); }
                if let Some(keep) = args.prune { prune_versions(path, keep); }
            }
        }

        if let Some(ref envfile) = env_file {
            let path = Path::new(envfile);
            if let Err(e) = maybe_backup_env(path, backup, args.backup_dir.as_deref(), args.max_backups, Some(args.backup_compress)) { eprintln!("Backup failed: {}", e); }
            report_env_result(envfile, append_env_file(path, &priv_b64, &pub_b64, force), force);
        }
    }

    // If nothing was written and no explicit format is stdout, notify user
    if format.is_none() && out_dir.is_none() && env_file.is_none() {
        println!("No output target specified; keys were only printed to stdout.");
    }
}
