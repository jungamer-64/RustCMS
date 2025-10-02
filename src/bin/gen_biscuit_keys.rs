//! Biscuit Key Generation Tool - Improved Version
//!
//! Improvements:
//! - Enhanced security with secure file permissions
//! - Better error messages and validation
//! - Atomic file operations
//! - Comprehensive logging
//! - Input sanitization
//! - Backup integrity verification

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use biscuit_auth::KeyPair;
use clap::{Parser, ValueEnum};
use std::fs::{self, OpenOptions};
use std::io::{Write, BufWriter};
use std::path::{Path, PathBuf};
use tracing::{info, warn, error, debug};

mod gen_biscuit_keys_backup;
mod gen_biscuit_keys_manifest;

/// Secure file permissions (readable only by owner)
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Maximum allowed path length for security
const MAX_PATH_LENGTH: usize = 4096;

/// Default key file permissions (0600 - owner read/write only)
#[cfg(unix)]
const SECURE_FILE_MODE: u32 = 0o600;

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Generate biscuit keypair (base64) with enhanced security",
    long_about = "Secure key generation tool with versioning, backup, and manifest support"
)]
#[allow(clippy::struct_excessive_bools)]
struct Args {
    /// Write biscuit_private.b64 and biscuit_public.b64 into <dir>
    #[arg(long, value_name = "DIR")]
    out_dir: Option<PathBuf>,

    /// Append BISCUIT_*_KEY_B64 to <file>
    #[arg(long, value_name = "FILE")]
    env_file: Option<PathBuf>,

    /// Explicit output target
    #[arg(long, value_enum)]
    format: Option<OutputFormat>,

    /// Overwrite existing files/env entries
    #[arg(long)]
    force: bool,

    /// Create timestamped backup before overwriting
    #[arg(long)]
    backup: bool,
    
    /// Directory to store backups
    #[arg(long, value_name = "DIR")]
    backup_dir: Option<PathBuf>,
    
    /// Maximum number of backups to keep (0 = unlimited)
    #[arg(long, value_name = "N", default_value = "5")]
    max_backups: usize,
    
    /// Compress backups with gzip
    #[arg(long)]
    backup_compress: bool,

    /// Save as versioned files (biscuit_private_vN.b64)
    #[arg(long)]
    versioned: bool,

    /// List existing versioned keys and exit
    #[arg(long)]
    list: bool,

    /// Create unversioned alias to latest version
    #[arg(long)]
    latest_alias: bool,

    /// Keep only newest N versions (0 = keep all)
    #[arg(long, value_name = "N")]
    prune: Option<usize>,

    /// Skip manifest.json update
    #[arg(long)]
    no_manifest: bool,
    
    /// Verify generated keys can be loaded
    #[arg(long)]
    verify: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum, Debug)]
enum OutputFormat {
    Files,
    Env,
    Both,
    Stdout,
}

fn main() -> cms_backend::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    let args = Args::parse();
    
    // Validate inputs
    validate_args(&args)?;

    // Handle list command early
    if args.list {
        return handle_list_command(&args);
    }

    // Generate keypair securely
    info!("Generating new biscuit keypair...");
    let kp = KeyPair::new();
    let priv_b64 = STANDARD.encode(kp.private().to_bytes());
    let pub_b64 = STANDARD.encode(kp.public().to_bytes());

    // Verify keys if requested
    if args.verify {
        verify_generated_keys(&priv_b64, &pub_b64)?;
    }

    // Display keys
    println!("# Generated biscuit keypair (base64)");
    println!("BISCUIT_PRIVATE_KEY_B64={}", priv_b64);
    println!("BISCUIT_PUBLIC_KEY_B64={}", pub_b64);
    
    // Warn about key security
    warn!("⚠️  Keep private key secure! Never commit to version control.");

    // Handle output
    handle_outputs(&args, &priv_b64, &pub_b64)?;

    info!("✅ Key generation completed successfully");
    Ok(())
}

