use cms_backend::utils::hash;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize)]
struct Manifest<'a> {
    latest_version: u32,
    generated_at: String,
    private_fingerprint: &'a str,
    public_fingerprint: &'a str,
}

pub fn update_manifest(dir: &Path, version: u32, priv_fp: &str, pub_fp: &str) {
    let manifest_path = dir.join("manifest.json");
    let manifest = Manifest {
        latest_version: version,
        generated_at: chrono::Utc::now().to_rfc3339(),
        private_fingerprint: priv_fp,
        public_fingerprint: pub_fp,
    };
    if let Ok(json) = serde_json::to_string_pretty(&manifest) {
        if let Err(e) = fs::write(&manifest_path, json) {
            eprintln!("Failed to write manifest {}: {e}", manifest_path.display(),);
        } else {
            println!("Updated manifest at {}", manifest_path.display());
        }
    }
}

pub fn prune_versions(dir: &Path, keep: usize) {
    if keep == 0 {
        return;
    }
    let mut versions: Vec<u32> = Vec::new();
    if let Ok(read) = fs::read_dir(dir) {
        for entry in read.flatten() {
            if let Some(name) = entry.file_name().to_str()
                && name.starts_with("biscuit_private_v")
                && std::path::Path::new(name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("b64"))
                && let Some(v) = super::parse_version(name)
            {
                versions.push(v);
            }
        }
    }
    if versions.len() <= keep {
        return;
    }
    versions.sort_unstable();
    let to_remove: Vec<u32> = versions
        .iter()
        .copied()
        .take(versions.len() - keep)
        .collect();
    for v in to_remove {
        for prefix in ["biscuit_private_v", "biscuit_public_v"] {
            let path = dir.join(format!("{}{}.b64", prefix, v));
            if path.exists() {
                if let Err(e) = fs::remove_file(&path) {
                    eprintln!("Failed to remove old version {}: {e}", path.display());
                } else {
                    println!("Pruned old version file {}", path.display());
                }
            }
        }
    }
}

pub fn finalize_versioned(
    path: &Path,
    priv_path: &Path,
    priv_b64: &str,
    pub_b64: &str,
    no_manifest: bool,
    prune: Option<usize>,
) {
    let v = priv_path
        .file_name()
        .and_then(|s| s.to_str())
        .and_then(super::parse_version)
        .unwrap_or_else(|| {
            eprintln!(
                "Could not determine version from path: {}",
                priv_path.display()
            );
            0
        });
    let priv_fp = hash::sha256_hex(priv_b64.as_bytes());
    let pub_fp = hash::sha256_hex(pub_b64.as_bytes());
    println!("private_fingerprint_sha256={priv_fp} public_fingerprint_sha256={pub_fp}");
    if !no_manifest {
        update_manifest(path, v, &priv_fp, &pub_fp);
    }
    if let Some(keep) = prune {
        prune_versions(path, keep);
    }
}

// New helper: perform the full files output flow to keep binary file small.
pub fn handle_files_output_full(ctx: &super::FilesOutputContext) -> cms_backend::Result<()> {
    // Ensure directory exists
    if let Err(e) = std::fs::create_dir_all(ctx.path) {
        return Err(cms_backend::AppError::Internal(format!(
            "Failed to create out-dir {}: {}",
            ctx.path.display(),
            e
        )));
    }
    // Resolve paths
    let (priv_path, pub_path) = super::resolve_output_paths(ctx.path, ctx.vopts.versioned);
    // Perform writes and backups
    super::write_files_flow(
        &priv_path,
        &pub_path,
        ctx.backup,
        ctx.options,
        ctx.priv_b64,
        ctx.pub_b64,
    );
    // Finalize (manifest, prune, alias)
    // call into bin's finalize logic (apply alias/manifest/prune)
    super::gen_biscuit_keys_manifest::finalize_versioned(
        ctx.path,
        &priv_path,
        ctx.priv_b64,
        ctx.pub_b64,
        ctx.vopts.no_manifest,
        ctx.vopts.prune,
    );
    Ok(())
}
