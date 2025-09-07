//! Biscuit token rotation & invalidation test (requires database + auth features).
#![cfg(all(feature = "auth", feature = "database"))]

use cms_backend::auth::AuthService;
use cms_backend::config::{AuthConfig, DatabaseConfig};
use cms_backend::database::Database;
use cms_backend::models::{User, UserRole};
use chrono::Utc;
use uuid::Uuid;

async fn build_db() -> Option<Database> {
    let url = std::env::var("DATABASE_URL").ok()?;
    if url.is_empty() { return None; }
    let cfg = DatabaseConfig {
        url,
        max_connections: 2,
        min_connections: 1,
        connection_timeout: 5,
        idle_timeout: 30,
        max_lifetime: 300,
        enable_migrations: false, // テスト: 既存テーブル想定 / 速度優先
    };
    Database::new(&cfg).await.ok()
}

async fn build_auth(db: &Database) -> AuthService {
    use biscuit_auth::KeyPair;
    use base64::Engine;
    let kp = KeyPair::new();
    let priv_b64 = base64::engine::general_purpose::STANDARD.encode(kp.private().to_bytes());
    let mut cfg = AuthConfig::default();
    cfg.biscuit_root_key = priv_b64;
    cfg.access_token_ttl_secs = 30; // short TTL for test
    cfg.refresh_token_ttl_secs = 300;
    AuthService::new(&cfg, db).await.expect("auth init")
}

fn dummy_user() -> User {
    let now = Utc::now();
    User {
        id: Uuid::new_v4(),
        username: format!("user_{}", Uuid::new_v4().simple()),
        email: format!("test+{}@example.com", Uuid::new_v4()),
        password_hash: Some("$argon2id$v=19$m=65536,t=3,p=4$YWJj$YWJj".into()),
        first_name: None,
        last_name: None,
        role: UserRole::Subscriber.as_str().to_string(),
        is_active: true,
        email_verified: false,
        last_login: None,
        created_at: now,
        updated_at: now,
    }
}

#[tokio::test]
async fn biscuit_refresh_rotation_invalidate_old() {
    let db = match build_db().await {
        Some(d) => d,
        None => { eprintln!("SKIP biscuit_refresh_rotation_invalidate_old: DATABASE_URL not set"); return; }
    };
    let auth = build_auth(&db).await;
    let user = dummy_user();

    let issued = auth.create_auth_response(user.clone(), false).await.expect("issue");
    assert!(!issued.access_token.is_empty());
    assert!(!issued.refresh_token.is_empty());

    let ctx = auth.verify_jwt(&issued.access_token).await.expect("verify access");
    assert_eq!(ctx.user_id, user.id);

    let rotated = auth.refresh_access_token(&issued.refresh_token).await.expect("refresh");
    assert_ne!(rotated.access_token, issued.access_token);
    assert_ne!(rotated.refresh_token, issued.refresh_token);

    // Old refresh must now fail
    assert!(auth.refresh_access_token(&issued.refresh_token).await.is_err());
}