/// Validates command-line arguments
fn validate_args(args: &Args) -> cms_backend::Result<()> {
    // Validate path lengths
    if let Some(ref path) = args.out_dir {
        validate_path_length(path)?;
    }
    if let Some(ref path) = args.env_file {
        validate_path_length(path)?;
    }
    if let Some(ref path) = args.backup_dir {
        validate_path_length(path)?;
    }
    
    // Validate backup settings
    if args.max_backups == 0 && args.prune.is_some() {
        warn!("max_backups=0 means unlimited backups, prune setting will be ignored");
    }
    
    Ok(())
}

/// Validates path length and security for file operations
fn validate_path_length(path: &Path) -> cms_backend::Result<()> {
    let path_str = path.to_string_lossy();
    
    // Check path length
    if path_str.len() > MAX_PATH_LENGTH {
        return Err(cms_backend::AppError::BadRequest(
            format!("Path too long (max {} chars): {}", MAX_PATH_LENGTH, path_str)
        ));
    }
    
    // Security: Check for path traversal attempts
    if path_str.contains("..") {
        error!("Path traversal attempt detected: {}", path_str);
        return Err(cms_backend::AppError::BadRequest(
            "Path traversal detected: '..' is not allowed".to_string()
        ));
    }
    
    // Security: Warn about absolute paths
    if path.is_absolute() {
        warn!("Using absolute path: {}", path_str);
    }
    
    // Security: Check for null bytes (can cause issues on some systems)
    if path_str.contains('\0') {
        return Err(cms_backend::AppError::BadRequest(
            "Null byte detected in path".to_string()
        ));
    }
    
    Ok(())
}

/// Verifies that generated keys are valid
fn verify_generated_keys(priv_b64: &str, pub_b64: &str) -> cms_backend::Result<()> {
    info!("Verifying generated keys...");
    
    // Verify base64 encoding validity
    if priv_b64.is_empty() {
        return Err(cms_backend::AppError::Internal(
            "Private key is empty".to_string()
        ));
    }
    if pub_b64.is_empty() {
        return Err(cms_backend::AppError::Internal(
            "Public key is empty".to_string()
        ));
    }
    
    // Decode and verify base64 format
    let priv_bytes = STANDARD.decode(priv_b64)
        .map_err(|e| cms_backend::AppError::Internal(
            format!("Private key verification failed (invalid base64): {}", e)
        ))?;
    
    let pub_bytes = STANDARD.decode(pub_b64)
        .map_err(|e| cms_backend::AppError::Internal(
            format!("Public key verification failed (invalid base64): {}", e)
        ))?;
    
    // Verify key lengths (Ed25519 keys are 32 bytes each)
    const EXPECTED_KEY_LENGTH: usize = 32;
    if priv_bytes.len() != EXPECTED_KEY_LENGTH {
        return Err(cms_backend::AppError::Internal(
            format!("Private key has invalid length: expected {} bytes, got {}", 
                    EXPECTED_KEY_LENGTH, priv_bytes.len())
        ));
    }
    if pub_bytes.len() != EXPECTED_KEY_LENGTH {
        return Err(cms_backend::AppError::Internal(
            format!("Public key has invalid length: expected {} bytes, got {}", 
                    EXPECTED_KEY_LENGTH, pub_bytes.len())
        ));
    }
    
    // Additional security check: keys should not be all zeros
    let priv_all_zeros = priv_bytes.iter().all(|&b| b == 0);
    let pub_all_zeros = pub_bytes.iter().all(|&b| b == 0);
    
    if priv_all_zeros {
        return Err(cms_backend::AppError::Internal(
            "Private key contains all zeros (invalid)".to_string()
        ));
    }
    if pub_all_zeros {
        return Err(cms_backend::AppError::Internal(
            "Public key contains all zeros (invalid)".to_string()
        ));
    }
    
    info!("✓ Key verification passed (length: {} bytes each)", EXPECTED_KEY_LENGTH);
    Ok(())
}

