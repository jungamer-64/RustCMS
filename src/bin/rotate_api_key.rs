use clap::{Parser, Subcommand};
use cms_backend::AppState;
use cms_backend::config::Config;

#[derive(Parser, Debug)]
#[command(
    name = "rotate_api_key",
    about = "APIキーのローテーション / 削除 / 作成を支援する管理CLI",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 既存キーをローテーション (新規発行し旧キーを即時 expire)
    Rotate {
        /// 対象キーID (UUID)
        #[arg(long)]
        id: String,
        /// 新しい名前 (省略時は既存名)
        #[arg(long)]
        name: Option<String>,
        /// 新しいパーミッション (複数指定可)。省略時は既存権限を引き継ぐ
        #[arg(long = "perm")]
        permissions: Vec<String>,
    },
    /// APIキーを削除 (DB から完全除去)
    Delete {
        #[arg(long)]
        id: String,
    },
}

#[tokio::main]
async fn main() -> cms_backend::Result<()> {
    init_env();
    let cli = Cli::parse();
    let config = Config::from_env()?;
    let state = AppState::from_config(config).await?;

    match cli.command {
        Commands::Rotate {
            id,
            name,
            permissions,
        } => {
            let uuid = uuid::Uuid::parse_str(&id)
                .map_err(|e| cms_backend::AppError::BadRequest(format!("invalid uuid: {e}")))?;
            let perms_opt = if permissions.is_empty() {
                None
            } else {
                Some(permissions)
            };
            let (resp, raw) = state.db_rotate_api_key(uuid, name, perms_opt).await?;
            println!(
                "{{\n  \"old_key_id\": \"{id}\",\n  \"new_key_id\": \"{}\",\n  \"new_raw_api_key\": \"{raw}\",\n  \"new_name\": \"{}\",\n  \"permissions\": {:?},\n  \"expires_at\": {:?}\n}}",
                resp.id, resp.name, resp.permissions, resp.expires_at
            );
        }
        Commands::Delete { id } => {
            let uuid = uuid::Uuid::parse_str(&id)
                .map_err(|e| cms_backend::AppError::BadRequest(format!("invalid uuid: {e}")))?;
            state.db_delete_api_key(uuid).await?;
            println!("{{\n  \"deleted\": true,\n  \"id\": \"{id}\"\n}}");
        }
    }
    Ok(())
}

fn init_env() {
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("Warning: Could not load .env file: {}", e);
    }
}
