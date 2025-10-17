use cms_backend::utils::hash;
use cms_backend::{AppError, Result};
use serde::Serialize;
use std::fs;
use std::path::Path; // Assuming AppError and Result are defined in cms_backend

#[derive(Serialize)]
struct Manifest<'a> {
    latest_version: u32,
    generated_at: String,
    private_fingerprint: &'a str,
    public_fingerprint: &'a str,
}

/// manifest.json を更新します。
/// 失敗した場合は I/O エラーを返します。
pub fn update_manifest(
    dir: &Path,
    version: u32,
    priv_fp: &str,
    pub_fp: &str,
) -> std::io::Result<()> {
    let manifest_path = dir.join("manifest.json");
    let manifest = Manifest {
        latest_version: version,
        generated_at: chrono::Utc::now().to_rfc3339(),
        private_fingerprint: priv_fp,
        public_fingerprint: pub_fp,
    };

    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    fs::write(&manifest_path, json)?;
    println!("Updated manifest at {}", manifest_path.display());
    Ok(())
}

/// Checks if a file is a versioned biscuit key file
fn is_versioned_key_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    path.file_name()
        .and_then(|s| s.to_str())
        .is_some_and(|name| {
            name.starts_with("biscuit_private_v")
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("b64"))
        })
}

/// Collects all version numbers from versioned key files in a directory
fn collect_versions(dir: &Path) -> std::io::Result<Vec<u32>> {
    let mut versions = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if is_versioned_key_file(&path) {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if let Some(v) = super::parse_version(name) {
                    versions.push(v);
                }
            }
        }
    }

    Ok(versions)
}

/// Removes key files for specified versions
fn remove_version_files(dir: &Path, versions: &[u32]) -> std::io::Result<()> {
    for &v in versions {
        for prefix in ["biscuit_private_v", "biscuit_public_v"] {
            let path = dir.join(format!("{prefix}{v}.b64"));
            if path.exists() {
                fs::remove_file(&path)?;
                println!("Pruned old version file {}", path.display());
            }
        }
    }
    Ok(())
}

/// 古いバージョンのキーファイルを指定された数だけ残して削除(プルーニング)します。
/// 失敗した場合は I/O エラーを返します。
pub fn prune_versions(dir: &Path, keep: usize) -> std::io::Result<()> {
    if keep == 0 {
        return Ok(());
    }

    // ディレクトリ内のバージョンファイルを収集する
    let mut versions = collect_versions(dir)?;

    if versions.len() <= keep {
        return Ok(());
    }

    versions.sort_unstable();

    // 保持する数を超えた古いバージョンを削除対象とする
    let to_remove_count = versions.len() - keep;
    let versions_to_remove = &versions[..to_remove_count];

    remove_version_files(dir, versions_to_remove)?;

    Ok(())
}

/// Parses version number from file path
fn extract_version_from_path(priv_path: &Path) -> Result<u32> {
    priv_path
        .file_name()
        .and_then(|s| s.to_str())
        .and_then(super::parse_version)
        .ok_or_else(|| {
            AppError::Internal(format!(
                "Could not determine version from path: {}",
                priv_path.display()
            ))
        })
}

/// Calculates and displays fingerprints for both keys
fn display_key_fingerprints(priv_b64: &str, pub_b64: &str) -> (String, String) {
    let priv_fp = hash::sha256_hex(priv_b64.as_bytes());
    let pub_fp = hash::sha256_hex(pub_b64.as_bytes());
    println!("private_fingerprint_sha256={priv_fp} public_fingerprint_sha256={pub_fp}");
    (priv_fp, pub_fp)
}

/// Performs post-versioning operations (alias, manifest, pruning)
/// Parameters for perform_versioned_operations to avoid too many function arguments.
struct PerformVersionedParams<'a> {
    path: &'a Path,
    v: u32,
    priv_b64: &'a str,
    pub_b64: &'a str,
    priv_fp: &'a str,
    pub_fp: &'a str,
    latest_alias: bool,
    no_manifest: bool,
    prune: Option<usize>,
}

fn perform_versioned_operations(params: PerformVersionedParams) -> std::io::Result<()> {
    if params.latest_alias {
        write_latest_alias(params.path, params.priv_b64, params.pub_b64)?;
    }

    if !params.no_manifest {
        update_manifest(
            params.path,
            params.v,
            params.priv_fp,
            params.pub_fp,
        )?;
    }

    if let Some(keep) = params.prune {
        prune_versions(params.path, keep)?;
    }

    Ok(())
}

/// バージョン管理されたファイルの最終処理(フィンガープリント計算、マニフェスト更新、プルーニング)を行います。
pub fn finalize_versioned(
    path: &Path,
    priv_path: &Path,
    priv_b64: &str,
    pub_b64: &str,
    latest_alias: bool,
    no_manifest: bool,
    prune: Option<usize>,
) -> Result<()> {
    // ファイルパスからバージョン番号をパースする。失敗した場合はエラーを返す。
    let v = extract_version_from_path(priv_path)?;

    let (priv_fp, pub_fp) = display_key_fingerprints(priv_b64, pub_b64);

    // エラーを map_err でアプリケーション固有のエラー型に変換
    let to_app_err = |e: std::io::Error| AppError::Internal(e.to_string());

    let params = PerformVersionedParams {
        path,
        v,
        priv_b64,
        pub_b64,
        priv_fp: &priv_fp,
        pub_fp: &pub_fp,
        latest_alias,
        no_manifest,
        prune,
    };

    perform_versioned_operations(params).map_err(to_app_err)?;

    Ok(())
}

fn write_latest_alias(dir: &Path, priv_b64: &str, pub_b64: &str) -> std::io::Result<()> {
    let priv_alias = dir.join("biscuit_private.b64");
    let pub_alias = dir.join("biscuit_public.b64");

    fs::write(&priv_alias, priv_b64)?;
    fs::write(&pub_alias, pub_b64)?;

    println!(
        "Updated latest alias files: {} & {}",
        priv_alias.display(),
        pub_alias.display()
    );

    Ok(())
}

// Note: This function is deprecated and no longer used.
// The functionality has been moved to the main gen_biscuit_keys module.
// Keeping this for reference during transition period.
/*
/// ファイル出力に関する一連のフローを処理します。
/// ディレクトリ作成、パス解決、ファイル書き込み、最終処理までを一貫して行います。
pub fn handle_files_output_full(ctx: &super::FilesOutputContext) -> Result<()> {
    // 出力ディレクトリが存在しない場合は作成する
    fs::create_dir_all(ctx.path).map_err(|e| {
        AppError::Internal(format!(
            "Failed to create out-dir {}: {}",
            ctx.path.display(),
            e
        ))
    })?;

    // 出力パスを解決
    let (priv_path, pub_path) = super::resolve_output_paths(ctx.path, ctx.vopts.versioned);

    // ファイル書き込みとバックアップ処理
    super::write_files_flow(
        &priv_path,
        &pub_path,
        ctx.backup,
        ctx.options,
        ctx.priv_b64,
        ctx.pub_b64,
    );

    // マニフェスト更新、プルーニングなどの最終処理
    if ctx.vopts.versioned {
        finalize_versioned(
            ctx.path,
            &priv_path,
            ctx.priv_b64,
            ctx.pub_b64,
            ctx.vopts.latest_alias,
            ctx.vopts.no_manifest,
            ctx.vopts.prune,
        )?;
    }

    Ok(())
}
*/