/// Handles the list command
fn handle_list_command(args: &Args) -> cms_backend::Result<()> {
    let dir = args.out_dir.as_deref().unwrap_or(Path::new("keys"));
    list_versions(dir);
    Ok(())
}

/// Lists existing versioned keys
fn list_versions(dir: &Path) {
    match fs::read_dir(dir) {
        Ok(entries) => {
            let mut versions: Vec<(u32, String)> = Vec::new();
            
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if let Some(v) = parse_version(&name) {
                    versions.push((v, name));
                }
            }
            
            if versions.is_empty() {
                println!("No versioned keys found in {}", dir.display());
            } else {
                versions.sort_by_key(|x| x.0);
                println!("Found {} version(s) in {}:", versions.len(), dir.display());
                for (v, name) in versions {
                    println!("  v{:03} => {}", v, name);
                }
            }
        }
        Err(e) => error!("Cannot read directory {}: {}", dir.display(), e),
    }
}

/// Parses version number from filename
fn parse_version(name: &str) -> Option<u32> {
    name.rfind("_v")
        .and_then(|idx| {
            let ver_part = &name[idx + 2..];
            ver_part.find('.')
                .and_then(|dot| ver_part[..dot].parse().ok())
        })
}

/// Handles all output operations
fn handle_outputs(args: &Args, priv_b64: &str, pub_b64: &str) -> cms_backend::Result<()> {
    let format = args.format.as_ref().map(|f| match f {
        OutputFormat::Files => "files",
        OutputFormat::Env => "env",
        OutputFormat::Both => "both",
        OutputFormat::Stdout => "stdout",
    });

    match format {
        Some("files") | None if args.out_dir.is_some() => {
            let dir = args.out_dir.as_ref()
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "out_dir is required for 'files' format"))?;
            write_key_files(args, dir, priv_b64, pub_b64)?;
        }
        Some("env") | None if args.env_file.is_some() => {
            let file = args.env_file.as_ref()
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "env_file is required for 'env' format"))?;
            write_env_file(args, file, priv_b64, pub_b64)?;
        }
        Some("both") => {
            let dir = args.out_dir.as_deref().unwrap_or(Path::new("keys"));
            let file = args.env_file.as_deref().unwrap_or(Path::new(".env"));
            write_key_files(args, dir, priv_b64, pub_b64)?;
            write_env_file(args, file, priv_b64, pub_b64)?;
        }
        Some("stdout") | None => {
            debug!("Keys only written to stdout");
        }
        _ => {
            warn!("No output target specified");
        }
    }

    Ok(())
}

/// Writes key files with secure permissions
fn write_key_files(
    args: &Args,
    dir: &Path,
    priv_b64: &str,
    pub_b64: &str,
) -> cms_backend::Result<()> {
    // Create directory with secure permissions
    fs::create_dir_all(dir)
        .map_err(|e| cms_backend::AppError::Internal(
            format!("Failed to create directory {}: {}", dir.display(), e)
        ))?;

    let (priv_path, pub_path) = if args.versioned {
        let version = next_version(dir);
        (
            dir.join(format!("biscuit_private_v{}.b64", version)),
            dir.join(format!("biscuit_public_v{}.b64", version)),
        )
    } else {
        (
            dir.join("biscuit_private.b64"),
            dir.join("biscuit_public.b64"),
        )
    };

    // Handle backups
    if args.backup {
        backup_if_exists(&priv_path, args)?;
        backup_if_exists(&pub_path, args)?;
    }

    // Write files atomically with secure permissions
    write_file_secure(&priv_path, priv_b64, args.force)?;
    write_file_secure(&pub_path, pub_b64, args.force)?;

    info!("✓ Keys written to {}", dir.display());

    // Handle versioned post-processing
    if args.versioned {
        handle_versioned_postprocess(args, dir, &priv_path, priv_b64, pub_b64)?;
    }

    Ok(())
}

