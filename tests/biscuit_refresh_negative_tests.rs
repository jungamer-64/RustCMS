//! Negative tests for refresh token: expiry & version mismatch

use chrono::Utc;
use cms_backend::auth::AuthService;
use cms_backend::config::{AuthConfig, DatabaseConfig};
use cms_backend::database::Database;
use cms_backend::models::{User, UserRole};
use uuid::Uuid;

async fn build_db() -> Option<Database> {
    let url = std::env::var("DATABASE_URL").ok()?;
    if url.is_empty() {
        return None;
    }
    let cfg = DatabaseConfig {
        url: url.into(),
        max_connections: 2,
        min_connections: 1,
        connection_timeout: 5,
        idle_timeout: 30,
        max_lifetime: 300,
        enable_migrations: false,
    };
    Database::new(&cfg).await.ok()
}

fn dummy_user() -> User {
    let now = Utc::now();
    User {
        id: Uuid::new_v4(),
        username: format!("user_neg_{}", Uuid::new_v4().simple()),
        email: format!("neg+{}@example.com", Uuid::new_v4()),
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

async fn build_auth(db: &Database, access_ttl: u64, refresh_ttl: u64) -> AuthService {
    use base64::Engine;
    use biscuit_auth::KeyPair;
    let kp = KeyPair::new();
    let priv_b64 = base64::engine::general_purpose::STANDARD.encode(kp.private().to_bytes());
    let cfg = AuthConfig {
        biscuit_root_key: priv_b64.into(),
        access_token_ttl_secs: access_ttl,
        refresh_token_ttl_secs: refresh_ttl,
        ..Default::default()
    };
    AuthService::new(&cfg, db).expect("auth init")
}

#[tokio::test]
async fn refresh_fails_after_expiry() {
    let Some(db) = build_db().await else {
        eprintln!("SKIP refresh_fails_after_expiry: DATABASE_URL not set");
        return;
    };
    // Very short refresh TTL
    let auth = build_auth(&db, 1, 2).await; // access 1s, refresh 2s
    let user = dummy_user();
    let issued = auth.create_auth_response(user, false).await.expect("issue");
    // Wait until refresh expiry passes
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    assert!(
        auth.refresh_access_token(&issued.tokens.refresh_token)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn refresh_version_mismatch_reuse_old() {
    let Some(db) = build_db().await else {
        eprintln!("SKIP refresh_version_mismatch_reuse_old: DATABASE_URL not set");
        return;
    };
    let auth = build_auth(&db, 30, 60).await;
    let user = dummy_user();
    let issued = auth
        .create_auth_response(user.clone(), false)
        .await
        .expect("issue");
    // First rotation
    let (rot_tokens, _) = auth
        .refresh_access_token(&issued.tokens.refresh_token)
        .await
        .expect("rotate");
    // Reuse original refresh => should fail (version bump)
    assert!(
        auth.refresh_access_token(&issued.tokens.refresh_token)
            .await
            .is_err()
    );
    // Rotated token still works once
    let _ = auth
        .refresh_access_token(&rot_tokens.refresh_token)
        .await
        .expect("second ok");
}
