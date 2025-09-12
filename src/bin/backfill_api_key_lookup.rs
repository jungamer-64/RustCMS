//! Backfill script for api_key_lookup_hash
//!
//! Usage (with features):
//!   cargo run --features "database auth" --bin backfill_api_key_lookup
//!
//! It scans api_keys rows where api_key_lookup_hash is empty string (""),
//! re-computes deterministic lookup hash from the *raw key cannot be recovered*.
//! Because raw keys are not stored, true recomputation is impossible. This tool
//! instead flags such rows so operators can rotate them. Optionally it can mark
//! them expired.
//!
//! Strategy:
//! 1. Find rows with empty lookup hash.
//! 2. If --expire is passed, set expires_at = now() for those rows.
//! 3. Output a JSON report listing affected key IDs and suggested action.
//!
//! Rationale: we cannot derive lookup hash post-hoc without the raw key. The
//! middleware already has a lazy fallback that updates on successful usage.
//! This batch script is for visibility & optional forced expiry.

use chrono::Utc;
use clap::Parser;
use cms_backend::utils::init::init_env;

#[derive(Parser, Debug)]
#[command(author, version, about = "Backfill/inspect api_key lookup hash state", long_about=None)]
struct Args {
    /// Expire (invalidate) legacy keys missing lookup hash
    #[arg(long)]
    expire: bool,
    /// Dry-run only (no DB writes) even if --expire is passed
    #[arg(long)]
    dry_run: bool,
    /// Output pretty JSON
    #[arg(long)]
    pretty: bool,
}

#[derive(serde::Serialize)]
struct RowReport {
    id: uuid::Uuid,
    name: String,
    user_id: uuid::Uuid,
    created_at: chrono::DateTime<Utc>,
}

#[derive(serde::Serialize)]
struct Report {
    scanned: usize,
    legacy_missing_lookup: usize,
    expired_marked: usize,
    rows: Vec<RowReport>,
    expire_mode: bool,
    dry_run: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_env();
    let args = Args::parse();

    #[cfg(not(all(feature = "database", feature = "auth")))]
    {
        eprintln!("Requires --features database,auth");
        return Ok(());
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    {
        let state = cms_backend::utils::init::init_app_state().await?;

        // 取得: 空 lookup_hash の行（AppState ラッパー）
        let rows: Vec<cms_backend::models::ApiKey> =
            state.db_list_api_keys_missing_lookup().await?;
        let scanned_count = rows.len();
        let expired_marked = if args.expire && !args.dry_run {
            let now = Utc::now();
            state
                .db_expire_api_keys_missing_lookup(now)
                .await?
        } else {
            0usize
        };

        let report = Report {
            scanned: scanned_count,
            legacy_missing_lookup: scanned_count,
            expired_marked,
            rows: rows
                .into_iter()
                .map(|r| RowReport {
                    id: r.id,
                    name: r.name,
                    user_id: r.user_id,
                    created_at: r.created_at,
                })
                .collect(),
            expire_mode: args.expire,
            dry_run: args.dry_run,
        };
        if args.pretty {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            println!("{}", serde_json::to_string(&report)?);
        }
    }
    Ok(())
}