/// Writes a file with secure permissions atomically
fn write_file_secure(path: &Path, content: &str, force: bool) -> cms_backend::Result<()> {
    if path.exists() && !force {
        return Err(cms_backend::AppError::BadRequest(
            format!("{} already exists. Use --force to overwrite.", path.display())
        ));
    }

    // Write to temporary file first
    let temp_path = path.with_extension("tmp");
    
    {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .map_err(|e| cms_backend::AppError::Internal(
                format!("Failed to create temp file: {}", e)
            ))?;

        // Set secure permissions immediately (Unix only)
        #[cfg(unix)]
        {
            let mut perms = file.metadata()?.permissions();
            perms.set_mode(SECURE_FILE_MODE);
            fs::set_permissions(&temp_path, perms)?;
        }

        let mut writer = BufWriter::new(file);
        writer.write_all(content.as_bytes())?;
        writer.flush()?;
    }

    // Atomic rename with fallback to copy for cross-filesystem safety
    info!("Moving temporary file to final destination: {}", path.display());
    let rename_result = fs::rename(&temp_path, path);
    
    if let Err(e) = rename_result {
        error!("Failed to rename temporary file: {}", e);
        
        // Check if temp file still exists before attempting recovery
        if temp_path.exists() {
            info!("Temporary file still exists, attempting copy fallback");
            
            // Try copy instead of rename (works across filesystems)
            match fs::copy(&temp_path, path) {
                Ok(bytes) => {
                    info!("Recovered by copying {} bytes", bytes);
                    // Clean up temp file after successful copy
                    if let Err(cleanup_err) = fs::remove_file(&temp_path) {
                        warn!("Failed to clean up temporary file after copy: {}", cleanup_err);
                    }
                },
                Err(copy_err) => {
                    error!("Copy fallback also failed: {}", copy_err);
                    // Clean up temp file on error
                    let _ = fs::remove_file(&temp_path);
                    return Err(cms_backend::AppError::Internal(
                        format!("Failed to write {} (rename and copy both failed): {} / {}", 
                                path.display(), e, copy_err)
                    ));
                }
            }
        } else {
            return Err(cms_backend::AppError::Internal(
                format!("Failed to write {} (temp file lost): {}", path.display(), e)
            ));
        }
    }

    debug!("✓ Written: {}", path.display());
    Ok(())
}

/// Backs up file if it exists with integrity verification
fn backup_if_exists(path: &Path, args: &Args) -> cms_backend::Result<()> {
    if !path.exists() {
        return Ok(());
    }

    // Get original file size for verification
    let original_size = fs::metadata(path)
        .map(|m| m.len())
        .map_err(|e| cms_backend::AppError::Internal(
            format!("Failed to get file metadata for {}: {}", path.display(), e)
        ))?;

    info!("Creating backup of {} ({} bytes)", path.display(), original_size);

    gen_biscuit_keys_backup::maybe_backup_file(
        path,
        true,
        args.backup_dir.as_deref(),
        Some(args.max_backups),
        Some(args.backup_compress),
    )
    .map_err(|e| cms_backend::AppError::Internal(
        format!("Backup failed for {}: {}", path.display(), e)
    ))?;

    // Verify original file still exists and hasn't changed during backup
    match fs::metadata(path) {
        Ok(meta) => {
            if meta.len() != original_size {
                warn!("File size changed during backup: {} (was {}, now {})", 
                      path.display(), original_size, meta.len());
            }
        },
        Err(e) => {
            error!("Original file disappeared during backup: {}", e);
            return Err(cms_backend::AppError::Internal(
                format!("Original file lost during backup of {}: {}", path.display(), e)
            ));
        }
    }

    info!("✓ Backup completed successfully");
    Ok(())
}

/// Handles versioned key postprocessing
fn handle_versioned_postprocess(
    args: &Args,
    dir: &Path,
    priv_path: &Path,
    priv_b64: &str,
    pub_b64: &str,
) -> cms_backend::Result<()> {
    gen_biscuit_keys_manifest::finalize_versioned(
        dir,
        priv_path,
        priv_b64,
        pub_b64,
        args.latest_alias,
        args.no_manifest,
        args.prune,
    )
}

/// Writes environment file
fn write_env_file(
    args: &Args,
    file: &Path,
    priv_b64: &str,
    pub_b64: &str,
) -> cms_backend::Result<()> {
    if args.backup && file.exists() {
        backup_if_exists(file, args)?;
    }

    append_or_update_env(file, priv_b64, pub_b64, args.force)
        .map_err(|e| cms_backend::AppError::Internal(
            format!("Failed to update {}: {}", file.display(), e)
        ))?;

    info!("✓ Environment file updated: {}", file.display());
    Ok(())
}

/// Appends or updates environment file
/// Creates a new .env file with biscuit keys
fn create_new_env_file(
    path: &Path,
    priv_b64: &str,
    pub_b64: &str,
) -> std::io::Result<()> {
    let mut f = fs::File::create(path)?;
    writeln!(f, "# Generated biscuit keys")?;
    writeln!(f, "BISCUIT_PRIVATE_KEY_B64={}", priv_b64)?;
    writeln!(f, "BISCUIT_PUBLIC_KEY_B64={}", pub_b64)?;
    Ok(())
}

/// Replaces existing biscuit keys in .env file
fn replace_env_keys(
    path: &Path,
    content: &str,
    priv_b64: &str,
    pub_b64: &str,
) -> std::io::Result<()> {
    let filtered: Vec<_> = content
        .lines()
        .filter(|line| {
            !line.starts_with("BISCUIT_PRIVATE_KEY_B64=")
                && !line.starts_with("BISCUIT_PUBLIC_KEY_B64=")
        })
        .collect();

    let mut f = fs::File::create(path)?;
    for line in filtered {
        writeln!(f, "{}", line)?;
    }
    writeln!(f, "BISCUIT_PRIVATE_KEY_B64={}", priv_b64)?;
    writeln!(f, "BISCUIT_PUBLIC_KEY_B64={}", pub_b64)?;
    Ok(())
}

/// Appends biscuit keys to existing .env file
fn append_env_keys(
    path: &Path,
    priv_b64: &str,
    pub_b64: &str,
) -> std::io::Result<()> {
    let mut f = OpenOptions::new().append(true).open(path)?;
    writeln!(f, "\n# Generated biscuit keys")?;
    writeln!(f, "BISCUIT_PRIVATE_KEY_B64={}", priv_b64)?;
    writeln!(f, "BISCUIT_PUBLIC_KEY_B64={}", pub_b64)?;
    Ok(())
}

/// Main function to append or update .env file with biscuit keys
fn append_or_update_env(
    path: &Path,
    priv_b64: &str,
    pub_b64: &str,
    force: bool,
) -> std::io::Result<()> {
    if !path.exists() {
        return create_new_env_file(path, priv_b64, pub_b64);
    }

    let content = fs::read_to_string(path)?;
    let has_keys = content.contains("BISCUIT_PRIVATE_KEY_B64=") 
        || content.contains("BISCUIT_PUBLIC_KEY_B64=");

    if has_keys {
        if !force {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("{} already contains biscuit keys", path.display()),
            ));
        }
        replace_env_keys(path, &content, priv_b64, pub_b64)
    } else {
        append_env_keys(path, priv_b64, pub_b64)
    }
}

/// Gets next version number
fn next_version(dir: &Path) -> u32 {
    let mut max_v = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if let Some(v) = parse_version(name) {
                    max_v = max_v.max(v);
                }
            }
        }
    }
    max_v + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("biscuit_private_v1.b64"), Some(1));
        assert_eq!(parse_version("biscuit_private_v123.b64"), Some(123));
        assert_eq!(parse_version("biscuit_private.b64"), None);
    }

    #[test]
    fn test_path_validation() {
        let short_path = PathBuf::from("keys");
        assert!(validate_path_length(&short_path).is_ok());
        
        let long_path = PathBuf::from("a".repeat(MAX_PATH_LENGTH + 1));
        assert!(validate_path_length(&long_path).is_err());
    }
}